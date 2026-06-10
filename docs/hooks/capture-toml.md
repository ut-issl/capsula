---
icon: material/hook
---

# capture-toml

Parses a single TOML file and embeds its parsed content in the run output
under the `parameters` field, alongside the configured path in `file`.

## Use Cases

- Make experiment hyperparameters queryable across runs
- Record the exact config used by a script in a structured, indexable form
- Compose multiple `capture-toml` entries to capture several config files

## Configuration

### Required Options

| Option | Type   | Description                                                |
| ------ | ------ | ---------------------------------------------------------- |
| `path` | string | Path to the TOML file to parse, relative to project root.  |

### Example

```toml
[[pre-run.hooks]]
id = "capture-toml"
path = "config/sat1/orbit.toml"
```

## Output Example

The `file` field contains the path verbatim as written in the config. The
`parameters` field contains the parsed TOML, converted to JSON.

Given `config/sat1/orbit.toml`:

```toml
[orbit]
a = 1.42
b = "LEO"
```

The hook output is:

```json
{
  "__meta": {
    "id": "capture-toml",
    "config": {
      "path": "config/sat1/orbit.toml"
    },
    "success": true
  },
  "file": "config/sat1/orbit.toml",
  "parameters": {
    "orbit": {
      "a": 1.42,
      "b": "LEO"
    }
  }
}
```

## Composing Multiple Files

This hook captures exactly one file per instance. Register one entry per
config file to capture them all:

```toml
[[pre-run.hooks]]
id = "capture-toml"
path = "config/sat1/orbit.toml"

[[pre-run.hooks]]
id = "capture-toml"
path = "config/sat2/orbit.toml"
```

Each entry produces its own row in `pre-run.json` with its own `file` and
`parameters` fields.

## TOML → JSON Conversion

TOML has a few types that JSON cannot represent natively. The hook handles
them as follows:

| TOML                         | JSON                                            |
| ---------------------------- | ----------------------------------------------- |
| String / Integer / Boolean   | matching JSON type                              |
| Float                        | JSON number (NaN / ±Inf become `null`)          |
| Datetime (offset / local)    | JSON string (RFC 3339 representation)           |
| Array                        | JSON array                                      |
| Table (`[section]`)          | JSON object                                     |

Example: `created_at = 2026-01-08T10:20:00Z` becomes
`"created_at": "2026-01-08T10:20:00Z"` in the JSON output, queryable as a
string.

## Error Behaviour

The hook fails (recorded with `__meta.success: false`) when:

- The file does not exist or is unreadable (`Io` error)
- The file content is not valid TOML (`Toml` error)

A failing `capture-toml` does not stop other hooks from running.

## See Also

- [`capture-file`](capture-file.md) — byte-exact archival of any file
- [`capture-json`](capture-json.md) — same shape, for JSON inputs
