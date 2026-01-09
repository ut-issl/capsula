# capture-machine

Captures system information including CPU, memory, OS, and hostname.

## Use Cases

- **Track hardware specifications** - Know what machine ran your experiment
- **Debug platform-specific issues** - Understand differences between environments
- **Performance analysis** - Correlate results with system capabilities
- **Resource planning** - Document what resources experiments require

## Configuration

This hook requires no configuration options.

```toml
[[pre-run.hooks]]
id = "capture-machine"
```

## Output Example

```json
{
  "__meta": {
    "id": "capture-machine",
    "config": {},
    "success": true
  },
  "hostname": "macbook-pro.local",
  "os": "Darwin",
  "os_version": "25.2.0",
  "kernel_version": "25.0.0",
  "architecture": "aarch64",
  "total_memory": 68719476736,
  "cpus": [
    {
      "name": "1",
      "brand": "Apple M3 Max",
      "vendor_id": "Apple",
      "frequency_mhz": 4056
    },
    {
      "name": "2",
      "brand": "Apple M3 Max",
      "vendor_id": "Apple",
      "frequency_mhz": 4056
    }
  ]
}
```

### Output Fields

| Field | Type | Description |
|-------|------|-------------|
| `hostname` | string | System hostname |
| `os` | string | Operating system name (e.g., "Linux", "Darwin", "Windows") |
| `os_version` | string | OS version string |
| `kernel_version` | string | Kernel version |
| `architecture` | string | CPU architecture (e.g., "x86_64", "aarch64") |
| `total_memory` | number | Total RAM in bytes |
| `cpus` | array | List of CPU cores |
| `cpus[].name` | string | Core identifier |
| `cpus[].brand` | string | CPU brand/model name |
| `cpus[].vendor_id` | string | CPU vendor (e.g., "Intel", "AMD", "Apple") |
| `cpus[].frequency_mhz` | number | CPU frequency in MHz |

## Complete Example

```toml title="capsula.toml"
[vault]
name = "experiments"

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
```

Run:

```bash
capsula run python train.py
```

Output in `pre-run.json`:

```json
[
  {
    "__meta": {
      "id": "capture-machine",
      "config": {},
      "success": true
    },
    "hostname": "research-server-01",
    "os": "Linux",
    "os_version": "5.15.0-78-generic",
    "kernel_version": "5.15.0",
    "architecture": "x86_64",
    "total_memory": 137438953472,
    "cpus": [
      {
        "name": "1",
        "brand": "Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz",
        "vendor_id": "GenuineIntel",
        "frequency_mhz": 2400
      },
      // ... more CPUs
    ]
  }
]
```

## Platform-Specific Information

### Linux

- `os`: `"Linux"`
- `os_version`: Kernel version (e.g., `"5.15.0-78-generic"`)
- Complete CPU information from `/proc/cpuinfo`

### macOS

- `os`: `"Darwin"`
- `os_version`: Darwin version (e.g., `"25.2.0"`)
- CPU information from system profiler

### Windows

- `os`: `"Windows"`
- `os_version`: Windows version
- CPU information from WMI/registry

## Understanding the Output

### Memory (RAM)

`total_memory` is in bytes. To convert:

- **GB**: `total_memory / (1024 ** 3)`
- **Example**: `137438953472` bytes = `128` GB

### CPU Frequency

`frequency_mhz` is the base frequency, not boost frequency.

### CPU Count

The `cpus` array contains one entry per logical core (including hyperthreading/SMT cores).

- **Physical cores**: Number of actual CPU cores
- **Logical cores**: Length of `cpus` array (may include hyperthreading)

## Tips

### Use in Pre-Run Phase

Capture machine info in pre-run since it doesn't change:

```toml
[[pre-run.hooks]]
id = "capture-machine"
```

### Combine with Environment Capture

For complete context:

```toml
[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"

[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi"]
```

### Compare Across Machines

When running on multiple machines, machine info helps identify which machine produced which results:

```bash
# On server 1
capsula run python experiment.py

# On server 2
capsula run python experiment.py

# Later, compare results by checking machine info in pre-run.json
```

## Common Patterns

### Pattern: ML/AI Experiments

```toml
[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi", "--query-gpu=name,memory.total", "--format=csv"]

[[pre-run.hooks]]
id = "capture-env"
name = "CUDA_VISIBLE_DEVICES"
```

### Pattern: Performance Testing

```toml
[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-env"
name = "OMP_NUM_THREADS"

[[pre-run.hooks]]
id = "capture-command"
command = ["lscpu"]  # Linux only
```

### Pattern: Cross-Platform Testing

```toml
[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-command"
command = ["uname", "-a"]
```

## Common Questions

**Q: Why doesn't this capture GPU information?**

GPU information is not included because it's platform-specific and requires different tools on different systems. To capture GPU info, use the `capture-command` hook:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["nvidia-smi"]  # NVIDIA GPUs

[[pre-run.hooks]]
id = "capture-command"
command = ["rocm-smi"]  # AMD GPUs
```

**Q: Can I get more detailed CPU information?**

For more details, use platform-specific commands:

```toml
# Linux
[[pre-run.hooks]]
id = "capture-command"
command = ["lscpu"]

# macOS
[[pre-run.hooks]]
id = "capture-command"
command = ["sysctl", "-a"]

# Windows
[[pre-run.hooks]]
id = "capture-command"
command = ["wmic", "cpu", "get"]
```

**Q: Why are there so many CPU entries?**

Modern CPUs have multiple cores, and many support hyperthreading/SMT (e.g., a 4-core CPU shows as 8 logical cores).

**Q: Is memory usage captured?**

No, only total memory is captured. To capture current memory usage:

```toml
# Linux/macOS
[[pre-run.hooks]]
id = "capture-command"
command = ["free", "-h"]  # Linux

[[pre-run.hooks]]
id = "capture-command"
command = ["vm_stat"]  # macOS
```

**Q: Is disk space captured?**

No, but you can capture it with:

```toml
# Linux/macOS
[[pre-run.hooks]]
id = "capture-command"
command = ["df", "-h"]

# Windows
[[pre-run.hooks]]
id = "capture-command"
command = ["wmic", "logicaldisk", "get", "size,freespace"]
```

**Q: Does this slow down execution?**

The hook is very fast (typically < 10ms). System information is read from OS APIs, not computed.

## Interpreting Results

### Example Analysis

Given this output:

```json
{
  "hostname": "gpu-server-03",
  "os": "Linux",
  "architecture": "x86_64",
  "total_memory": 137438953472,
  "cpus": [...  # 32 entries
  ]
}
```

You can determine:

- **Server**: `gpu-server-03`
- **OS**: Linux
- **RAM**: `137438953472 / (1024**3)` = 128 GB
- **CPUs**: 32 logical cores

### Comparing Runs

To find all runs on a specific machine:

```bash
# Search for runs on a specific hostname
grep -r '"hostname": "gpu-server-03"' .capsula/*/*/*/_ capsula/pre-run.json
```

## Related Hooks

- [capture-command](capture-command.md) - Capture detailed system commands
- [capture-env](capture-env.md) - Capture environment variables affecting performance

[:octicons-arrow-left-24: Back to Hooks](../hooks.md)
