# capture_machine

Captures system information including hostname, OS, CPU, and memory.

## Configuration

```toml
[[pre_run]]
type = "capture_machine"
```

## Parameters

This hook has no configuration parameters.

## Phases

- ✅ Pre-run
- ✅ Post-run

## Output

```json
{
  "__meta": {
    "id": "capture_machine",
    "config": {},
    "success": true
  },
  "hostname": "alice-macbook",
  "os": {
    "name": "Darwin",
    "version": "25.2.0",
    "distribution": "macOS"
  },
  "cpu": {
    "model": "Apple M1",
    "cores": 8,
    "physical_cores": 8
  },
  "memory": {
    "total_bytes": 17179869184,
    "total_gb": 16.0
  }
}
```

### Fields

- `hostname` (string): Machine hostname
- `os` (object): Operating system information
  - `name` (string): OS kernel name (e.g., "Linux", "Darwin", "Windows")
  - `version` (string): OS version
  - `distribution` (string, optional): Distribution name (e.g., "Ubuntu", "macOS")
- `cpu` (object): CPU information
  - `model` (string): CPU model name
  - `cores` (number): Total CPU cores (including hyper-threading)
  - `physical_cores` (number): Physical CPU cores
- `memory` (object): Memory information
  - `total_bytes` (number): Total RAM in bytes
  - `total_gb` (number): Total RAM in gigabytes

## Use Cases

### Document Hardware for Experiments

Record hardware specifications for reproducibility:

```toml
[[pre_run]]
type = "capture_machine"
```

### Debug Hardware-Specific Issues

Capture system info when investigating platform-specific bugs:

```toml
[vault]
name = "bug-reports"

[[pre_run]]
type = "capture_machine"

[[pre_run]]
type = "capture_env"
include = ["PATH", "SHELL"]
```

### Track Resource Availability

Document available resources for resource-intensive tasks:

```toml
[vault]
name = "training-runs"

[[pre_run]]
type = "capture_machine"

[[pre_run]]
type = "capture_env"
include = ["CUDA_VISIBLE_DEVICES"]
```

## Examples

### Basic Usage

```toml
[vault]
name = "experiments"

[[pre_run]]
type = "capture_machine"
```

```bash
capsula run python train.py
```

Output in `pre-run.json`:

```json
[
  {
    "__meta": {
      "id": "capture_machine",
      "config": {},
      "success": true
    },
    "hostname": "compute-node-01",
    "os": {
      "name": "Linux",
      "version": "5.15.0-91-generic",
      "distribution": "Ubuntu 22.04"
    },
    "cpu": {
      "model": "Intel Xeon E5-2680 v4",
      "cores": 56,
      "physical_cores": 28
    },
    "memory": {
      "total_bytes": 137438953472,
      "total_gb": 128.0
    }
  }
]
```

### Combined with Environment Capture

```toml
[vault]
name = "ml-experiments"

[[pre_run]]
type = "capture_machine"

[[pre_run]]
type = "capture_env"
include = [
    "CUDA_VISIBLE_DEVICES",
    "OMP_NUM_THREADS",
    "MKL_NUM_THREADS"
]

[[pre_run]]
type = "capture_command"
command = "nvidia-smi --query-gpu=name,memory.total --format=csv"
```

### Platform-Specific Debugging

```toml
[vault]
name = "platform-tests"

[[pre_run]]
type = "capture_machine"

[[pre_run]]
type = "capture_env"
include = ["ARCH", "PLATFORM"]

[[post_run]]
type = "capture_command"
command = "uname -a"
```

## Output Examples by Platform

### macOS

```json
{
  "hostname": "macbook-pro",
  "os": {
    "name": "Darwin",
    "version": "25.2.0",
    "distribution": "macOS"
  },
  "cpu": {
    "model": "Apple M2 Pro",
    "cores": 12,
    "physical_cores": 12
  },
  "memory": {
    "total_bytes": 34359738368,
    "total_gb": 32.0
  }
}
```

### Linux

```json
{
  "hostname": "gpu-server",
  "os": {
    "name": "Linux",
    "version": "6.5.0-14-generic",
    "distribution": "Ubuntu 24.04"
  },
  "cpu": {
    "model": "AMD EPYC 7763",
    "cores": 128,
    "physical_cores": 64
  },
  "memory": {
    "total_bytes": 549755813888,
    "total_gb": 512.0
  }
}
```

### Windows

```json
{
  "hostname": "DESKTOP-ABC123",
  "os": {
    "name": "Windows",
    "version": "10.0.22631",
    "distribution": "Windows 11"
  },
  "cpu": {
    "model": "Intel Core i7-13700K",
    "cores": 24,
    "physical_cores": 16
  },
  "memory": {
    "total_bytes": 68719476736,
    "total_gb": 64.0
  }
}
```

## Use in Multi-Machine Environments

### Cluster Computing

When running experiments across multiple machines:

```toml
[vault]
name = "distributed-training"

[[pre_run]]
type = "capture_machine"

[[pre_run]]
type = "capture_env"
include = [
    "SLURM_JOB_ID",
    "SLURM_NODELIST",
    "HOSTNAME"
]
```

### Cloud Instances

Track which cloud instance type was used:

```toml
[vault]
name = "cloud-experiments"

[[pre_run]]
type = "capture_machine"

[[pre_run]]
type = "capture_env"
include = [
    "AWS_INSTANCE_TYPE",
    "GCP_MACHINE_TYPE",
    "AZURE_VM_SIZE"
]
```

## Error Handling

This hook rarely fails. Possible errors:

- Unable to determine hostname
- Cannot read CPU information
- Cannot read memory information

Partial information is captured even if some fields cannot be determined:

```json
{
  "__meta": {
    "id": "capture_machine",
    "config": {},
    "success": true
  },
  "hostname": "unknown",
  "os": {
    "name": "Linux",
    "version": "unknown"
  },
  "cpu": {
    "model": "unknown",
    "cores": null
  },
  "memory": {
    "total_bytes": 0,
    "total_gb": 0.0
  }
}
```

## See Also

- [capture_env](capture-env.md) - Capture environment variables
- [capture_command](capture-command.md) - Run diagnostic commands
- [capture_cwd](capture-cwd.md) - Capture working directory
