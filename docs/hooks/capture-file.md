---
icon: material/hook
---

# capture-file

Captures files by copying them, moving them, or computing their hash.

## Use Cases

- Preserve input configurations
- Archive output results
- Verify file integrity without copying large files
- Organize experiment artifacts

## Configuration

### Required Options

| Option | Type | Description |
| -------- | ------ | ------------- |
| `glob` | string | File pattern to match (e.g., `"*.txt"`, `"results/**/*.png"`) |

### Optional Options

| Option | Type | Default | Description |
| -------- | ------ | --------- | ------------- |
| `mode` | string | `"copy"` | How to handle files: `"copy"`, `"move"`, or `"none"` |
| `hash` | string | `"sha256"` | Hash algorithm: `"sha256"` or `"none"` |

### Glob Patterns

`glob` is matched against paths relative to the project root (the directory
containing `capsula.toml`):

- `*` and `?` match within one path component, so `*.txt` matches only files
  at the project root, and `results/*.csv` only files directly in `results/`.
- `**` matches any number of directories, so `**/*.txt` matches `.txt` files
  at any depth.
- Use `/` as the path separator. On Windows, `\` is also a separator; on other
  platforms it is an ordinary file name character.
- A pattern that is absolute (such as `/home/me/.aws/*`) or contains a `..`
  component (such as `../data/*`) is rejected when the configuration is
  loaded.
- Symbolic links are followed, as in a shell, so `data/*.csv` matches files
  in `data/` even if `data` is a link to a directory elsewhere.

Only directories that can contain a match are read: `results/*.csv` reads
`results/` but no other directory, while `**` reads every directory below
it. A directory that must be read but cannot be fails the hook.

!!! note "This is not a security boundary"
    Rejecting absolute and `..` patterns prevents mistakes, such as moving
    files out of another project. It does not stop a malicious
    `capsula.toml`, which could capture the same files through a symbolic
    link or another hook such as `capture-command`. Run only configurations
    you trust.

### Mode Options

- `"copy"` - Copies files to the hook's artifact directory, leaves originals
- `"move"` - Copies files to the hook's artifact directory, then removes the
  originals
- `"none"` - Only computes hashes, doesn't copy or move

In `"copy"` and `"move"` modes, each file is read once, and the hash is
computed from the bytes as they are written to the artifact, so it always
describes the archived file, even if the original changes during the copy. In
`"none"` mode, the original itself is hashed.

In `"move"` mode, an original is removed only after its copy is complete, and
only if it is still the file that was copied and its size and modification
time are unchanged. Otherwise the original is left in place and the hook
fails; the artifact holds the bytes that were copied. A matched symbolic link
is followed: its target is archived, and in `"move"` mode the link is removed
and its target is kept.

!!! warning "Move only files that nothing is still writing"
    `"move"` mode assumes that no other process writes to a matched file
    while the hook runs, as is the case for the outputs of a command that has
    exited. The checks above catch most concurrent writes, but not all of
    them:

    - The original is checked just before it is removed. A write between
      that check and the removal is lost.
    - A write that keeps the file's size is detected only through its
      modification time, so it can go unnoticed if the file system records
      the time too coarsely.

    Use `"copy"` mode for a file that may still be written, such as the log
    of a background process.

Every destination is checked before any file is copied or moved. If capturing
a file fails after others were captured, the hook fails, and its output still
lists the files captured so far, so that a moved file stays recorded.

!!! note "Per-hook artifact directory"
    When `mode` is `"copy"` or `"move"`, files are placed in a per-hook
    artifact directory named `{phase}-{index}-capture-file/` under the run
    directory (e.g., `post-0-capture-file/`), so different `capture-file`
    hooks do not collide. Each file keeps its path relative to the project
    root: `results/a/data.csv` is stored as
    `post-0-capture-file/results/a/data.csv`, so files with the same name in
    different directories are kept apart. An existing file is never
    overwritten: if the destination already exists, the hook fails. Copied
    files keep the permissions of the original.

### Example

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.csv"
mode = "copy"
hash = "sha256"
```

## Output Example

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {
      "glob": "results/*.csv",
      "mode": "copy",
      "hash": "sha256"
    },
    "success": true
  },
  "files": [
    {
      "path": "/home/me/project/results/data.csv",
      "copied_path": "/home/me/project/.capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/results/data.csv",
      "hash": "sha256:a1b2c3d4e5f6..."
    },
    {
      "path": "/home/me/project/results/summary.csv",
      "copied_path": "/home/me/project/.capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/results/summary.csv",
      "hash": "sha256:b2c3d4e5f6a7..."
    }
  ]
}
```
