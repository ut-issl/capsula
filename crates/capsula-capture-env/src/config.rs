use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvVarHookConfig {
    pub name: String,
}
