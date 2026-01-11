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
                message: format!("Unknown hook id '{hook_id}'. Available types: {available}",),
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

/// Schema information for a hook type
#[derive(Debug, Clone, serde::Serialize)]
pub struct HookSchema {
    pub id: &'static str,
    pub config_schema: schemars::Schema,
}

/// Generate JSON schemas for all registered hook types
///
/// Returns a vector of hook schemas, each containing the hook ID and its configuration schema.
/// The schemas describe the configuration structure for each hook type.
///
/// # Example
/// ```ignore
/// let schemas = generate_hook_schemas();
/// for schema in schemas {
///     println!("Hook '{}' schema:", schema.id);
///     println!("{}", serde_json::to_string_pretty(&schema.config_schema).unwrap());
/// }
/// ```
#[must_use]
pub fn generate_hook_schemas() -> Vec<HookSchema> {
    use capsula_core::hook::Hook;

    vec![
        HookSchema {
            id: <capsula_capture_cwd::CwdHook as Hook<PreRun>>::ID,
            config_schema: schemars::schema_for!(capsula_capture_cwd::CwdHookConfig),
        },
        HookSchema {
            id: <capsula_capture_git_repo::GitHook as Hook<PreRun>>::ID,
            config_schema: schemars::schema_for!(capsula_capture_git_repo::GitHookConfig),
        },
        HookSchema {
            id: <capsula_capture_file::FileHook as Hook<PreRun>>::ID,
            config_schema: schemars::schema_for!(capsula_capture_file::FileHookConfig),
        },
        HookSchema {
            id: <capsula_capture_env::EnvVarHook as Hook<PreRun>>::ID,
            config_schema: schemars::schema_for!(capsula_capture_env::EnvVarHookConfig),
        },
        HookSchema {
            id: <capsula_capture_command::CommandHook as Hook<PreRun>>::ID,
            config_schema: schemars::schema_for!(capsula_capture_command::CommandHookConfig),
        },
        HookSchema {
            id: <capsula_capture_machine::MachineHook as Hook<PreRun>>::ID,
            config_schema: schemars::schema_for!(capsula_capture_machine::MachineHookConfig),
        },
        HookSchema {
            id: <capsula_notify_slack::SlackNotifyHook as Hook<PreRun>>::ID,
            config_schema: schemars::schema_for!(capsula_notify_slack::SlackNotifyHookConfig),
        },
    ]
}

/// Generate a complete JSON schema for the entire capsula.toml configuration file
///
/// This schema validates the full structure including vault, dotenv, server,
/// and both pre-run and post-run hook arrays.
///
/// # Returns
/// A JSON Schema value that can be used to validate capsula.toml files
#[expect(
    clippy::too_many_lines,
    reason = "Schema generation is inherently verbose"
)]
#[must_use]
pub fn generate_full_config_schema() -> serde_json::Value {
    let hook_schemas = generate_hook_schemas();

    // Collect all $defs from hook schemas to move them to root level
    let mut root_defs = serde_json::Map::new();

    // Build oneOf array for hook validation based on ID
    let hook_one_of: Vec<serde_json::Value> = hook_schemas
        .iter()
        .map(|hook| {
            let mut hook_schema =
                serde_json::to_value(&hook.config_schema).unwrap_or_else(|_| serde_json::json!({}));

            // Extract $defs from this hook schema and add to root_defs with prefixed names
            if let Some(defs) = hook_schema.get("$defs").and_then(|d| d.as_object()) {
                for (def_name, def_value) in defs {
                    let prefixed_name = format!("{}_{}", hook.id.replace('-', "_"), def_name);
                    root_defs.insert(prefixed_name.clone(), def_value.clone());
                }
                hook_schema.as_object_mut().unwrap().remove("$defs");
            }

            // Update $ref paths in properties to point to root level
            if let Some(props) = hook_schema
                .get_mut("properties")
                .and_then(|p| p.as_object_mut())
            {
                for (_, prop_value) in props.iter_mut() {
                    if let Some(ref_path) = prop_value.get("$ref").and_then(|r| r.as_str()) {
                        if ref_path.starts_with("#/$defs/") {
                            let def_name = ref_path.strip_prefix("#/$defs/").unwrap();
                            let new_ref =
                                format!("#/$defs/{}_{}", hook.id.replace('-', "_"), def_name);
                            prop_value
                                .as_object_mut()
                                .unwrap()
                                .insert("$ref".to_string(), serde_json::json!(new_ref));
                        }
                    }
                }
            }

            // Remove the nested $schema field (not needed in oneOf)
            hook_schema.as_object_mut().unwrap().remove("$schema");

            // Add the 'id' field requirement to the schema
            if let Some(props) = hook_schema
                .get_mut("properties")
                .and_then(|p| p.as_object_mut())
            {
                props.insert(
                    "id".to_string(),
                    serde_json::json!({
                        "const": hook.id,
                        "type": "string"
                    }),
                );
            } else {
                // If no properties exist, create them
                hook_schema["properties"] = serde_json::json!({
                    "id": {
                        "const": hook.id,
                        "type": "string"
                    }
                });
            }

            // Ensure 'id' is in required array
            if let Some(required) = hook_schema
                .get_mut("required")
                .and_then(|r| r.as_array_mut())
            {
                if !required.iter().any(|v| v == "id") {
                    required.insert(0, serde_json::json!("id"));
                }
            } else {
                hook_schema["required"] = serde_json::json!(["id"]);
            }

            hook_schema
        })
        .collect();

    let mut schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Capsula Configuration",
        "description": "Configuration file schema for Capsula - a tool for capturing and preserving command execution context",
        "type": "object",
        "required": ["vault"],
        "properties": {
            "vault": {
                "type": "object",
                "description": "Vault configuration for storing captured runs",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the vault (should be unique in the storage)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to the vault directory (defaults to .capsula/{name})"
                    }
                }
            },
            "dotenv": {
                "type": "string",
                "description": "Path to .env file to load environment variables from"
            },
            "server": {
                "type": "string",
                "description": "URL of the Capsula server for pushing runs (e.g., http://localhost:8500)",
                "format": "uri"
            },
            "pre-run": {
                "type": "object",
                "description": "Hooks to run before command execution",
                "properties": {
                    "hooks": {
                        "type": "array",
                        "description": "Array of hooks to execute before the command",
                        "items": {
                            "oneOf": hook_one_of.clone()
                        }
                    }
                },
                "additionalProperties": false
            },
            "post-run": {
                "type": "object",
                "description": "Hooks to run after command execution",
                "properties": {
                    "hooks": {
                        "type": "array",
                        "description": "Array of hooks to execute after the command",
                        "items": {
                            "oneOf": hook_one_of
                        }
                    }
                },
                "additionalProperties": false
            }
        },
        "additionalProperties": false
    });

    // Add $defs if we have any
    if !root_defs.is_empty() {
        schema
            .as_object_mut()
            .unwrap()
            .insert("$defs".to_string(), serde_json::Value::Object(root_defs));
    }

    schema
}
