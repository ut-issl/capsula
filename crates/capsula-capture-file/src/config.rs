use crate::{CaptureMode, FileContext, HashAlgorithm, KEY};
use capsula_core::context::ContextErased;
use capsula_core::context::ContextFactory;
use capsula_core::error::CoreResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FileContextConfig {
    pub glob: String,
    #[serde(default)]
    pub mode: CaptureMode,
    #[serde(default)]
    pub hash: HashAlgorithm,
}

pub struct FileContextFactory;

impl ContextFactory for FileContextFactory {
    fn key(&self) -> &'static str {
        KEY
    }

    fn create_context(
        &self,
        config: &Value,
        _project_root: &Path,
    ) -> CoreResult<Box<dyn ContextErased>> {
        let config: FileContextConfig = serde_json::from_value(config.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let context = FileContext {
            glob: config.glob,
            mode: config.mode,
            hash: config.hash,
        };

        Ok(Box::new(context))
    }
}
