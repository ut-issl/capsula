use crate::{CaptureMode, HashAlgorithm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileHookConfig {
    pub glob: String,
    #[serde(default)]
    pub mode: CaptureMode,
    #[serde(default)]
    pub hash: HashAlgorithm,
}
