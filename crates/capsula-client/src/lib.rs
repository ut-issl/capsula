use capsula_api_types::{
    RunDetailResponse, UploadResponse, VaultExistsResponse, VaultInfo, VaultsResponse,
};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::StatusCode;
use reqwest::blocking::multipart;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use serde_json::Value as JsonValue;
use std::path::Path;
use thiserror::Error;

/// `NON_ALPHANUMERIC` minus the RFC 3986 unreserved characters (`-`,
/// `.`, `_`, `~`), whose escaped and unescaped forms are defined to be
/// equivalent (§2.3: such escapes "should not be created").
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Percent-encode a run-relative file path for use as a URL path suffix.
///
/// Each `/`-separated segment is encoded with allowlist polarity
/// (everything except `[0-9A-Za-z]` and the RFC 3986 unreserved set is
/// encoded), so any byte round-trips through the server's
/// `/files/{*path}` wildcard route without a curated character list.
/// This notably covers `\`, which the WHATWG URL parser would otherwise
/// rewrite to `/` in an http(s) path, silently changing the requested
/// file.
fn encode_file_path(file_path: &str) -> String {
    file_path
        .split('/')
        .map(|segment| utf8_percent_encode(segment, PATH_SEGMENT_ENCODE_SET).to_string())
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

    #[error("Invalid header '{name}': {message}")]
    InvalidHeader { name: String, message: String },
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// Outcome of registering a run on the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateRunOutcome {
    /// The run was newly created.
    Created,
    /// A run with the same ID already exists on the server.
    AlreadyExists,
}

#[derive(Debug, Clone)]
pub struct CapsulaClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl CapsulaClient {
    /// Create a new Capsula client
    pub fn new(base_url: impl Into<String>) -> Self {
        #[expect(
            clippy::expect_used,
            reason = "mirrors reqwest::blocking::Client::new(), which panics if the client cannot be initialized"
        )]
        let client = reqwest::blocking::Client::builder()
            .redirect(Policy::none())
            .build()
            .expect("failed to initialize HTTP client");
        Self {
            base_url: base_url.into(),
            client,
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

        // Never follow redirects: a cross-origin redirect would forward the
        // configured credential headers to the redirect target
        let client = reqwest::blocking::Client::builder()
            .default_headers(header_map)
            .redirect(Policy::none())
            .build()?;

        Ok(Self {
            base_url: base_url.into(),
            client,
        })
    }

    fn server_error(action: &str, status: StatusCode) -> ClientError {
        let hint = if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
            " (authentication may be missing or expired; check [server.headers] in capsula.toml)"
        } else if status.is_redirection() {
            " (the server redirected the request, possibly to a login page; redirects are not followed — check the server URL and authentication configuration)"
        } else {
            ""
        };
        ClientError::ServerError(format!("{action}: {status}{hint}"))
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
            return Err(Self::server_error(
                &format!("Failed to fetch run {run_id}"),
                response.status(),
            ));
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
            return Err(Self::server_error(
                &format!("Failed to download file '{file_path}' of run {run_id}"),
                response.status(),
            ));
        }

        Ok(response.bytes()?.to_vec())
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

    /// Register a run's metadata on the server.
    pub fn create_run(&self, run: &JsonValue) -> Result<CreateRunOutcome> {
        let url = format!("{}/api/v1/runs", self.base_url);
        let response = self.client.post(&url).json(run).send()?;

        match response.status() {
            StatusCode::CONFLICT => Ok(CreateRunOutcome::AlreadyExists),
            status if status.is_success() => Ok(CreateRunOutcome::Created),
            status => Err(Self::server_error("Failed to create run on server", status)),
        }
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

        assert_eq!(vaults, [] as [capsula_api_types::VaultInfo; 0]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_run_sends_headers_and_maps_conflict_to_already_exists() {
        use wiremock::matchers::{body_json, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let created = MockServer::start().await;
        let payload = serde_json::json!({ "id": "01RUN", "vault": "v" });
        Mock::given(method("POST"))
            .and(path("/api/v1/runs"))
            .and(header("x-auth", "secret"))
            .and(body_json(payload.clone()))
            .respond_with(ResponseTemplate::new(201))
            .mount(&created)
            .await;

        let conflicting = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/runs"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&conflicting)
            .await;

        let created_uri = created.uri();
        let conflicting_uri = conflicting.uri();
        let (first, second) = tokio::task::spawn_blocking(move || {
            let auth = [("x-auth".to_string(), "secret".to_string())];
            let first = CapsulaClient::with_headers(created_uri, &auth)
                .unwrap()
                .create_run(&payload);
            let second = CapsulaClient::with_headers(conflicting_uri, &auth)
                .unwrap()
                .create_run(&payload);
            (first, second)
        })
        .await
        .unwrap();

        assert_eq!(first.unwrap(), CreateRunOutcome::Created);
        assert_eq!(second.unwrap(), CreateRunOutcome::AlreadyExists);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn vault_exists_sends_configured_headers() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/vaults/my-vault"))
            .and(header("x-auth", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "status": "ok", "exists": false, "vault": null }),
            ))
            .mount(&server)
            .await;

        let uri = server.uri();
        let vault = tokio::task::spawn_blocking(move || {
            CapsulaClient::with_headers(uri, &[("x-auth".to_string(), "secret".to_string())])
                .unwrap()
                .vault_exists("my-vault")
        })
        .await
        .unwrap()
        .unwrap();

        assert!(vault.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn does_not_follow_redirects_and_reports_them() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let target = MockServer::start().await;
        let origin = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/vaults"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("location", format!("{}/api/v1/vaults", target.uri())),
            )
            .mount(&origin)
            .await;

        let uri = origin.uri();
        let error = tokio::task::spawn_blocking(move || {
            CapsulaClient::with_headers(
                uri,
                &[(
                    "cf-access-client-secret".to_string(),
                    "top-secret".to_string(),
                )],
            )
            .unwrap()
            .list_vaults()
        })
        .await
        .unwrap()
        .unwrap_err();

        assert!(error.to_string().contains("redirects are not followed"));
        // The credential header must never reach the redirect target
        assert!(target.received_requests().await.unwrap().is_empty());
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

    #[test]
    fn encode_file_path_keeps_separators_and_unreserved_chars() {
        assert_eq!(encode_file_path("output/result.txt"), "output/result.txt");
        assert_eq!(encode_file_path("a-b_c~d.e/f"), "a-b_c~d.e/f");
    }

    #[test]
    fn encode_file_path_encodes_url_significant_and_non_ascii_chars() {
        assert_eq!(encode_file_path("a b?c#d%e"), "a%20b%3Fc%23d%25e");
        assert_eq!(
            encode_file_path("日本語.txt"),
            "%E6%97%A5%E6%9C%AC%E8%AA%9E.txt"
        );
    }
}
