---
icon: material/hook
---

# capture-json

Parses a single JSON file and embeds its parsed content in the run output
under the `content` field.

## Use Cases

- Make experiment hyperparameters queryable across runs
- Record the exact config used by a script in a structured, indexable form
- Compose multiple `capture-json` entries to capture several config files

## Configuration

### Required Options

| Option | Type   | Description                                                |
| ------ | ------ | ---------------------------------------------------------- |
| `path` | string | Path to the JSON file to parse, relative to project root.  |

### Example

```toml
[[pre-run.hooks]]
id = "capture-json"
path = "config/sat1/orbit.json"
```

## Output Example

The `content` field contains the parsed JSON. The configured path is not
duplicated in the output body — it is already preserved in the standard
`__meta.config.path` field injected by the orchestrator.

```json
{
  "__meta": {
    "id": "capture-json",
    "config": {
      "path": "config/sat1/orbit.json"
    },
    "success": true
  },
  "content": {
    "a": 1.42,
    "b": "LEO"
  }
}
```

To distinguish multiple `capture-json` outputs (e.g., two files with the
same basename), filter on `__meta.config.path`.

## Composing Multiple Files

This hook captures exactly one file per instance. Register one entry per
config file to capture them all:

```toml
[[pre-run.hooks]]
id = "capture-json"
path = "config/sat1/orbit.json"

[[pre-run.hooks]]
id = "capture-json"
path = "config/sat2/orbit.json"
```

Each entry produces its own row in `pre-run.json` with its own `content`
field; the configured path is available under `__meta.config.path`.

## Error Behaviour

The hook fails (recorded with `__meta.success: false`) when:

- The file does not exist or is unreadable (`Io` error)
- The file content is not valid JSON (`Json` error)

A failing `capture-json` does not stop other hooks from running.

## See Also

- [`capture-file`](capture-file.md) — byte-exact archival of any file
- [`capture-toml`](capture-toml.md) — same shape, for TOML inputs
