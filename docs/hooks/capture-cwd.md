# capture-cwd

Captures the current working directory where Capsula is running.

## Use Cases

- **Record execution location** - Know where commands were run from
- **Debugging path issues** - Understand relative path contexts
- **Audit compliance** - Track where processes execute

## Configuration

This hook requires no configuration options.

```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

That's it - no additional options needed!

## Output Example

```json
{
  "__meta": {
    "id": "capture-cwd",
    "config": {},
    "success": true
  },
  "cwd": "/Users/username/projects/my-experiment"
}
```

### Output Fields

| Field | Type | Description |
|-------|------|-------------|
| `cwd` | string | Absolute path to the current working directory |

## Complete Example

```toml title="capsula.toml"
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-cwd"
```

Run:

```bash
cd /path/to/project
capsula run python script.py
```

Output in `pre-run.json`:

```json
[
  {
    "__meta": {
      "id": "capture-cwd",
      "config": {},
      "success": true
    },
    "cwd": "/path/to/project"
  }
]
```

## Tips

### Use with Relative Paths

When your command uses relative file paths, capturing the working directory helps understand those paths later:

```toml
[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"  # Relative to cwd
mode = "copy"
```

### Combine with Git Hook

Together, `capture-cwd` and `capture-git-repo` give complete location context:

```toml
[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
```

This records both:

- Where you ran the command (cwd)
- Which repository you're in (git)

## Common Questions

**Q: Can I change the captured directory?**

No, this hook always captures the actual current working directory. If you want to run commands from a specific directory, use `cd` first:

```bash
cd /path/to/dir && capsula run python script.py
```

**Q: What if my working directory is not a git repository?**

That's fine! `capture-cwd` works anywhere. It just captures the directory path, regardless of whether it's in a git repository or not.

**Q: Is this different from `CAPSULA_PROJECT_ROOT`?**

Yes:
- `capture-cwd` captures the **current** working directory when Capsula runs
- `CAPSULA_PROJECT_ROOT` is the directory containing `capsula.toml`

They may be different if you run Capsula from a subdirectory.

## Related Hooks

- [capture-git-repo](capture-git-repo.md) - Capture repository information
- [capture-env](capture-env.md) - Capture the `PWD` environment variable

[:octicons-arrow-left-24: Back to Hooks](../hooks.md)
