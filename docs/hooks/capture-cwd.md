# capture-cwd

Captures the current working directory where Capsula is running.

## Use Cases

- Record execution location
- Debug relative path issues
- Track where processes execute

## Configuration

No configuration options needed.

```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

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
