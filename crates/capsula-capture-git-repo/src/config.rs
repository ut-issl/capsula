use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for GitHook
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHookConfig {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub allow_dirty: bool,
}
