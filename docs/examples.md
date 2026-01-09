# Examples

This page provides complete, real-world examples of using Capsula for different scenarios.

## Machine Learning Experiments

Track machine learning experiments with full reproducibility.

### Configuration

```toml title="capsula.toml"
dotenv = ".env"  # Load SLACK_BOT_TOKEN

[vault]
name = "ml-experiments"

# Pre-run: Capture environment and validate
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false  # Require clean git state

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"

[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "list"]

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
hash = "sha256"

[[pre-run.hooks]]
id = "notify-slack"
channel = "#ml-training"

# Post-run: Capture results and notify
[[post-run.hooks]]
id = "capture-file"
glob = "results/metrics.json"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "results/plots/*.png"
mode = "copy"

[[post-run.hooks]]
id = "capture-file"
glob = "models/best_model.pkl"
mode = "none"
hash = "sha256"  # Hash only, file is large

[[post-run.hooks]]
id = "notify-slack"
channel = "#ml-training"
attachment_globs = ["results/plots/training_curve.png", "results/metrics.json"]
```

### Usage

```bash
# Set environment
export CUDA_VISIBLE_DEVICES=0,1

# Run training
capsula run python train.py --config config.yaml --epochs 100
```

### What Gets Captured

- ✅ Exact git commit
- ✅ System specs (CPU, RAM, GPU)
- ✅ Python version and installed packages
- ✅ Training configuration file
- ✅ Results and plots
- ✅ Model file hash
- ✅ Slack notifications at start and end

---

## Data Processing Pipeline

Track data processing jobs with input/output file verification.

### Configuration

```toml title="capsula.toml"
[vault]
name = "data-pipeline"

# Pre-run: Verify inputs exist
[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "data/input.csv"]
abort_on_failure = true

[[pre-run.hooks]]
id = "capture-file"
glob = "data/input.csv"
mode = "none"
hash = "sha256"  # Verify input hasn't changed

# Post-run: Capture outputs
[[post-run.hooks]]
id = "capture-file"
glob = "data/output.csv"
mode = "copy"
hash = "sha256"

[[post-run.hooks]]
id = "capture-command"
command = ["wc", "-l", "data/output.csv"]  # Count output rows
```

### Usage

```bash
capsula run python process_data.py --input data/input.csv --output data/output.csv
```

### What Gets Captured

- ✅ Input file hash (for change detection)
- ✅ Output file with hash
- ✅ Output row count
- ✅ Abort if input file missing

---

## Continuous Integration / Builds

Track build artifacts and test results.

### Configuration

```toml title="capsula.toml"
[vault]
name = "ci-builds"

# Pre-run: Capture environment
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false

[[pre-run.hooks]]
id = "capture-command"
command = ["rustc", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["cargo", "--version"]

# Post-run: Capture build artifacts and results
[[post-run.hooks]]
id = "capture-file"
glob = "target/release/my-binary"
mode = "copy"
hash = "sha256"

[[post-run.hooks]]
id = "capture-command"
command = ["ls", "-lh", "target/release/"]

[[post-run.hooks]]
id = "capture-file"
glob = "test-results.xml"
mode = "copy"
```

### Usage

```bash
# Build and test
capsula run bash -c 'cargo build --release && cargo test'
```

### What Gets Captured

- ✅ Git commit (ensures reproducible builds)
- ✅ Rust toolchain version
- ✅ Build binary with hash
- ✅ Build directory listing
- ✅ Test results

---

## Research Paper Experiments

Track experiments for academic papers with full reproducibility.

### Configuration

```toml title="capsula.toml"
[vault]
name = "paper-experiments"

# Pre-run: Strict reproducibility
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false  # Must commit before running

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-env"
name = "OMP_NUM_THREADS"

[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]

[[pre-run.hooks]]
id = "capture-command"
command = ["pip", "freeze"]  # Exact package versions

[[pre-run.hooks]]
id = "capture-file"
glob = "experiments/config_*.json"
mode = "copy"

# Post-run: Archive everything
[[post-run.hooks]]
id = "capture-file"
glob = "results/**/*"
mode = "move"  # Move all results to vault

[[post-run.hooks]]
id = "capture-command"
command = ["python", "scripts/generate_summary.py"]
```

### Usage

```bash
# Run experiment 1
capsula run python run_experiment.py --config experiments/config_1.json

# Run experiment 2
capsula run python run_experiment.py --config experiments/config_2.json

# List all runs
capsula list
```

### What Gets Captured

- ✅ Exact code version (git commit)
- ✅ Hardware specifications
- ✅ Exact Python package versions
- ✅ All experiment configurations
- ✅ All results (moved to vault)
- ✅ Generated summary

---

## Nightly Cron Jobs

Track automated jobs with notifications.

### Configuration

```toml title="capsula.toml"
dotenv = ".env"

[vault]
name = "nightly-jobs"

# Pre-run: Validate preconditions
[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-command"
command = ["df", "-h"]  # Check disk space

[[pre-run.hooks]]
id = "capture-command"
command = ["date"]

# Post-run: Notify and save logs
[[post-run.hooks]]
id = "capture-file"
glob = "*.log"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "#cron-jobs"
attachment_globs = ["summary.log"]
```

### Usage

Add to crontab:

```bash
# Run every night at 2 AM
0 2 * * * cd /path/to/project && capsula run ./nightly_job.sh
```

### What Gets Captured

- ✅ Job execution time
- ✅ Disk space before running
- ✅ All log files
- ✅ Slack notification with summary

---

## Development and Testing

Quick iterations during development.

### Configuration

```toml title="dev.toml"
[vault]
name = "dev-runs"

# Minimal hooks for fast iteration
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true  # Allow uncommitted changes

[[pre-run.hooks]]
id = "capture-cwd"

# Post-run: Quick checks
[[post-run.hooks]]
id = "capture-command"
command = ["ls", "-la", "output/"]
```

### Usage

```bash
# Quick test with dev config
capsula --config dev.toml run python test_feature.py

# Production run with strict config
capsula --config capsula.toml run python run_production.py
```

---

## Multi-Repository Projects

Track projects spanning multiple git repositories.

### Configuration

```toml title="capsula.toml"
[vault]
name = "multi-repo-project"

# Capture state of all repositories
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = false

[[pre-run.hooks]]
id = "capture-git-repo"
path = "../shared-library"
allow_dirty = false

[[pre-run.hooks]]
id = "capture-git-repo"
path = "../data-repo"
allow_dirty = false

[[pre-run.hooks]]
id = "capture-cwd"
```

### What Gets Captured

- ✅ Main project git state
- ✅ Dependency library git state
- ✅ Data repository git state

---

## GPU Compute Jobs

Track GPU usage and configuration.

### Configuration

```toml title="capsula.toml"
[vault]
name = "gpu-jobs"

# Pre-run: Capture GPU info
[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_HOME"

[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi"]

[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi", "--query-gpu=name,memory.total,memory.free", "--format=csv"]

# Post-run: Record final GPU state
[[post-run.hooks]]
id = "capture-command"
command = ["nvidia-smi"]
```

### Usage

```bash
export CUDA_VISIBLE_DEVICES=0,1
capsula run python train_on_gpu.py
```

### What Gets Captured

- ✅ Which GPUs were used
- ✅ GPU specs and memory
- ✅ GPU state before and after

---

## Simple Script Tracking

Minimal configuration for simple scripts.

### Configuration

```toml title="capsula.toml"
[vault]
name = "scripts"

[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true
```

### Usage

```bash
capsula run python my_script.py
```

### What Gets Captured

- ✅ Where script was run
- ✅ Git state
- ✅ Command output

---

## Tips for All Examples

### Use Different Configs

Keep multiple configs for different purposes:

```bash
capsula --config dev.toml run ...      # Development
capsula --config experiment.toml run ... # Experiments
capsula --config ci.toml run ...       # CI/CD
```

### Review Captured Data

After running, check what was captured:

```bash
# List runs
capsula list

# View latest run
ls -la .capsula/vault-name/$(date +%Y-%m-%d)/

# Check captured data
cat .capsula/vault-name/*/latest/_capsula/pre-run.json | jq .
```

### Start Simple

Begin with minimal hooks and add more as needed:

```toml
# Start here
[[pre-run.hooks]]
id = "capture-cwd"

# Add gradually
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."

# Keep expanding
[[pre-run.hooks]]
id = "capture-machine"
```

## What's Next?

<div class="grid cards" markdown>

-   :material-cog:{ .lg .middle } **Configuration**

    ---

    Complete configuration reference.

    [:octicons-arrow-right-24: Configuration guide](configuration.md)

-   :material-hook:{ .lg .middle } **Hooks**

    ---

    Detailed documentation for each hook.

    [:octicons-arrow-right-24: Hook reference](hooks.md)

-   :material-help-circle:{ .lg .middle } **Troubleshooting**

    ---

    Common issues and solutions.

    [:octicons-arrow-right-24: Troubleshooting](troubleshooting.md)

</div>
