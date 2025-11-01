use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{HookErased, HookFactory, PhaseMarker, PostRun, PreRun};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Unknown hook type: '{0}'")]
    HookTypeNotFound(String),

    #[error("Hook id '{0}' is already registered")]
    AlreadyRegistered(String),

    #[error("Failed to create hook '{hook}': {message}")]
    HookCreationFailed { hook: String, message: String },
}

impl From<RegistryError> for CapsulaError {
    fn from(err: RegistryError) -> Self {
        match err {
            RegistryError::HookTypeNotFound(ty) => CapsulaError::Configuration {
                message: format!(
                    "Unknown hook id '{}'. Check your configuration file for typos.",
                    ty
                ),
            },
            RegistryError::AlreadyRegistered(ty) => CapsulaError::Configuration {
                message: format!("Hook id '{}' is already registered", ty),
            },
            RegistryError::HookCreationFailed { hook, message } => CapsulaError::Configuration {
                message: format!("Failed to create '{}' hook: {}", hook, message),
            },
        }
    }
}

/// Hook factory registry
pub struct HookRegistry<P: PhaseMarker> {
    factories: HashMap<&'static str, Box<dyn HookFactory<P>>>,
}

impl<P: PhaseMarker> HookRegistry<P> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a hook factory
    pub fn register(&mut self, factory: Box<dyn HookFactory<P>>) -> Result<(), RegistryError> {
        let hook_id = factory.key();
        if self.factories.contains_key(hook_id) {
            return Err(RegistryError::AlreadyRegistered(hook_id.to_string()));
        }

        self.factories.insert(hook_id, factory);
        Ok(())
    }

    /// Create a hook from type name and configuration
    pub fn create_hook(
        &self,
        hook_id: &str,
        config: &Value,
        project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased<P>>> {
        let factory = self.factories.get(hook_id).ok_or_else(|| {
            let available = self.registered_types().join(", ");
            CapsulaError::Configuration {
                message: format!(
                    "Unknown hook id '{}'. Available types: {}",
                    hook_id, available
                ),
            }
        })?;

        factory.create_hook(config, project_root).map_err(|e| {
            // Enhance error message with hook type information
            match e {
                CapsulaError::HookFailed { .. } => e,
                _ => CapsulaError::HookFailed {
                    hook: hook_id.to_string(),
                    message: e.to_string(),
                    source: Box::new(e),
                },
            }
        })
    }

    /// Get list of registered hook types
    pub fn registered_types(&self) -> Vec<&'static str> {
        self.factories.keys().copied().collect()
    }
}

impl<P> Default for HookRegistry<P>
where
    P: PhaseMarker,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for setting up a registry with standard hook types
pub struct RegistryBuilder<P: PhaseMarker> {
    registry: HookRegistry<P>,
}

impl<P: PhaseMarker> RegistryBuilder<P> {
    pub fn new() -> Self {
        Self {
            registry: HookRegistry::new(),
        }
    }

    pub fn with_factory(mut self, factory: Box<dyn HookFactory<P>>) -> Result<Self, RegistryError> {
        self.registry.register(factory)?;
        Ok(self)
    }

    pub fn build(self) -> HookRegistry<P> {
        self.registry
    }
}

impl<P> Default for RegistryBuilder<P>
where
    P: PhaseMarker,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Create a standard registry with all built-in hook types
///
/// This is feature-gated so only enabled hooks are included:
/// - "hook-cwd": includes CWD hook
/// - "hook-git": includes Git hook
///
/// You can disable hooks by turning off features in Cargo.toml
pub fn standard_pre_run_hook_registry() -> HookRegistry<PreRun> {
    let mut builder = RegistryBuilder::new();

    #[cfg(feature = "hook-cwd")]
    {
        builder = builder
            .with_factory(capsula_capture_cwd::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-cwd hook: {}", e));
    }

    #[cfg(feature = "hook-git")]
    {
        builder = builder
            .with_factory(capsula_capture_git_repo::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-git-repo hook: {}", e));
    }

    #[cfg(feature = "hook-file")]
    {
        builder = builder
            .with_factory(capsula_capture_file::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-file hook: {}", e));
    }

    #[cfg(feature = "hook-env")]
    {
        builder = builder
            .with_factory(capsula_capture_env::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-env hook: {}", e));
    }

    #[cfg(feature = "hook-command")]
    {
        builder = builder
            .with_factory(capsula_capture_command::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-command hook: {}", e));
    }

    #[cfg(feature = "hook-machine")]
    {
        builder = builder
            .with_factory(capsula_capture_machine::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register Machine hook: {}", e));
    }

    #[cfg(feature = "hook-slack")]
    {
        builder = builder
            .with_factory(capsula_notify_slack::create_pre_run_hook_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-slack hook: {}", e));
    }

    builder.build()
}

pub fn standard_post_run_hook_registry() -> HookRegistry<PostRun> {
    let mut builder = RegistryBuilder::new();

    #[cfg(feature = "hook-cwd")]
    {
        builder = builder
            .with_factory(capsula_capture_cwd::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-cwd hook: {}", e));
    }

    #[cfg(feature = "hook-git")]
    {
        builder = builder
            .with_factory(capsula_capture_git_repo::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-git-repo hook: {}", e));
    }

    #[cfg(feature = "hook-file")]
    {
        builder = builder
            .with_factory(capsula_capture_file::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-file hook: {}", e));
    }

    #[cfg(feature = "hook-env")]
    {
        builder = builder
            .with_factory(capsula_capture_env::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-env hook: {}", e));
    }

    #[cfg(feature = "hook-command")]
    {
        builder = builder
            .with_factory(capsula_capture_command::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-command hook: {}", e));
    }

    #[cfg(feature = "hook-machine")]
    {
        builder = builder
            .with_factory(capsula_capture_machine::create_factory())
            .unwrap_or_else(|e| panic!("Failed to register Machine hook: {}", e));
    }

    #[cfg(feature = "hook-slack")]
    {
        builder = builder
            .with_factory(capsula_notify_slack::create_post_run_hook_factory())
            .unwrap_or_else(|e| panic!("Failed to register capture-slack hook: {}", e));
    }

    builder.build()
}
