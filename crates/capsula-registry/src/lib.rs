use capsula_core::context::{ContextErased, ContextFactory};
use capsula_core::error::{CoreError, CoreResult};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Context type '{0}' not found in registry")]
    ContextTypeNotFound(String),
    #[error("Context type '{0}' already registered")]
    AlreadyRegistered(String),
}

impl From<RegistryError> for CoreError {
    fn from(err: RegistryError) -> Self {
        CoreError::from(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

/// Context factory registry
pub struct ContextRegistry {
    factories: HashMap<&'static str, Box<dyn ContextFactory>>,
}

impl ContextRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a context factory
    pub fn register(&mut self, factory: Box<dyn ContextFactory>) -> Result<(), RegistryError> {
        let context_type = factory.key();
        if self.factories.contains_key(context_type) {
            return Err(RegistryError::AlreadyRegistered(context_type.to_string()));
        }

        self.factories.insert(context_type, factory);
        Ok(())
    }

    /// Create a context from type name and configuration
    pub fn create_context(
        &self,
        context_type: &str,
        config: &Value,
        project_root: &Path,
    ) -> CoreResult<Box<dyn ContextErased>> {
        let factory = self
            .factories
            .get(context_type)
            .ok_or_else(|| RegistryError::ContextTypeNotFound(context_type.to_string()))?;

        factory.create_context(config, project_root)
    }

    /// Get list of registered context types
    pub fn registered_types(&self) -> Vec<&'static str> {
        self.factories.keys().copied().collect()
    }
}

impl Default for ContextRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for setting up a registry with standard context types
pub struct RegistryBuilder {
    registry: ContextRegistry,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
        }
    }

    pub fn with_factory(mut self, factory: Box<dyn ContextFactory>) -> Result<Self, RegistryError> {
        self.registry.register(factory)?;
        Ok(self)
    }

    pub fn build(self) -> ContextRegistry {
        self.registry
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a standard registry with all built-in context types
///
/// This is feature-gated so only enabled contexts are included:
/// - "ctx-cwd": includes CWD context
/// - "ctx-git": includes Git context
///
/// You can disable contexts by turning off features in Cargo.toml
pub fn standard_registry() -> ContextRegistry {
    let mut builder = RegistryBuilder::new();

    #[cfg(feature = "ctx-cwd")]
    {
        builder = builder
            .with_factory(capsula_cwd_context::create_factory())
            .expect("Failed to register CWD context");
    }

    #[cfg(feature = "ctx-git")]
    {
        builder = builder
            .with_factory(capsula_git_context::create_factory())
            .expect("Failed to register Git context");
    }

    #[cfg(feature = "ctx-file")]
    {
        builder = builder
            .with_factory(capsula_file_context::create_factory())
            .expect("Failed to register File context");
    }

    #[cfg(feature = "ctx-env")]
    {
        builder = builder
            .with_factory(capsula_env_context::create_factory())
            .expect("Failed to register Env context");
    }

    #[cfg(feature = "ctx-command")]
    {
        builder = builder
            .with_factory(capsula_command_context::create_factory())
            .expect("Failed to register Command context");
    }

    builder.build()
}
