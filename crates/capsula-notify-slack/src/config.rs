use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackNotifyHookConfig {
    pub channel: String,
    pub token: String,
}
