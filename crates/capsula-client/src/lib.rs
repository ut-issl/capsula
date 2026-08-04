use capsula_api_types::{
    RunDetailResponse, UploadResponse, VaultExistsResponse, VaultInfo, VaultsResponse,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::blocking::multipart;
use serde_json::Value as JsonValue;
use std::path::Path;
use thiserror::Error;

/// Percent-encode a run-relative file path for use as a URL path suffix.
///
/// Each `/`-separated segment is encoded with `NON_ALPHANUMERIC`
/// (allowlist polarity: everything except `[0-9A-Za-z]` is encoded), so
/// any byte round-trips through the server's `/files/{*path}` wildcard
/// route without a curated character list. This notably covers `\`,
/// which the WHATWG URL parser would otherwise rewrite to `/` in an
/// http(s) path, silently changing the requested file.
fn encode_file_path(file_path: &str) -> String {
    file_path
        .split('/')
        .map(|segment| utf8_percent_encode(segment, NON_ALPHANUMERIC).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Server returned error: {0}")]
    ServerError(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug, Clone)]
pub struct CapsulaClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl CapsulaClient {
    /// Create a new Capsula client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Base URL of the server this client talks to
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Fetch a run's detail (record, hook outputs, and captured file list)
    pub fn get_run(&self, run_id: &str) -> Result<RunDetailResponse> {
        let url = format!("{}/api/v1/runs/{}", self.base_url, run_id);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Failed to fetch run {}: {}",
                run_id,
                response.status()
            )));
        }

        let detail: RunDetailResponse = response.json()?;
        Ok(detail)
    }

    /// Download a captured file of a run, returning its raw bytes
    pub fn download_file(&self, run_id: &str, file_path: &str) -> Result<Vec<u8>> {
        let url = format!(
            "{}/api/v1/runs/{}/files/{}",
            self.base_url,
            run_id,
            encode_file_path(file_path)
        );
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Failed to download file '{}' of run {}: {}",
                file_path,
                run_id,
                response.status()
            )));
        }

        Ok(response.bytes()?.to_vec())
    }

    /// List all vaults on the server
    pub fn list_vaults(&self) -> Result<Vec<VaultInfo>> {
        let url = format!("{}/api/v1/vaults", self.base_url);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Failed to list vaults: {}",
                response.status()
            )));
        }

        let vaults_response: VaultsResponse = response.json()?;
        Ok(vaults_response.vaults)
    }

    /// Check if a vault exists on the server
    pub fn vault_exists(&self, vault_name: &str) -> Result<Option<VaultInfo>> {
        let url = format!("{}/api/v1/vaults/{}", self.base_url, vault_name);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Failed to check vault: {}",
                response.status()
            )));
        }

        let vault_response: VaultExistsResponse = response.json()?;
        Ok(vault_response.vault)
    }

    /// Upload a run's data and files to the server
    pub fn upload_run(
        &self,
        run_id: &str,
        files: &[(impl AsRef<Path>, impl AsRef<Path>)],
        pre_run_hooks: Option<Vec<JsonValue>>,
        post_run_hooks: Option<Vec<JsonValue>>,
    ) -> Result<UploadResponse> {
        let url = format!("{}/api/v1/upload", self.base_url);

        let mut form = multipart::Form::new().text("run_id", run_id.to_string());

        // Add pre-run hooks if provided
        if let Some(hooks) = pre_run_hooks {
            let hooks_json = serde_json::to_string(&hooks)?;
            form = form.text("pre_run", hooks_json);
        }

        // Add post-run hooks if provided
        if let Some(hooks) = post_run_hooks {
            let hooks_json = serde_json::to_string(&hooks)?;
            form = form.text("post_run", hooks_json);
        }

        // Add files
        for (local_path, relative_path) in files {
            let local_path = local_path.as_ref();
            let relative_path = relative_path.as_ref();

            // Read file content
            let content = std::fs::read(local_path)?;

            // Add path field
            form = form.text("path", relative_path.to_string_lossy().to_string());

            // Add file part
            let file_name = local_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");

            let part = multipart::Part::bytes(content).file_name(file_name.to_string());

            form = form.part("file", part);
        }

        let response = self.client.post(&url).multipart(form).send()?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Upload failed: {}",
                response.status()
            )));
        }

        let upload_response: UploadResponse = response.json()?;
        Ok(upload_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CapsulaClient::new("http://localhost:8500");
        assert_eq!(client.base_url, "http://localhost:8500");
    }

    #[test]
    fn encode_file_path_keeps_segment_separators() {
        assert_eq!(encode_file_path("output/result.txt"), "output/result%2Etxt");
    }

    #[test]
    fn encode_file_path_encodes_url_significant_and_non_ascii_chars() {
        assert_eq!(encode_file_path("a b?c#d%e"), "a%20b%3Fc%23d%25e");
        assert_eq!(
            encode_file_path("日本語.txt"),
            "%E6%97%A5%E6%9C%AC%E8%AA%9E%2Etxt"
        );
    }
}
