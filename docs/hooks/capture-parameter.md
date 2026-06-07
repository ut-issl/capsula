---
icon: material/hook
---

# capture-parameter

Parses structured parameter files (TOML / JSON / YAML) into JSON and embeds the
parsed values directly into `pre-run.json` / `post-run.json` so they can be
queried by the server.

## Use Cases

- Make experiment hyperparameters queryable across runs
- Power dashboards that show parameter values alongside metrics
- Normalize configs written in different formats (TOML / JSON / YAML) into a
  single queryable schema
- Diff parameter sets between two runs without re-reading raw files

## Configuration

### Required Options

| Option | Type   | Description                                                                  |
| ------ | ------ | ---------------------------------------------------------------------------- |
| `glob` | string | File pattern relative to project root (e.g., `"configs/*.yaml"`, `"*.toml"`) |

### Optional Options

| Option         | Type   | Default | Description                                                                                                                              |
| -------------- | ------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `strip_prefix` | string | (none)  | Literal path prefix to remove from each matched file's relative path before constructing nested keys. Must be a prefix of every match.   |

### Supported File Formats

The hook detects the format by file extension (case-insensitive):

| Extension       | Parser        |
| --------------- | ------------- |
| `.json`         | `serde_json`  |
| `.toml`         | `toml`        |
| `.yaml`, `.yml` | `serde_yaml`  |

Files matching the glob whose extension is none of the above produce an
`UnsupportedFileType` error and abort the hook.

### Example

```toml
[[pre-run.hooks]]
id = "capture-parameter"
glob = "configs/*.yaml"
```

## Output Example

Each matched file's path (relative to `project_root`, with `strip_prefix`
removed if set) becomes a sequence of nested keys: directory components are
intermediate keys, and the file stem is the final key. Sibling files merge
into the same object naturally.

Given:

```text
config/sat1/orbit.json     { "a": 1, "b": 2 }
config/sat1/attitude.yaml  { "q": [0, 0, 0, 1] }
config/sat2/orbit.json     { "a": 3 }
```

with config:

```toml
[[pre-run.hooks]]
id = "capture-parameter"
glob = "config/**/*.{json,yaml}"
strip_prefix = "config"
```

the captured output is:

```json
{
  "__meta": {
    "id": "capture-parameter",
    "config": {
      "glob": "config/**/*.{json,yaml}",
      "strip_prefix": "config"
    },
    "success": true
  },
  "parameters": {
    "sat1": {
      "attitude": { "q": [0, 0, 0, 1] },
      "orbit":    { "a": 1, "b": 2 }
    },
    "sat2": {
      "orbit": { "a": 3 }
    }
  }
}
```

The server can then query the parsed values directly, e.g.,
`WHERE parameters.sat1.orbit.a > 0`.

## Merge & Conflict Semantics

Files whose computed key paths collide are **deep-merged** into the same
object. This unifies two situations that would otherwise need separate
handling:

1. **Same-stem cross-format files** — `orbit.json` and `orbit.yaml` in the
   same directory both contribute to `parameters.<dir>.orbit`.
2. **Leaf vs. intermediate node** — a file `sat1.json` at level N and any
   files under `sat1/` at level N+1 both contribute to `parameters.sat1`.

A `ParameterConflict` error is raised only when the merge cannot reconcile
two values. The error message includes the dotted key path where the conflict
occurred.

| Situation                                                | Outcome                            |
| -------------------------------------------------------- | ---------------------------------- |
| Disjoint keys at the same level                          | Merged                             |
| Same key, identical scalar / array value                 | Merged (idempotent)                |
| Same key, **different** scalar / array values            | `ParameterConflict` at that path   |
| Same key, one side object and the other scalar / array   | `ParameterConflict` at that path   |

Arrays are treated as **opaque leaf values** — element-wise merging is not
performed. Two arrays must be exactly equal to merge.

!!! note "`strip_prefix` mismatch"
    If `strip_prefix` is set but a matched file's relative path does not start
    with it, the hook fails with `StripPrefixMismatch`. Either narrow the glob
    so it cannot escape the prefix, or remove `strip_prefix`.

## When to Use vs `capture-file`

`capture-parameter` and [`capture-file`](capture-file.md) can both record the
state of a configuration file at run time, but they answer different questions.
Pick by what you intend to do with the captured data downstream.

### Axis 1: Queryability vs Byte-exactness

| Concern                                   | `capture-file`                   | `capture-parameter`                       |
| ----------------------------------------- | -------------------------------- | ----------------------------------------- |
| Stored content                            | Raw bytes (+ optional hash)      | Parsed JSON value                         |
| Cross-run server query (sql-json-path)    | Hash / path only                 | Full structured query on parameter values |
| Bit-exact reproduction of the input file  | Yes                              | No (formatting / comments lost)           |
| Hash for content-addressable identity     | Yes                              | No                                        |
| File format constraint                    | Any (binary OK)                  | TOML / JSON / YAML only                   |
| Comments preserved                        | Yes                              | No                                        |
| Format-specific types (TOML datetime)     | Preserved as written             | Coerced to JSON string                    |
| Output location                           | Artifact directory + meta entry  | Inline `parameters` map in `pre-run.json` |

### Axis 2: Use case decision matrix

| Scenario                                                    | Recommended hook            |
| ----------------------------------------------------------- | --------------------------- |
| Hyperparameter sweep, want to query top runs by `lr` value  | `capture-parameter`         |
| Leaderboard dashboard joining params with metrics           | `capture-parameter`         |
| Reproduce a bug exactly from a tricky YAML formatting       | `capture-file`              |
| Compliance archive of every config used in production       | `capture-file`              |
| Save a model checkpoint (`.pth`) or dataset (`.csv`)        | `capture-file`              |
| Preserve hand-written comments in a YAML config             | `capture-file`              |
| Normalize configs across teams using different formats      | `capture-parameter`         |
| Full audit + queryable params (production ML)               | **Both** (see below)        |

### Combined pattern (recommended for production)

For production-grade tracking, register both hooks against the same glob. You
get a byte-exact archive **and** queryable structured values.

```toml
[[pre-run.hooks]]
id = "capture-file"
glob = "configs/*.yaml"
mode = "copy"
hash = "sha256"

[[pre-run.hooks]]
id = "capture-parameter"
glob = "configs/*.yaml"
```

The two outputs serve different consumers:

- `capture-file` → archival, replay, compliance audit
- `capture-parameter` → server query, dashboards, cross-run analysis

### Mental model

- `capture-file` is `git add path/to/file` — it preserves bytes.
- `capture-parameter` is `INSERT INTO params (...) VALUES (...)` — it gives
  the server queryable data.

If you only ever look at one run at a time, `capture-file` is usually enough.
The moment you want to ask a question that **spans runs**
(`WHERE learning_rate > 0.01`, `GROUP BY architecture`), you need
`capture-parameter`.

## Limitations

The conversion to JSON is intentionally simple and drops several pieces of
information that JSON cannot represent. If any of these matter for a given
file, capture the raw bytes with `capture-file` instead (or in addition).

- **Comments** in TOML / YAML are removed.
- **TOML datetime types** are coerced to RFC 3339 strings; the original type
  tag is not preserved.
- **YAML anchors / aliases** (`&` / `*`) are resolved at parse time. References
  become duplicated content.
- **YAML merge keys** (`<<`) are resolved at parse time.
- **YAML non-string mapping keys** are not supported and will fail
  serialization to JSON.
- **YAML special floats** (`.inf`, `.nan`) are not representable in JSON.
- **Multi-document YAML streams** (`---` separators) are not supported.
- **Conflicting parameter values** across matched files are rejected with
  `ParameterConflict`. See [Merge & Conflict Semantics](#merge-conflict-semantics).
  Disambiguate via a narrower glob, by renaming files / keys, or by splitting
  into multiple `capture-parameter` entries.
- **Element-wise array merge** is not supported. Arrays must be exactly equal
  across files to coexist at the same key.

## See Also

- [`capture-file`](capture-file.md) — byte-exact file capture
- [`capture-env`](capture-env.md) — single environment variable capture
