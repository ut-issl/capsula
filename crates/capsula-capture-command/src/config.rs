use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandHookConfig {
    pub command: Vec<String>,
    #[serde(default)]
    pub abort_on_failure: bool,
}
