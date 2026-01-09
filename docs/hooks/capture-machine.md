# capture-machine

Captures system information including CPU, memory, OS, and hostname.

## Use Cases

- Track hardware specifications
- Debug platform-specific issues
- Correlate results with system capabilities
- Document resource requirements

## Configuration

No configuration options needed.

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

[:octicons-arrow-left-24: Back to Configuration](../configuration.md#available-hook-types)
