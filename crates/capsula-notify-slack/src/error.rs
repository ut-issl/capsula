use capsula_core::error::CapsulaError;
use thiserror::Error;

/// Slack notification hook specific errors
#[derive(Debug, Error)]
pub enum SlackNotifyError {
    /// HTTP request to Slack API failed
    #[error("HTTP request to Slack API failed: {0}")]
    HttpRequest(#[from] reqwest::Error),
    /// Slack API returned an error
    #[error("Slack API returned an error: {message}")]
    SlackApi { message: String },
    /// Serialization failed
    #[error("Failed to serialize Slack notify hook: {0}")]
    Serialization(#[from] serde_json::Error),
    /// Slack token not provided
    #[error("Slack token not provided in config and SLACK_BOT_TOKEN environment variable not set")]
    MissingToken,
    /// Glob pattern error
    #[error("Invalid glob pattern '{pattern}': {source}")]
    GlobPattern {
        pattern: String,
        source: glob::PatternError,
    },
    /// File I/O error
    #[error("Failed to read attachment file '{path}': {source}")]
    FileIo {
        path: String,
        source: std::io::Error,
    },
}

/// Convert `SlackNotifyError` to `CapsulaError`
impl From<SlackNotifyError> for CapsulaError {
    fn from(err: SlackNotifyError) -> Self {
        Self::HookFailed {
            hook: "notify-slack".to_string(),
            source: Box::new(err),
        }
    }
}
