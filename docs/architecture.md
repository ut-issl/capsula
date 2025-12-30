# Architecture

Capsula is built with a modular architecture designed for extensibility and type safety. This document explains the core design principles and implementation details.

## Project Structure

Capsula is organized as a Rust workspace with 11 crates in a 3-tier hierarchy:

```
capsula/
├── crates/
│   ├── capsula-core/           # Tier 1: Core traits and types
│   ├── capsula-registry/       # Tier 2: Hook registry
│   ├── capsula-config/         # Tier 2: Configuration parsing
│   ├── capsula-cli/            # Tier 3: CLI interface
│   ├── capsula-capture-cwd/    # Tier 3: Hook implementation
│   ├── capsula-capture-env/    # Tier 3: Hook implementation
│   ├── capsula-capture-git-repo/  # Tier 3: Hook implementation
│   ├── capsula-capture-file/   # Tier 3: Hook implementation
│   ├── capsula-capture-machine/   # Tier 3: Hook implementation
│   ├── capsula-capture-command/   # Tier 3: Hook implementation
│   └── capsula-notify-slack/   # Tier 3: Hook implementation
└── Cargo.toml
```

### Tier 1: Core Infrastructure

**capsula-core** - Foundation traits and types:

- `Hook<P>` trait - Generic, type-safe hook interface
- `HookErased<P>` trait - Object-safe trait for heterogeneous collections
- `Captured` trait - Output contract for hook results
- `Run<Dir>` struct - Type-state pattern for run lifecycle
- `CapsulaError` - Error types

### Tier 2: System Support

**capsula-registry** - Hook type registry:

- Maps hook IDs to factory functions
- Enables dynamic hook creation from configuration
- Provides standard hook registries

**capsula-config** - Configuration parsing:

- TOML deserialization
- Strongly-typed configuration structs
- JSON interchange format for hook configs

### Tier 3: CLI and Implementations

**capsula-cli** - Command-line interface:

- Argument parsing
- Orchestration of hook execution
- Output formatting

**Hook crates** - Individual hook implementations:

- Each hook is a separate crate
- Implements `Hook<P>` trait
- Self-contained with minimal dependencies

## Core Trait System

### The `Hook<P>` Trait

Generic, type-safe interface for all hooks:

```rust
pub trait Hook<P: PhaseMarker>: Send + Sync {
    const ID: &'static str;
    type Config: for<'de> Deserialize<'de> + Serialize + Send + Sync;
    type Output: Captured;

    fn from_config(
        config: Self::Config,
        project_root: &Path,
    ) -> Result<Self, CapsulaError>
    where
        Self: Sized;

    fn run(
        &self,
        metadata: &Metadata,
        params: &RunParams<P>,
    ) -> Result<Self::Output, CapsulaError>;
}
```

**Key features**:

- `P: PhaseMarker` distinguishes `PreRun` vs `PostRun` at compile time
- Each hook has unique `ID`, `Config`, and `Output` types
- Factory pattern via `from_config`
- Execution via `run` method

### The `HookErased<P>` Trait

Object-safe trait for heterogeneous hook collections:

```rust
pub trait HookErased<P: PhaseMarker>: Send + Sync {
    fn run_erased(
        &self,
        metadata: &Metadata,
        params: &RunParams<P>,
    ) -> Result<Box<dyn Captured>, CapsulaError>;

    fn id(&self) -> &'static str;
}
```

**Purpose**: Enables storing different hook types in a single vector:

```rust
Vec<Box<dyn HookErased<PreRun>>>
```

**Blanket implementation**: All `Hook<P>` automatically implement `HookErased<P>`.

### The `Captured` Trait

Output contract for all hook results:

```rust
pub trait Captured: Send + Sync {
    fn serialize_json(&self) -> Result<serde_json::Value, CapsulaError>;

    fn abort_requested(&self) -> bool {
        false
    }
}
```

**Features**:

- Must be JSON-serializable
- Can optionally request run abortion
- Automatically includes `__meta` field in output

## Design Patterns

### Factory + Registry Pattern

The registry stores function pointers for dynamic hook creation:

```rust
pub struct HookRegistry<P: PhaseMarker> {
    hooks: HashMap<
        &'static str,
        fn(serde_json::Value, &Path) -> Result<Box<dyn HookErased<P>>, CapsulaError>
    >,
}
```

**Flow**:

1. Configuration contains hook ID and JSON config
2. Registry lookup by ID returns factory function
3. Factory deserializes JSON to typed `Config`
4. Factory creates `Box<dyn HookErased<P>>`

**Benefits**:

- No compile-time dependency between registry and hook implementations
- Easy to add new hooks
- Type-safe deserialization per hook

### Type State Pattern

The `Run<Dir>` struct uses phantom types to enforce setup ordering:

```rust
pub struct Run<Dir> {
    id: Ulid,
    name: String,
    directory: Dir,  // Type state
    // ...
}
```

**States**:

- `Run<()>`: Unprepared, directory not created (cannot execute)
- `Run<PathBuf>`: Prepared, directory exists (can execute)

**Transition**:

```rust
impl Run<()> {
    pub fn setup_run_dir(self, vault: &VaultConfig)
        -> Result<Run<PathBuf>, CapsulaError>
}
```

**Benefit**: Compile-time guarantee that directory exists before execution.

### Phase-Based Execution

Hooks organized into two phases using phantom types:

```rust
pub struct PreRun;
pub struct PostRun;

pub trait PhaseMarker: Send + Sync + 'static {}
impl PhaseMarker for PreRun {}
impl PhaseMarker for PostRun {}
```

**Benefits**:

- Same trait for both phases
- Type-safe distinction at compile time
- Hooks can implement both phases

## Configuration Pipeline

```
capsula.toml
    ↓ Parse TOML (capsula-config)
CapsulaConfig {
    vault: VaultConfig,
    pre_run: Vec<HookEnvelope>,
    post_run: Vec<HookEnvelope>,
}
    ↓ For each HookEnvelope
registry.create_hook(id, config_json, project_root)
    ↓ Lookup factory by ID
Hook::from_config() → deserialize JSON to typed Config
    ↓ Create instance
Box<dyn HookErased<P>>
    ↓ Collect all hooks
Vec<Box<dyn HookErased<P>>>
    ↓ Execute each
hook.run(metadata, params) → Box<dyn Captured>
    ↓ Serialize
JSON output with __meta field
```

### Why JSON as Interchange?

Hook configs are stored as `serde_json::Value` (dynamic JSON) rather than statically typed.

**Rationale**:

- Different config types per hook
- No recompilation when adding hooks
- Type-safe deserialization happens in hook crate
- Registry doesn't need to know config types

## Execution Flow

1. **CLI parses arguments** (`capsula-cli`)
   - Load `capsula.toml`
   - Parse command to execute

2. **Create registries** (`capsula-registry`)
   - `standard_pre_run_hook_registry()`
   - `standard_post_run_hook_registry()`

3. **Build Run** (`capsula-core`)
   - Generate ULID
   - Generate random name (e.g., "chubby-back")
   - Create `Run<()>`

4. **Setup run directory** (`capsula-cli`)
   - `Run<()>` → `Run<PathBuf>`
   - Create `.capsula/{vault}/{date}/{time-name}/_capsula/`
   - Write `metadata.json`

5. **Pre-run phase**
   - Build hooks from config
   - Execute each hook in order
   - Serialize to `pre-run.json`
   - Check `abort_requested()`
   - Abort if any hook requests it

6. **Command execution**
   - Spawn child process
   - Set environment variables
   - Capture stdout/stderr in parallel threads
   - Wait for completion
   - Write `command.json`

7. **Post-run phase**
   - Build and execute post-run hooks
   - Serialize to `post-run.json`

## Output Structure

```
.capsula/{vault-name}/{YYYY-MM-DD}/{HHMMSS-name}/
├── _capsula/
│   ├── metadata.json      # Run metadata
│   ├── pre-run.json       # Pre-run hook outputs (array)
│   ├── command.json       # Command execution results
│   └── post-run.json      # Post-run hook outputs (array)
└── [captured files]       # Files copied by file hooks
```

### JSON Structure

Each hook's output includes `__meta` field:

```json
{
  "__meta": {
    "id": "capture_git_repo",
    "config": { "allow_dirty": true },
    "success": true
  },
  "commit": "a1b2c3d4...",
  "branch": "main"
}
```

Failed hooks:

```json
{
  "__meta": {
    "id": "capture_file",
    "config": { "path": "missing.txt" },
    "success": false,
    "error": "File not found: missing.txt"
  }
}
```

## Error Handling

### Non-Fatal Errors

Each hook's error is caught, logged, and stored:

```rust
match hook.run_erased(metadata, params) {
    Ok(captured) => outputs.push(captured),
    Err(e) => {
        warn!("Hook {} failed: {}", hook.id(), e);
        outputs.push(error_captured(hook.id(), e));
    }
}
```

**Rationale**: Partial success is valuable for debugging.

### Fatal Errors

**Abort execution**:

- Configuration parse errors
- Run directory creation failures
- Command execution failures
- Hook requests abort via `abort_requested()`

**Example**: Git hook with `allow_dirty = false`:

```rust
impl Captured for GitCaptured {
    fn abort_requested(&self) -> bool {
        !self.allow_dirty && self.dirty
    }
}
```

## Hook Implementation Pattern

Each hook follows this structure:

### 1. Config Struct

```rust
#[derive(Deserialize, Serialize)]
pub struct GitHookConfig {
    pub allow_dirty: Option<bool>,
}
```

### 2. Captured Output Struct

```rust
#[derive(Serialize)]
pub struct GitCaptured {
    pub commit: String,
    pub branch: String,
    pub dirty: bool,
    #[serde(skip)]
    allow_dirty: bool,
}
```

### 3. Hook Struct

```rust
pub struct GitHook {
    config: GitHookConfig,
}

impl<P: PhaseMarker> Hook<P> for GitHook {
    const ID: &'static str = "capture_git_repo";
    type Config = GitHookConfig;
    type Output = GitCaptured;

    fn from_config(config: Self::Config, _: &Path)
        -> Result<Self, CapsulaError> {
        Ok(Self { config })
    }

    fn run(&self, _: &Metadata, _: &RunParams<P>)
        -> Result<Self::Output, CapsulaError> {
        // Implementation
    }
}
```

### 4. Captured Implementation

```rust
impl Captured for GitCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, CapsulaError> {
        serde_json::to_value(self).map_err(Into::into)
    }

    fn abort_requested(&self) -> bool {
        !self.allow_dirty && self.dirty
    }
}
```

## Adding a New Hook

1. **Create crate**: `capsula-{hook-id}`
2. **Implement traits**: `Hook<P>` and `Captured`
3. **Add dependency**: In `capsula-registry/Cargo.toml`
4. **Register**: In `capsula-registry/src/lib.rs`:

```rust
pub fn standard_pre_run_hook_registry() -> HookRegistry<PreRun> {
    HookRegistry::new()
        .with_hook::<CwdHook>()
        .with_hook::<GitHook>()
        .with_hook::<YourNewHook>()  // Add here
}
```

No changes needed to CLI or config parser.

## Dependency Graph

```
capsula-cli
    ├── capsula-registry
    │   ├── capsula-core
    │   ├── capsula-capture-cwd
    │   │   └── capsula-core
    │   ├── capsula-capture-env
    │   │   └── capsula-core
    │   └── ... (other hook crates)
    ├── capsula-config
    │   └── capsula-core
    └── capsula-core
```

**Key principle**: Hook crates only depend on `capsula-core`, not on each other.

## Performance Considerations

### Hook Execution

- Hooks execute **sequentially** in config order
- No parallelization (by design)
- Long-running hooks block execution

### File Operations

- File copying is synchronous
- Hash computation reads entire file
- Consider impact for large files

### JSON Serialization

- All outputs serialized to JSON
- Minimal overhead for typical data sizes
- Very large outputs (> 10 MB) may impact performance

## Thread Safety

All types are thread-safe:

- `Hook<P>: Send + Sync`
- `Captured: Send + Sync`
- `HookErased<P>: Send + Sync`

This enables future parallelization if needed.

## Next Steps

- [Development Guide](development.md) - Learn how to add hooks
- [Configuration](configuration.md) - Understand configuration system
- [Hooks Reference](hooks.md) - See available hooks
