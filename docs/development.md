# Development Guide

This guide covers setting up a development environment, building Capsula, running tests, and adding new hooks.

## Prerequisites

- Rust 1.70 or later
- Git
- (Optional) `just` command runner for convenience

## Setup

### Clone the Repository

```bash
git clone https://github.com/ut-issl/capsula.git
cd capsula
```

### Install Development Tools

```bash
# Install just (optional but recommended)
cargo install just

# Install clippy and rustfmt (usually included with Rust)
rustup component add clippy rustfmt
```

## Building

### Build the Workspace

```bash
# Build all crates
cargo build --workspace

# Build in release mode
cargo build --workspace --release
```

### Build Specific Crate

```bash
# Build only the CLI
cargo build -p capsula-cli

# Build a specific hook
cargo build -p capsula-capture-git-repo
```

### Install CLI Locally

```bash
cargo install --path crates/capsula-cli --locked
```

## Testing

### Run All Tests

```bash
# Run all tests in workspace
cargo test --workspace

# Run tests with output
cargo test --workspace -- --nocapture
```

### Run Tests for Specific Crate

```bash
# Test core library
cargo test -p capsula-core

# Test specific hook
cargo test -p capsula-capture-file
```

### Run Specific Test

```bash
cargo test test_git_hook_clean_repo
```

## Linting

### Using Just

```bash
just lint
```

### Manual Linting

```bash
# Run clippy with all features
cargo clippy --workspace --all-targets --all-features

# Run clippy with no default features
cargo clippy --workspace --all-targets --no-default-features

# Check formatting
cargo fmt --check --all

# Generate documentation
cargo doc --workspace --no-deps

# Run cargo check
cargo check --workspace
```

## Running the CLI

### Run Without Installing

```bash
# Run a command
cargo run -p capsula-cli -- run echo "Hello, Capsula!"

# List runs
cargo run -p capsula-cli -- list

# Use custom config
cargo run -p capsula-cli -- --config path/to/config.toml run python script.py
```

### Run Installed Version

```bash
capsula run echo "Hello, Capsula!"
```

## Adding a New Hook

Follow these steps to add a new hook type.

### 1. Create a New Crate

```bash
# Create the crate directory
mkdir -p crates/capsula-capture-myfeature
cd crates/capsula-capture-myfeature

# Initialize with Cargo
cargo init --lib
```

### 2. Update Cargo.toml

Edit `crates/capsula-capture-myfeature/Cargo.toml`:

```toml
[package]
name = "capsula-capture-myfeature"
version = "0.1.0"
edition = "2021"

[dependencies]
capsula-core = { path = "../capsula-core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### 3. Implement the Hook

Create `crates/capsula-capture-myfeature/src/lib.rs`:

```rust
use capsula_core::{Captured, CapsulaError, Hook, Metadata, PhaseMarker, RunParams};
use serde::{Deserialize, Serialize};
use std::path::Path;

// Configuration struct
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MyFeatureConfig {
    pub some_option: Option<String>,
}

// Output struct
#[derive(Debug, Serialize)]
pub struct MyFeatureCaptured {
    pub result: String,
}

// Hook struct
pub struct MyFeatureHook {
    config: MyFeatureConfig,
}

// Implement Hook trait for both PreRun and PostRun
impl<P: PhaseMarker> Hook<P> for MyFeatureHook {
    const ID: &'static str = "capture_myfeature";
    type Config = MyFeatureConfig;
    type Output = MyFeatureCaptured;

    fn from_config(
        config: Self::Config,
        _project_root: &Path,
    ) -> Result<Self, CapsulaError> {
        Ok(Self { config })
    }

    fn run(
        &self,
        _metadata: &Metadata,
        _params: &RunParams<P>,
    ) -> Result<Self::Output, CapsulaError> {
        // Implement your hook logic here
        let result = format!(
            "Captured with option: {}",
            self.config.some_option.as_deref().unwrap_or("default")
        );

        Ok(MyFeatureCaptured { result })
    }
}

// Implement Captured trait
impl Captured for MyFeatureCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, CapsulaError> {
        serde_json::to_value(self).map_err(|e| {
            CapsulaError::SerializationError(format!("Failed to serialize: {}", e))
        })
    }

    // Optional: Implement abort logic
    fn abort_requested(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capsula_core::{PreRun, PostRun};

    #[test]
    fn test_myfeature_hook() {
        let config = MyFeatureConfig {
            some_option: Some("test".to_string()),
        };

        let hook = MyFeatureHook::from_config(config, Path::new(".")).unwrap();

        // Create dummy metadata and params for testing
        // (You'll need to construct these based on your needs)

        // let metadata = Metadata { /* ... */ };
        // let params = RunParams::<PreRun> { /* ... */ };
        // let result = hook.run(&metadata, &params).unwrap();

        // assert_eq!(result.result, "Captured with option: test");
    }
}
```

### 4. Add to Workspace

Edit the root `Cargo.toml` to add your crate to the workspace:

```toml
[workspace]
members = [
    "crates/capsula-core",
    "crates/capsula-registry",
    "crates/capsula-config",
    "crates/capsula-cli",
    "crates/capsula-capture-cwd",
    "crates/capsula-capture-env",
    "crates/capsula-capture-git-repo",
    "crates/capsula-capture-file",
    "crates/capsula-capture-machine",
    "crates/capsula-capture-command",
    "crates/capsula-notify-slack",
    "crates/capsula-capture-myfeature",  # Add your crate here
]
```

### 5. Register the Hook

Edit `crates/capsula-registry/Cargo.toml` to add dependency:

```toml
[dependencies]
capsula-core = { path = "../capsula-core" }
capsula-capture-cwd = { path = "../capsula-capture-cwd" }
capsula-capture-env = { path = "../capsula-capture-env" }
# ... other hooks
capsula-capture-myfeature = { path = "../capsula-capture-myfeature" }
```

Edit `crates/capsula-registry/src/lib.rs` to register your hook:

```rust
use capsula_capture_myfeature::MyFeatureHook;

pub fn standard_pre_run_hook_registry() -> HookRegistry<PreRun> {
    HookRegistry::new()
        .with_hook::<CwdHook>()
        .with_hook::<EnvHook>()
        .with_hook::<GitHook>()
        .with_hook::<FileHook>()
        .with_hook::<MachineHook>()
        .with_hook::<CommandHook>()
        .with_hook::<MyFeatureHook>()  // Add here for pre-run
}

pub fn standard_post_run_hook_registry() -> HookRegistry<PostRun> {
    HookRegistry::new()
        .with_hook::<CwdHook>()
        .with_hook::<EnvHook>()
        .with_hook::<GitHook>()
        .with_hook::<FileHook>()
        .with_hook::<MachineHook>()
        .with_hook::<CommandHook>()
        .with_hook::<SlackHook>()
        .with_hook::<MyFeatureHook>()  // Add here for post-run
}
```

### 6. Test Your Hook

Create a test configuration `test-config.toml`:

```toml
[vault]
name = "test-myfeature"

[[pre_run]]
type = "capture_myfeature"
some_option = "hello"
```

Run a test command:

```bash
cargo run -p capsula-cli -- --config test-config.toml run echo "Testing my hook"
```

Check the output:

```bash
cat .capsula/test-myfeature/*/latest/_capsula/pre-run.json
```

### 7. Write Tests

Add comprehensive tests in your hook crate:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use capsula_core::{PreRun, Metadata, RunParams};
    use std::path::PathBuf;

    #[test]
    fn test_from_config() {
        let config = MyFeatureConfig {
            some_option: Some("test".to_string()),
        };

        let hook = MyFeatureHook::from_config(config, Path::new("."));
        assert!(hook.is_ok());
    }

    #[test]
    fn test_hook_run() {
        let config = MyFeatureConfig {
            some_option: Some("test".to_string()),
        };

        let hook = MyFeatureHook::from_config(config, Path::new(".")).unwrap();

        // Create test metadata and params
        // Run the hook and verify output
    }

    #[test]
    fn test_serialization() {
        let captured = MyFeatureCaptured {
            result: "test result".to_string(),
        };

        let json = captured.serialize_json();
        assert!(json.is_ok());
    }
}
```

Run your tests:

```bash
cargo test -p capsula-capture-myfeature
```

## Hook Implementation Guidelines

### Best Practices

1. **Keep hooks focused**: Each hook should do one thing well
2. **Handle errors gracefully**: Return `CapsulaError` for failures
3. **Make configs optional**: Use `Option<T>` for optional settings
4. **Document behavior**: Add doc comments to config fields
5. **Test thoroughly**: Write unit tests and integration tests
6. **Minimize dependencies**: Only add necessary dependencies

### Configuration Design

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]  // Catch typos in config
pub struct MyConfig {
    /// Required field with documentation
    pub required_field: String,

    /// Optional field with default
    #[serde(default)]
    pub optional_field: Option<String>,

    /// Field with custom default
    #[serde(default = "default_value")]
    pub with_default: String,
}

fn default_value() -> String {
    "default".to_string()
}
```

### Error Handling

```rust
use capsula_core::CapsulaError;

fn some_operation() -> Result<String, CapsulaError> {
    // Convert errors to CapsulaError
    std::fs::read_to_string("file.txt")
        .map_err(|e| CapsulaError::IoError(e))?;

    // Or use custom errors
    Err(CapsulaError::HookError {
        hook_id: "my_hook",
        message: "Something went wrong".to_string(),
    })
}
```

### Implementing Abort Logic

```rust
impl Captured for MyFeatureCaptured {
    fn abort_requested(&self) -> bool {
        // Return true to abort the run
        self.some_condition_failed
    }
}
```

## Project Conventions

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use `clippy` recommendations (`cargo clippy`)
- Write doc comments for public APIs
- Keep functions focused and testable

### Naming Conventions

- Hook crates: `capsula-{action}-{target}` (e.g., `capsula-capture-env`)
- Hook IDs: `{action}_{target}` (e.g., `capture_env`)
- Config structs: `{Target}HookConfig` (e.g., `EnvHookConfig`)
- Output structs: `{Target}Captured` (e.g., `EnvCaptured`)
- Hook structs: `{Target}Hook` (e.g., `EnvHook`)

### Documentation

- Add doc comments to all public items
- Include examples in doc comments
- Update README.md when adding features
- Add documentation pages for new hooks

## Continuous Integration

The project uses GitHub Actions for CI:

- Run tests on Linux, macOS, and Windows
- Check formatting with `cargo fmt`
- Run `cargo clippy`
- Build documentation
- Check for security vulnerabilities

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-new-hook`
3. Make your changes
4. Run tests: `cargo test --workspace`
5. Run lints: `just lint` or manual commands
6. Commit your changes: `git commit -am 'Add new hook'`
7. Push to the branch: `git push origin feature/my-new-hook`
8. Create a Pull Request

## Debugging Tips

### Enable Debug Logging

```bash
RUST_LOG=debug capsula run python script.py
```

### Print Hook Output

```bash
capsula run python script.py
cat .capsula/vault/*/latest/_capsula/pre-run.json | jq .
```

### Test Single Hook

Create minimal config with only your hook:

```toml
[vault]
name = "debug"

[[pre_run]]
type = "my_hook"
```

### Use `dbg!` Macro

In your hook implementation:

```rust
fn run(&self, metadata: &Metadata, params: &RunParams<P>)
    -> Result<Self::Output, CapsulaError> {
    dbg!(&self.config);
    let result = do_something();
    dbg!(&result);
    Ok(result)
}
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Serde Documentation](https://serde.rs/)
- [Capsula Architecture](architecture.md)
- [Configuration Guide](configuration.md)
- [Hooks Reference](hooks.md)

## Getting Help

- Open an issue on GitHub
- Check existing issues and PRs
- Review example hook implementations in `crates/`

## Next Steps

- [Architecture](architecture.md) - Understand the system design
- [Configuration](configuration.md) - Learn configuration format
- [Hooks Reference](hooks.md) - See existing hook implementations
