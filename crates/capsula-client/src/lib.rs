use reqwest::blocking::multipart;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::Path;
use thiserror::Error;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultInfo {
    pub name: String,
    pub run_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultsResponse {
    pub status: String,
    pub vaults: Vec<VaultInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultExistsResponse {
    pub status: String,
    pub exists: bool,
    pub vault: Option<VaultInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    pub status: String,
    pub files_processed: u64,
    pub total_bytes: u64,
    pub pre_run_hooks: u64,
    pub post_run_hooks: u64,
}

impl CapsulaClient {
    /// Create a new Capsula client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
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
        let client = CapsulaClient::new("http://localhost:3000");
        assert_eq!(client.base_url, "http://localhost:3000");
    }
}
