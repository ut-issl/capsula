use capsula_api_types::{UploadResponse, VaultExistsResponse, VaultInfo, VaultsResponse};
use reqwest::StatusCode;
use reqwest::blocking::multipart;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
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

    #[error("Invalid header '{name}': {message}")]
    InvalidHeader { name: String, message: String },
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

    /// Create a client that attaches the given headers to every request.
    pub fn with_headers(base_url: impl Into<String>, headers: &[(String, String)]) -> Result<Self> {
        let mut header_map = HeaderMap::new();
        for (name, value) in headers {
            let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
                ClientError::InvalidHeader {
                    name: name.clone(),
                    message: e.to_string(),
                }
            })?;
            let mut header_value =
                HeaderValue::from_str(value).map_err(|e| ClientError::InvalidHeader {
                    name: name.clone(),
                    message: e.to_string(),
                })?;
            // Keep credential values out of debug logs
            header_value.set_sensitive(true);
            header_map.insert(header_name, header_value);
        }

        let client = reqwest::blocking::Client::builder()
            .default_headers(header_map)
            .build()?;

        Ok(Self {
            base_url: base_url.into(),
            client,
        })
    }

    fn server_error(action: &str, status: StatusCode) -> ClientError {
        let auth_hint = if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            " (authentication may be missing or expired; check [server.headers] in capsula.toml)"
        } else {
            ""
        };
        ClientError::ServerError(format!("{action}: {status}{auth_hint}"))
    }

    /// List all vaults on the server
    pub fn list_vaults(&self) -> Result<Vec<VaultInfo>> {
        let url = format!("{}/api/v1/vaults", self.base_url);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(Self::server_error(
                "Failed to list vaults",
                response.status(),
            ));
        }

        let vaults_response: VaultsResponse = response.json()?;
        Ok(vaults_response.vaults)
    }

    /// Check if a vault exists on the server
    pub fn vault_exists(&self, vault_name: &str) -> Result<Option<VaultInfo>> {
        let url = format!("{}/api/v1/vaults/{}", self.base_url, vault_name);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(Self::server_error(
                "Failed to check vault",
                response.status(),
            ));
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
            return Err(Self::server_error("Upload failed", response.status()));
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
    fn rejects_invalid_header_name() {
        let result = CapsulaClient::with_headers(
            "http://localhost:8500",
            &[("bad header".to_string(), "value".to_string())],
        );

        assert!(matches!(result, Err(ClientError::InvalidHeader { .. })));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sends_configured_headers_with_requests() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/vaults"))
            .and(header("cf-access-token", "test-jwt"))
            .and(header("authorization", "Bearer abc123"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "status": "ok", "vaults": [] })),
            )
            .mount(&server)
            .await;

        let uri = server.uri();
        // The blocking client must be created and dropped outside the async runtime
        let vaults = tokio::task::spawn_blocking(move || {
            let client = CapsulaClient::with_headers(
                uri,
                &[
                    ("cf-access-token".to_string(), "test-jwt".to_string()),
                    ("Authorization".to_string(), "Bearer abc123".to_string()),
                ],
            )
            .unwrap();
            client.list_vaults()
        })
        .await
        .unwrap()
        .unwrap();

        assert!(vaults.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn hints_at_authentication_on_forbidden_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/vaults"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let uri = server.uri();
        let error = tokio::task::spawn_blocking(move || CapsulaClient::new(uri).list_vaults())
            .await
            .unwrap()
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("authentication may be missing or expired")
        );
    }
}
