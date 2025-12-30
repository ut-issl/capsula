mod error;
use crate::error::SlackNotifyError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PostRun, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Configuration for the Slack notification hook
///
/// # Fields
/// * `channel` - The Slack channel to send notifications to (e.g., "#random")
/// * `token` - The Slack bot token. If not provided, will read from `SLACK_BOT_TOKEN` environment variable
/// * `attachment_globs` - Optional list of glob patterns to match files for attachment (up to 10 files)
///
/// # Example
/// ```toml
/// [[post_run.hooks]]
/// id = "notify-slack"
/// channel = "#random"
/// attachment_globs = ["*.png", "outputs/*.jpg"]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackNotifyHookConfig {
    channel: String,
    #[serde(default = "token_from_env")]
    token: String,
    #[serde(default)]
    attachment_globs: Vec<String>,
}

fn token_from_env() -> String {
    std::env::var("SLACK_BOT_TOKEN").unwrap_or_default()
}

/// Resolve glob patterns and collect matching file paths
/// Globs are resolved relative to the provided base directory
fn resolve_attachment_globs(
    globs: &[String],
    base_dir: &Path,
) -> Result<Vec<PathBuf>, SlackNotifyError> {
    let mut files = Vec::new();

    for pattern in globs {
        // Resolve pattern relative to base_dir
        let full_pattern = base_dir.join(pattern);
        let pattern_str = full_pattern
            .to_str()
            .ok_or_else(|| SlackNotifyError::GlobPattern {
                pattern: pattern.clone(),
                source: glob::PatternError {
                    pos: 0,
                    msg: "Invalid UTF-8 in path",
                },
            })?;

        // Execute glob
        let paths = glob::glob(pattern_str).map_err(|e| SlackNotifyError::GlobPattern {
            pattern: pattern.clone(),
            source: e,
        })?;

        // Collect successful matches, skip errors
        for entry in paths {
            match entry {
                Ok(path) if path.is_file() => files.push(path),
                Ok(_) | Err(_) => {} // Skip directories and glob errors (e.g., permission denied)
            }
        }
    }

    // Limit to 10 files (Slack API limit)
    files.truncate(10);

    Ok(files)
}

/// Upload a single file to Slack and return its `file_id`
fn upload_file_to_slack(
    client: &reqwest::blocking::Client,
    token: &str,
    path: &Path,
) -> Result<(String, String), SlackNotifyError> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let file_content = std::fs::read(path).map_err(|e| SlackNotifyError::FileIo {
        path: path.display().to_string(),
        source: e,
    })?;

    let file_size = file_content.len();

    // Get upload URL (must use form parameters, not JSON)
    let form = reqwest::blocking::multipart::Form::new()
        .text("filename", file_name.clone())
        .text("length", file_size.to_string());

    let upload_url_res = client
        .post("https://slack.com/api/files.getUploadURLExternal")
        .bearer_auth(token)
        .multipart(form)
        .send()
        .map_err(SlackNotifyError::from)?;

    let upload_url_json: serde_json::Value = upload_url_res
        .json()
        .map_err(SlackNotifyError::from)?;

    if !upload_url_json.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false) {
        let error_msg = upload_url_json.get("error").and_then(|v| v.as_str()).unwrap_or("unknown error");

        // Provide helpful message for missing_scope error
        let message = if error_msg == "missing_scope" {
            "Failed to upload file: missing_scope. The Slack bot needs the 'files:write' OAuth scope. \
             Please add this scope in your Slack app settings under OAuth & Permissions."
                .to_string()
        } else {
            format!("Failed to get upload URL: {error_msg}")
        };

        return Err(SlackNotifyError::SlackApi { message });
    }

    let upload_url = upload_url_json
        .get("upload_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SlackNotifyError::SlackApi {
            message: "Missing upload_url in response".to_string(),
        })?;

    let file_id = upload_url_json
        .get("file_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SlackNotifyError::SlackApi {
            message: "Missing file_id in response".to_string(),
        })?;

    // Upload file to the URL
    client
        .post(upload_url)
        .body(file_content)
        .send()
        .map_err(SlackNotifyError::from)?;

    Ok((file_id.to_string(), file_name))
}

/// Send a simple message to Slack without attachments
fn send_simple_message(
    client: &reqwest::blocking::Client,
    token: &str,
    channel: &str,
    text: &str,
) -> Result<String, SlackNotifyError> {
    let payload = json!({
        "channel": channel,
        "text": text,
    });

    let res = client
        .post("https://slack.com/api/chat.postMessage")
        .bearer_auth(token)
        .json(&payload)
        .send()
        .map_err(SlackNotifyError::from)?;

    if !res.status().is_success() {
        return Err(SlackNotifyError::SlackApi {
            message: format!("Failed to send Slack notification: {}", res.status()),
        });
    }

    Ok(res.text().unwrap_or_else(|_| String::from("{}")))
}

/// Send a message with optional file attachments to Slack
fn send_slack_message(
    token: &str,
    channel: &str,
    text: &str,
    attachment_paths: &[PathBuf],
) -> Result<(String, Vec<String>), SlackNotifyError> {
    let client = reqwest::blocking::Client::new();

    // If no attachments, use simple chat.postMessage
    if attachment_paths.is_empty() {
        let response = send_simple_message(&client, token, channel, text)?;
        return Ok((response, Vec::new()));
    }

    // With attachments, use the new Slack files API workflow:
    // 1. Get upload URLs
    // 2. Upload files to those URLs
    // 3. Complete the upload and share to channel

    let mut file_ids = Vec::new();
    let mut attached_files = Vec::new();

    // Step 1 & 2: Get upload URL and upload each file
    for path in attachment_paths {
        let (file_id, file_name) = upload_file_to_slack(&client, token, path)?;
        file_ids.push(file_id);
        attached_files.push(file_name);
    }

    // Step 3: Complete upload and share to channel
    // Build files array as JSON string
    let files_json = serde_json::to_string(
        &file_ids
            .iter()
            .map(|id| json!({"id": id}))
            .collect::<Vec<_>>(),
    )
    .map_err(SlackNotifyError::Serialization)?;

    // Use form data (not JSON) for the request
    let mut complete_form = reqwest::blocking::multipart::Form::new()
        .text("files", files_json)
        .text("initial_comment", text.to_string());

    // Add channel_id if it's a channel ID (starts with C), otherwise use channels parameter
    if channel.starts_with('C') {
        complete_form = complete_form.text("channel_id", channel.to_string());
    } else {
        complete_form = complete_form.text("channels", channel.to_string());
    }

    let complete_res = client
        .post("https://slack.com/api/files.completeUploadExternal")
        .bearer_auth(token)
        .multipart(complete_form)
        .send()
        .map_err(SlackNotifyError::from)?;

    if !complete_res.status().is_success() {
        return Err(SlackNotifyError::SlackApi {
            message: format!("Failed to complete file upload: {}", complete_res.status()),
        });
    }

    // Check if the Slack API returned an error in the response
    let complete_json: serde_json::Value = complete_res
        .json()
        .map_err(SlackNotifyError::from)?;

    if !complete_json.get("ok").and_then(serde_json::Value::as_bool).unwrap_or(false) {
        let error_msg = complete_json.get("error").and_then(|v| v.as_str()).unwrap_or("unknown error");
        return Err(SlackNotifyError::SlackApi {
            message: format!("Failed to complete file upload: {error_msg}"),
        });
    }

    Ok((
        serde_json::to_string(&complete_json).unwrap_or_else(|_| String::from("{}")),
        attached_files,
    ))
}

#[derive(Debug)]
pub struct SlackNotifyHook {
    config: SlackNotifyHookConfig,
    project_root: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct SlackNotifyCaptured {
    message: String,
    response: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attached_files: Vec<String>,
}

impl Captured for SlackNotifyCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

impl Hook<PreRun> for SlackNotifyHook {
    const ID: &'static str = "notify-slack";

    type Config = SlackNotifyHookConfig;
    type Output = SlackNotifyCaptured;

    fn from_config(
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config = serde_json::from_value::<SlackNotifyHookConfig>(config.clone())?;
        if config.token.is_empty() {
            return Err(SlackNotifyError::MissingToken.into());
        }
        Ok(Self {
            config,
            project_root: project_root.to_path_buf(),
        })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        _params: &RuntimeParams<PreRun>,
    ) -> CapsulaResult<Self::Output> {
        let text = format!(
            "Run `{}` (ID: `{}`) is starting.",
            metadata.name, metadata.id
        );

        // Resolve attachment globs relative to project root
        let attachment_paths =
            resolve_attachment_globs(&self.config.attachment_globs, &self.project_root)?;

        // Send message with attachments
        let (response, attached_files) = send_slack_message(
            &self.config.token,
            &self.config.channel,
            &text,
            &attachment_paths,
        )?;

        Ok(SlackNotifyCaptured {
            message: "Slack notification sent successfully".to_string(),
            response: Some(response),
            attached_files,
        })
    }
}

impl Hook<PostRun> for SlackNotifyHook {
    const ID: &'static str = "notify-slack";

    type Config = SlackNotifyHookConfig;
    type Output = SlackNotifyCaptured;

    fn from_config(
        config: &serde_json::Value,
        project_root: &std::path::Path,
    ) -> CapsulaResult<Self> {
        let config = serde_json::from_value::<SlackNotifyHookConfig>(config.clone())?;
        if config.token.is_empty() {
            return Err(SlackNotifyError::MissingToken.into());
        }
        Ok(Self {
            config,
            project_root: project_root.to_path_buf(),
        })
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        _params: &RuntimeParams<PostRun>,
    ) -> CapsulaResult<Self::Output> {
        let text = format!(
            "Run `{}` (ID: `{}`) has completed.",
            metadata.name, metadata.id
        );

        // Resolve attachment globs relative to project root
        let attachment_paths =
            resolve_attachment_globs(&self.config.attachment_globs, &self.project_root)?;

        // Send message with attachments
        let (response, attached_files) = send_slack_message(
            &self.config.token,
            &self.config.channel,
            &text,
            &attachment_paths,
        )?;

        Ok(SlackNotifyCaptured {
            message: "Slack notification sent successfully".to_string(),
            response: Some(response),
            attached_files,
        })
    }
}
