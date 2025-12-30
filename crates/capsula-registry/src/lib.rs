use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{HookErased, PhaseMarker, PostRun, PreRun};
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
            RegistryError::HookTypeNotFound(ty) => Self::Configuration {
                message: format!(
                    "Unknown hook id '{ty}'. Check your configuration file for typos.",
                ),
            },
            RegistryError::AlreadyRegistered(ty) => Self::Configuration {
                message: format!("Hook id '{ty}' is already registered"),
            },
            RegistryError::HookCreationFailed { hook, message } => Self::Configuration {
                message: format!("Failed to create '{hook}' hook: {message}"),
            },
        }
    }
}

/// Type alias for a hook creator function
type HookCreator<P> = fn(&Value, &Path) -> CapsulaResult<Box<dyn HookErased<P>>>;

/// Hook registry that stores hook creators
pub struct HookRegistry<P: PhaseMarker> {
    creators: HashMap<&'static str, HookCreator<P>>,
}

impl<P: PhaseMarker> HookRegistry<P> {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            creators: HashMap::new(),
        }
    }

    /// Register a hook type by providing its type parameter
    ///
    /// This method uses the Hook trait's associated constant ID and `from_config` method
    /// to register the hook type in the registry.
    ///
    /// # Example
    /// ```ignore
    /// let mut registry = HookRegistry::<PreRun>::new();
    /// registry.register::<CwdHook>()?;
    /// ```
    pub fn register<H>(&mut self) -> Result<(), RegistryError>
    where
        H: capsula_core::hook::Hook<P> + 'static,
    {
        let id = H::ID;
        if self.creators.contains_key(id) {
            return Err(RegistryError::AlreadyRegistered(id.to_string()));
        }

        // Create a function pointer that calls H::from_config and boxes the result
        let creator: HookCreator<P> = |config, project_root| {
            let hook = H::from_config(config, project_root)?;
            Ok(Box::new(hook))
        };

        self.creators.insert(id, creator);
        Ok(())
    }

    /// Create a hook from type name and configuration
    pub fn create_hook(
        &self,
        hook_id: &str,
        config: &Value,
        project_root: &Path,
    ) -> CapsulaResult<Box<dyn HookErased<P>>> {
        let creator = self.creators.get(hook_id).ok_or_else(|| {
            let available = self.registered_types().join(", ");
            CapsulaError::Configuration {
                message: format!("Unknown hook id '{hook_id}'. Available types: {available}",),
            }
        })?;

        creator(config, project_root).map_err(|e| {
            // Enhance error message with hook type information
            match e {
                CapsulaError::HookFailed { .. } => e,
                _ => CapsulaError::HookFailed {
                    hook: hook_id.to_string(),
                    source: Box::new(e),
                },
            }
        })
    }

    /// Get list of registered hook types
    #[must_use]
    pub fn registered_types(&self) -> Vec<&'static str> {
        self.creators.keys().copied().collect()
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: HookRegistry::new(),
        }
    }

    /// Register a hook type in the registry
    ///
    /// # Example
    /// ```ignore
    /// RegistryBuilder::<PreRun>::new()
    ///     .with_hook::<CwdHook>()?
    ///     .build()
    /// ```
    pub fn with_hook<H>(mut self) -> Result<Self, RegistryError>
    where
        H: capsula_core::hook::Hook<P> + 'static,
    {
        self.registry.register::<H>()?;
        Ok(self)
    }

    #[must_use]
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

/// Create a standard registry with all built-in hook types for pre-run phase
#[must_use]
pub fn standard_pre_run_hook_registry() -> HookRegistry<PreRun> {
    RegistryBuilder::new()
        .with_hook::<capsula_capture_cwd::CwdHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-cwd hook: {e}"))
        .with_hook::<capsula_capture_git_repo::GitHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-git-repo hook: {e}"))
        .with_hook::<capsula_capture_file::FileHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-file hook: {e}"))
        .with_hook::<capsula_capture_env::EnvVarHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-env hook: {e}"))
        .with_hook::<capsula_capture_command::CommandHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-command hook: {e}"))
        .with_hook::<capsula_capture_machine::MachineHook>()
        .unwrap_or_else(|e| panic!("Failed to register Machine hook: {e}"))
        .with_hook::<capsula_notify_slack::SlackNotifyHook>()
        .unwrap_or_else(|e| panic!("Failed to register notify-slack hook: {e}"))
        .build()
}

/// Create a standard registry with all built-in hook types for post-run phase
#[must_use]
pub fn standard_post_run_hook_registry() -> HookRegistry<PostRun> {
    RegistryBuilder::new()
        .with_hook::<capsula_capture_cwd::CwdHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-cwd hook: {e}"))
        .with_hook::<capsula_capture_git_repo::GitHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-git-repo hook: {e}"))
        .with_hook::<capsula_capture_file::FileHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-file hook: {e}"))
        .with_hook::<capsula_capture_env::EnvVarHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-env hook: {e}"))
        .with_hook::<capsula_capture_command::CommandHook>()
        .unwrap_or_else(|e| panic!("Failed to register capture-command hook: {e}"))
        .with_hook::<capsula_capture_machine::MachineHook>()
        .unwrap_or_else(|e| panic!("Failed to register Machine hook: {e}"))
        .with_hook::<capsula_notify_slack::SlackNotifyHook>()
        .unwrap_or_else(|e| panic!("Failed to register notify-slack hook: {e}"))
        .build()
}
