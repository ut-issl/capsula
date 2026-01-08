use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Run {
    pub id: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub vault: String,
    pub project_root: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRunRequest {
    pub id: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub vault: String,
    pub project_root: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}
