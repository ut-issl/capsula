---
icon: material/hook
---

# capture-json

Parses a single JSON file and embeds its parsed content in the run output
under the `parameters` field, alongside the configured path in `file`.

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

The `file` field contains the path verbatim as written in the config (useful
for distinguishing multiple `capture-json` outputs that share a basename).
The `parameters` field contains the parsed JSON.

```json
{
  "__meta": {
    "id": "capture-json",
    "config": {
      "path": "config/sat1/orbit.json"
    },
    "success": true
  },
  "file": "config/sat1/orbit.json",
  "parameters": {
    "a": 1.42,
    "b": "LEO"
  }
}
```

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

Each entry produces its own row in `pre-run.json` with its own `file` and
`parameters` fields.

## Error Behaviour

The hook fails (recorded with `__meta.success: false`) when:

- The file does not exist or is unreadable (`Io` error)
- The file content is not valid JSON (`Json` error)

A failing `capture-json` does not stop other hooks from running.

## See Also

- [`capture-file`](capture-file.md) — byte-exact archival of any file
