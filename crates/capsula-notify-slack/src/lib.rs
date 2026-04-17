mod error;
mod secret;
use crate::error::SlackNotifyError;
use crate::secret::SecretString;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, PostRun, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Maximum number of files that can be attached to a Slack message (API limit)
const MAX_SLACK_ATTACHMENTS: usize = 10;

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
    token: SecretString,
    #[serde(default)]
    attachment_globs: Vec<String>,
}

fn token_from_env() -> SecretString {
    SecretString::new(std::env::var("SLACK_BOT_TOKEN").unwrap_or_default())
}

/// Build Slack Block Kit blocks for a run notification
///
/// This is a pure function that constructs the rich-formatted Slack message blocks.
/// It creates a structured message with header, divider, and run details section.
///
/// # Arguments
/// * `header_text` - The header text to display (e.g., "🚀 Capsula Run Starting")
/// * `run_name` - The name of the run
/// * `run_id` - The unique ID of the run
/// * `timestamp_unix` - Unix timestamp for Slack's date formatting
/// * `timestamp_rfc3339` - RFC3339 formatted timestamp as fallback
/// * `command_display` - The formatted command string to display
fn build_slack_blocks(
    header_text: &str,
    run_name: &str,
    run_id: impl std::fmt::Display,
    timestamp_unix: i64,
    timestamp_rfc3339: &str,
    command_display: &str,
) -> Vec<serde_json::Value> {
    vec![
        // Header
        json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": header_text,
                "emoji": true
            }
        }),
        // Divider
        json!({
            "type": "divider"
        }),
        // Run details
        json!({
            "type": "section",
            "fields": [
                {
                    "type": "mrkdwn",
                    "text": format!("*Run Name:*\n{}", run_name)
                },
                {
                    "type": "mrkdwn",
                    "text": format!("*Run ID:*\n`{}`", run_id)
                },
                {
                    "type": "mrkdwn",
                    "text": format!("*Timestamp:*\n<!date^{}^{{date_num}} {{time_secs}}|{}>",
                        timestamp_unix, timestamp_rfc3339)
                },
                {
                    "type": "mrkdwn",
                    "text": format!("*Command:*\n```{}```", command_display)
                }
            ]
        }),
    ]
}

/// Resolve glob patterns and collect matching file paths
/// Globs are resolved relative to the provided base directory
#[doc(hidden)]
pub fn resolve_attachment_globs(
    globs: &[String],
    base_dir: &Path,
) -> Result<Vec<PathBuf>, SlackNotifyError> {
    debug!(
        "Resolving {} attachment glob patterns from base: {}",
        globs.len(),
        base_dir.display()
    );

    let mut files = Vec::new();

    for pattern in globs {
        debug!("Processing glob pattern: {}", pattern);
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
        let mut pattern_matches = 0;
        for entry in paths {
            match entry {
                Ok(path) if path.is_file() => {
                    files.push(path);
                    pattern_matches += 1;
                }
                Ok(_) | Err(_) => {} // Skip directories and glob errors (e.g., permission denied)
            }
        }
        debug!("Pattern '{}' matched {} files", pattern, pattern_matches);
    }

    // Limit to MAX_SLACK_ATTACHMENTS files (Slack API limit)
    let original_count = files.len();
    files.truncate(MAX_SLACK_ATTACHMENTS);

    if original_count > MAX_SLACK_ATTACHMENTS {
        debug!(
            "Truncated attachment list from {} to {} files (Slack API limit)",
            original_count, MAX_SLACK_ATTACHMENTS
        );
    }

    debug!("Total files resolved: {}", files.len());
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

    debug!("Reading file content: {}", path.display());
    let file_content = std::fs::read(path).map_err(|e| SlackNotifyError::FileIo {
        path: path.display().to_string(),
        source: e,
    })?;

    let file_size = file_content.len();
    debug!("File size: {} bytes", file_size);

    // Get upload URL (must use form parameters, not JSON)
    debug!("Requesting upload URL from Slack for file: {}", file_name);
    let form = reqwest::blocking::multipart::Form::new()
        .text("filename", file_name.clone())
        .text("length", file_size.to_string());

    let upload_url_res = client
        .post("https://slack.com/api/files.getUploadURLExternal")
        .bearer_auth(token)
        .multipart(form)
        .send()
        .map_err(SlackNotifyError::from)?;

    let upload_url_json: serde_json::Value =
        upload_url_res.json().map_err(SlackNotifyError::from)?;

    debug!("Received upload URL response");

    if !upload_url_json
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        let error_msg = upload_url_json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");

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
    debug!("Uploading file content to URL (file_id: {})", file_id);
    client
        .post(upload_url)
        .body(file_content)
        .send()
        .map_err(SlackNotifyError::from)?;

    debug!("File upload completed successfully");
    Ok((file_id.to_string(), file_name))
}

/// Send a simple message to Slack without attachments using Block Kit
/// Returns the response JSON and optionally the thread timestamp
fn send_simple_message(
    client: &reqwest::blocking::Client,
    token: &str,
    channel: &str,
    text: &str,
    blocks: &[serde_json::Value],
    thread_ts: Option<&str>,
) -> Result<(String, Option<String>), SlackNotifyError> {
    debug!(
        "Sending simple message to channel: {} (thread: {})",
        channel,
        thread_ts.unwrap_or("none")
    );

    let mut payload = json!({
        "channel": channel,
        "text": text,
        "blocks": blocks,
    });

    // Add thread_ts if provided
    if let Some(ts) = thread_ts {
        payload["thread_ts"] = json!(ts);
    }

    let res = client
        .post("https://slack.com/api/chat.postMessage")
        .bearer_auth(token)
        .json(&payload)
        .send()
        .map_err(SlackNotifyError::from)?;

    debug!("Received response from chat.postMessage");

    if !res.status().is_success() {
        return Err(SlackNotifyError::SlackApi {
            message: format!("Failed to send Slack notification: {}", res.status()),
        });
    }

    let response_text = res.text().unwrap_or_else(|_| String::from("{}"));

    // Extract the timestamp from the response for threading
    let ts = serde_json::from_str::<serde_json::Value>(&response_text)
        .ok()
        .and_then(|v| v.get("ts").and_then(|t| t.as_str()).map(String::from));

    Ok((response_text, ts))
}

/// Send a message with optional file attachments to Slack
#[expect(
    clippy::too_many_lines,
    reason = "TODO: Refactor into smaller functions"
)]
fn send_slack_message(
    token: &str,
    channel: &str,
    text: &str,
    blocks: &[serde_json::Value],
    attachment_paths: &[PathBuf],
    run_name: &str,
) -> Result<(String, Vec<String>), SlackNotifyError> {
    tracing::debug!("Sending Slack message to channel: {}", channel);
    tracing::debug!("Attachment paths: {} files", attachment_paths.len());

    let client = reqwest::blocking::Client::new();

    // If no attachments, use simple chat.postMessage
    if attachment_paths.is_empty() {
        tracing::debug!("No attachments, sending simple message");
        let (response, _ts) = send_simple_message(&client, token, channel, text, blocks, None)?;
        return Ok((response, Vec::new()));
    }

    // With attachments, use the new Slack files API workflow:
    // 1. Get upload URLs
    // 2. Upload files to those URLs
    // 3. Complete the upload and share to channel

    let mut file_ids = Vec::new();
    let mut attached_files = Vec::new();

    // Step 1 & 2: Get upload URL and upload each file
    tracing::debug!("Uploading {} files to Slack", attachment_paths.len());
    for path in attachment_paths {
        tracing::debug!("Uploading file: {}", path.display());
        let (file_id, file_name) = upload_file_to_slack(&client, token, path)?;
        tracing::debug!(
            "File uploaded successfully: {} (ID: {})",
            file_name,
            file_id
        );
        file_ids.push(file_id);
        attached_files.push(file_name);
    }

    // Step 4: Send the main message with blocks first
    tracing::debug!("Sending main message with blocks");
    let (message_response, thread_ts) =
        send_simple_message(&client, token, channel, text, blocks, None)?;

    // Step 5: Share files in thread with initial_comment and thread_ts
    let ts = thread_ts.ok_or_else(|| SlackNotifyError::SlackApi {
        message: "Failed to get timestamp from message response".to_string(),
    })?;
    tracing::debug!("Main message sent, thread timestamp: {}", ts);

    let files_json = serde_json::to_string(
        &file_ids
            .iter()
            .map(|id| json!({"id": id}))
            .collect::<Vec<_>>(),
    )
    .map_err(SlackNotifyError::Serialization)?;

    // Complete upload WITHOUT sharing to channel - just finalize the upload
    // This way the file is uploaded but not posted anywhere yet
    tracing::debug!("Completing file upload without sharing to channel");
    let complete_form = reqwest::blocking::multipart::Form::new().text("files", files_json);

    let complete_res = client
        .post("https://slack.com/api/files.completeUploadExternal")
        .bearer_auth(token)
        .multipart(complete_form)
        .send()
        .map_err(SlackNotifyError::from)?;

    if !complete_res.status().is_success() {
        return Err(SlackNotifyError::SlackApi {
            message: format!("Failed to share files in thread: {}", complete_res.status()),
        });
    }

    let complete_json: serde_json::Value = complete_res.json().map_err(SlackNotifyError::from)?;

    if !complete_json
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        let error_msg = complete_json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(SlackNotifyError::SlackApi {
            message: format!("Failed to share files in thread: {error_msg}"),
        });
    }

    // Step 6: Extract file permalinks from response for broadcast message
    tracing::debug!("Extracting file permalinks from response");
    let mut file_permalinks = Vec::new();

    if let Some(files_info) = complete_json.get("files").and_then(|v| v.as_array()) {
        for file in files_info {
            if let Some(permalink) = file.get("permalink").and_then(|v| v.as_str()) {
                tracing::debug!("Found permalink: {}", permalink);
                file_permalinks.push(permalink.to_string());
            }
        }
    }
    tracing::debug!("Extracted {} permalinks", file_permalinks.len());

    // Step 7: Post a broadcast message with file links
    tracing::debug!(
        "Posting broadcast message with file links for run: {}",
        run_name
    );
    let file_links_text = if file_permalinks.is_empty() {
        format!(
            "📎 `{}`: {} file(s) attached",
            run_name,
            attached_files.len()
        )
    } else {
        let links = file_permalinks
            .iter()
            .zip(&attached_files)
            .map(|(link, name)| format!("<{link}|{name}>"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("📎 Attachments (Run Name: `{run_name}`):\n{links}")
    };

    let broadcast_payload = json!({
        "channel": channel,
        "thread_ts": ts,
        "reply_broadcast": true,
        "text": file_links_text,
        "unfurl_links": true,
        "unfurl_media": true,
    });

    let broadcast_res = client
        .post("https://slack.com/api/chat.postMessage")
        .bearer_auth(token)
        .json(&broadcast_payload)
        .send()
        .map_err(SlackNotifyError::from)?;

    if broadcast_res.status().is_success() {
        debug!("Broadcast message sent successfully");
    } else {
        // Don't fail the whole operation if broadcast fails
        warn!(
            "Warning: Failed to broadcast file message: {}",
            broadcast_res.status()
        );
    }

    Ok((message_response, attached_files))
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

impl SlackNotifyHook {
    /// Shared deserializer used by both `Hook<PreRun>` and `Hook<PostRun>`
    /// impls below. Validates that a token is present after applying the
    /// `SLACK_BOT_TOKEN` env fallback.
    fn build(config: &serde_json::Value, project_root: &Path) -> CapsulaResult<Self> {
        let config = serde_json::from_value::<SlackNotifyHookConfig>(config.clone())?;
        if config.token.is_empty() {
            return Err(SlackNotifyError::MissingToken.into());
        }
        Ok(Self {
            config,
            project_root: project_root.to_path_buf(),
        })
    }

    /// Send the notification. `header_text` decorates the Block Kit header
    /// (e.g. "🚀 Capsula Run Starting") and `status_phrase` is embedded in
    /// the plain-text fallback (e.g. "is starting" vs "has completed").
    fn send(
        &self,
        metadata: &PreparedRun,
        phase_label: &'static str,
        header_text: &str,
        status_phrase: &str,
    ) -> CapsulaResult<SlackNotifyCaptured> {
        debug!(
            "SlackNotifyHook ({phase_label}): Sending notification for run '{}' (ID: {})",
            metadata.name, metadata.id
        );

        let fallback_text = format!(
            "Run `{}` (ID: `{}`) {status_phrase}.",
            metadata.name, metadata.id
        );

        let command_display = shlex::try_join(metadata.command.iter().map(String::as_str))
            .unwrap_or_else(|_| metadata.command.join(" "));

        let blocks = build_slack_blocks(
            header_text,
            &metadata.name,
            metadata.id,
            metadata.timestamp().timestamp(),
            &metadata.timestamp().to_rfc3339(),
            &command_display,
        );

        let attachment_paths =
            resolve_attachment_globs(&self.config.attachment_globs, &self.project_root)?;

        let (response, attached_files) = send_slack_message(
            self.config.token.expose(),
            &self.config.channel,
            &fallback_text,
            &blocks,
            &attachment_paths,
            &metadata.name,
        )?;

        debug!(
            "SlackNotifyHook ({phase_label}): Notification sent successfully with {} attachments",
            attached_files.len()
        );

        Ok(SlackNotifyCaptured {
            message: "Slack notification sent successfully".to_string(),
            response: Some(response),
            attached_files,
        })
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
        Self::build(config, project_root)
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        _params: &RuntimeParams<PreRun>,
    ) -> CapsulaResult<Self::Output> {
        self.send(metadata, "PreRun", "🚀 Capsula Run Starting", "is starting")
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
        Self::build(config, project_root)
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        _params: &RuntimeParams<PostRun>,
    ) -> CapsulaResult<Self::Output> {
        self.send(
            metadata,
            "PostRun",
            "✅ Capsula Run Completed",
            "has completed",
        )
    }
}
