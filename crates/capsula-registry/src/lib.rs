use capsula_core::context::{ContextErased, ContextFactory};
use capsula_core::error::{CapsulaError, CoreResult};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Unknown context type: '{0}'")]
    ContextTypeNotFound(String),

    #[error("Context type '{0}' is already registered")]
    AlreadyRegistered(String),

    #[error("Failed to create context '{context}': {message}")]
    ContextCreationFailed { context: String, message: String },
}

impl From<RegistryError> for CapsulaError {
    fn from(err: RegistryError) -> Self {
        match err {
            RegistryError::ContextTypeNotFound(ty) => CapsulaError::Configuration {
                message: format!(
                    "Unknown context type '{}'. Check your configuration file for typos.",
                    ty
                ),
            },
            RegistryError::AlreadyRegistered(ty) => CapsulaError::Configuration {
                message: format!("Context type '{}' is already registered", ty),
            },
            RegistryError::ContextCreationFailed { context, message } => {
                CapsulaError::Configuration {
                    message: format!("Failed to create '{}' context: {}", context, message),
                }
            }
        }
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
        let factory = self.factories.get(context_type).ok_or_else(|| {
            let available = self.registered_types().join(", ");
            CapsulaError::Configuration {
                message: format!(
                    "Unknown context type '{}'. Available types: {}",
                    context_type, available
                ),
            }
        })?;

        factory.create_context(config, project_root).map_err(|e| {
            // Enhance error message with context type information
            match e {
                CapsulaError::ContextFailed { .. } => e,
                _ => CapsulaError::ContextFailed {
                    context: context_type.to_string(),
                    message: e.to_string(),
                    source: Box::new(e),
                },
            }
        })
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
            .unwrap_or_else(|e| panic!("Failed to register CWD context: {}", e));
    }

    #[cfg(feature = "ctx-git")]
    {
        builder = builder
            .with_factory(capsula_git_context::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register Git context: {}", e));
    }

    #[cfg(feature = "ctx-file")]
    {
        builder = builder
            .with_factory(capsula_file_context::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register File context: {}", e));
    }

    #[cfg(feature = "ctx-env")]
    {
        builder = builder
            .with_factory(capsula_env_context::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register Env context: {}", e));
    }

    #[cfg(feature = "ctx-command")]
    {
        builder = builder
            .with_factory(capsula_command_context::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register Command context: {}", e));
    }

    #[cfg(feature = "ctx-machine")]
    {
        builder = builder
            .with_factory(capsula_machine_context::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register Machine context: {}", e));
    }

    builder.build()
}
