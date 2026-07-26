---
icon: material/hook
---

# capture-yaml

Parses a single YAML file and embeds its parsed content in the run output
under the `content` field.

## Use Cases

- Make experiment hyperparameters queryable across runs
- Record the exact config used by a script in a structured, indexable form
- Compose multiple `capture-yaml` entries to capture several config files

## Configuration

### Required Options

| Option | Type   | Description                                                                               |
| ------ | ------ | ----------------------------------------------------------------------------------------- |
| `path` | string | Path to the YAML file to parse, relative to project root. Absolute paths are also accepted. |

### Example

```toml
[[pre-run.hooks]]
id = "capture-yaml"
path = "config/sat1/orbit.yaml"
```

## Output Example

The `content` field contains the parsed YAML, converted to JSON. The
configured path is not duplicated in the output body — it is already
preserved in the standard `__meta.config.path` field injected by the
orchestrator.

Given `config/sat1/orbit.yaml`:

```yaml
orbit:
  a: 1.42
  b: LEO
```

The hook output is:

```json
{
  "__meta": {
    "id": "capture-yaml",
    "config": {
      "path": "config/sat1/orbit.yaml"
    },
    "success": true
  },
  "content": {
    "orbit": {
      "a": 1.42,
      "b": "LEO"
    }
  }
}
```

To distinguish multiple `capture-yaml` outputs, filter on
`__meta.config.path`.

## Composing Multiple Files

This hook captures exactly one file per instance. Register one entry per
config file to capture them all:

```toml
[[pre-run.hooks]]
id = "capture-yaml"
path = "config/sat1/orbit.yaml"

[[pre-run.hooks]]
id = "capture-yaml"
path = "config/sat2/orbit.yaml"
```

Each entry produces its own row in `pre-run.json` with its own `content`
field; the configured path is available under `__meta.config.path`.

## YAML → JSON Conversion

The capture contract is intentionally minimal: the output is exactly what
[`yaml_serde`](https://crates.io/crates/yaml_serde)'s untyped
deserialization into JSON produces — the hook adds no validation layer of
its own. Plain, JSON-representable YAML maps as expected:

| YAML                          | JSON                                           |
| ----------------------------- | ---------------------------------------------- |
| String / Integer / Boolean    | matching JSON type                             |
| Float                         | JSON number (`.nan` / `±.inf` become `null`)   |
| Null (`~`, `null`, empty)     | JSON `null`                                    |
| Sequence (`- item`)           | JSON array                                     |
| Mapping (`key: value`)        | JSON object                                    |

Note that unlike TOML, YAML has no dedicated datetime type; timestamps
written as plain scalars are captured as strings.

### Edge-Case Behaviour

The table below illustrates some notable behaviors of the parser provided
by the `yaml_serde` crate. It is not exhaustive and these behaviors are
not guaranteed by Capsula.

| YAML input                             | Behaviour                                            |
| -------------------------------------- | ---------------------------------------------------- |
| Multi-document stream (`---`)          | Parse error                                          |
| Anchors / aliases (`&a` / `*a`)        | Expanded into the referenced value                   |
| Merge keys (`<<: *a`)                  | **Not applied** — captured as a literal `"<<"` key   |
| Non-string scalar keys (`1:`, `true:`) | Stringified (`"1"`, `"true"`)                        |
| Key collision after stringification    | **Last write wins** — the other value is dropped     |
| `!!binary` values                      | Captured as the raw base64 string                    |
| Custom tags (`!Custom`)                | Parse error                                          |
| Non-scalar mapping keys (`[1, 2]:`)    | Parse error                                          |

If your YAML relies on merge keys, non-string keys, or tags, the captured
JSON may not match the effective YAML semantics. Keep captured files to
plain, JSON-representable YAML to stay clear of these edge cases.

## Error Behaviour

The hook fails (recorded with `__meta.success: false`) when:

- The file does not exist or is unreadable (`Io` error)
- The file content is not valid YAML, or uses a YAML feature listed as a
  parse error above (`Yaml` error)

A failing `capture-yaml` does not stop other hooks from running.

## See Also

- [`capture-file`](capture-file.md) — byte-exact archival of any file
- [`capture-json`](capture-json.md) — same shape, for JSON inputs
- [`capture-toml`](capture-toml.md) — same shape, for TOML inputs