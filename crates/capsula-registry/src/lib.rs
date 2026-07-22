use capsula_core::error::{CapsulaError, CapsulaResult};
use capsula_core::hook::{HookErased, PhaseMarker, PostRun, PreRun};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use tracing::debug;

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
    fn new() -> Self {
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
    fn register<H>(&mut self) -> Result<(), RegistryError>
    where
        H: capsula_core::hook::Hook<P> + 'static,
    {
        let id = H::ID;
        if self.creators.contains_key(id) {
            return Err(RegistryError::AlreadyRegistered(id.to_string()));
        }

        debug!("Registering hook type: {}", id);

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
        debug!("Looking up hook creator for: {}", hook_id);
        let creator = self.creators.get(hook_id).ok_or_else(|| {
            let available = self.registered_types().join(", ");
            CapsulaError::Configuration {
                message: format!("Unknown hook id '{hook_id}'. Available types: {available}"),
            }
        })?;

        debug!("Creating hook instance for: {}", hook_id);
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
    fn registered_types(&self) -> Vec<&'static str> {
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
    fn new() -> Self {
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
    fn with_hook<H>(mut self) -> Result<Self, RegistryError>
    where
        H: capsula_core::hook::Hook<P> + 'static,
    {
        self.registry.register::<H>()?;
        Ok(self)
    }

    #[must_use]
    fn build(self) -> HookRegistry<P> {
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

/// Create a standard registry with all built-in hook types.
///
/// Each built-in hook type implements both `Hook<PreRun>` and `Hook<PostRun>`,
/// so a single generic builder covers both phases and the pre/post fns below
/// simply delegate.
fn standard_hook_registry<P: PhaseMarker>() -> Result<HookRegistry<P>, RegistryError>
where
    capsula_capture_cwd::CwdHook: capsula_core::hook::Hook<P>,
    capsula_capture_git_repo::GitHook: capsula_core::hook::Hook<P>,
    capsula_capture_file::FileHook: capsula_core::hook::Hook<P>,
    capsula_capture_env::EnvVarHook: capsula_core::hook::Hook<P>,
    capsula_capture_command::CommandHook: capsula_core::hook::Hook<P>,
    capsula_capture_machine::MachineHook: capsula_core::hook::Hook<P>,
    capsula_capture_json::JsonHook: capsula_core::hook::Hook<P>,
    capsula_capture_toml::TomlHook: capsula_core::hook::Hook<P>,
    capsula_capture_yaml::YamlHook: capsula_core::hook::Hook<P>,
    capsula_notify_slack::SlackNotifyHook: capsula_core::hook::Hook<P>,
{
    Ok(RegistryBuilder::new()
        .with_hook::<capsula_capture_cwd::CwdHook>()?
        .with_hook::<capsula_capture_git_repo::GitHook>()?
        .with_hook::<capsula_capture_file::FileHook>()?
        .with_hook::<capsula_capture_env::EnvVarHook>()?
        .with_hook::<capsula_capture_command::CommandHook>()?
        .with_hook::<capsula_capture_machine::MachineHook>()?
        .with_hook::<capsula_capture_json::JsonHook>()?
        .with_hook::<capsula_capture_toml::TomlHook>()?
        .with_hook::<capsula_capture_yaml::YamlHook>()?
        .with_hook::<capsula_notify_slack::SlackNotifyHook>()?
        .build())
}

/// Create a standard registry with all built-in hook types for pre-run phase
pub fn standard_pre_run_hook_registry() -> Result<HookRegistry<PreRun>, RegistryError> {
    standard_hook_registry::<PreRun>()
}

/// Create a standard registry with all built-in hook types for post-run phase
pub fn standard_post_run_hook_registry() -> Result<HookRegistry<PostRun>, RegistryError> {
    standard_hook_registry::<PostRun>()
}
