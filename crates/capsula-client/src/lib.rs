use serde::{Deserialize, Serialize};
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
    client: reqwest::Client,
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

impl CapsulaClient {
    /// Create a new Capsula client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// List all vaults on the server
    pub async fn list_vaults(&self) -> Result<Vec<VaultInfo>> {
        let url = format!("{}/api/vaults", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Failed to list vaults: {}",
                response.status()
            )));
        }

        let vaults_response: VaultsResponse = response.json().await?;
        Ok(vaults_response.vaults)
    }

    /// Check if a vault exists on the server
    pub async fn vault_exists(&self, vault_name: &str) -> Result<Option<VaultInfo>> {
        let url = format!("{}/api/vaults/{}", self.base_url, vault_name);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError(format!(
                "Failed to check vault: {}",
                response.status()
            )));
        }

        let vault_response: VaultExistsResponse = response.json().await?;
        Ok(vault_response.vault)
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
