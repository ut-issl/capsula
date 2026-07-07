---
icon: material/hook
---

# capture-git-repo

Captures git repository state including commit hash, branch, and whether there are uncommitted changes.

## Use Cases

- Record the exact code version used for reproducibility
- Prevent execution if there are uncommitted changes
- Ensure the commit is pushed to a remote so others can access it
- Track which code produced which results
- Audit which code version was used

## Configuration

### Required Options

| Option | Type | Description |
| -------- | ------ | ------------- |
| `name` | string | Base name used for the patch file (`<name>.patch`) when the repository has uncommitted changes |
| `path` | string | Path to the git repository (`.` for current directory) |

### Optional Options

| Option | Type | Default | Description |
| -------- | ------ | --------- | ------------- |
| `allow_dirty` | boolean | `false` | If `false`, Capsula aborts when the repository has uncommitted changes |
| `require_pushed` | boolean | `false` | If `true`, Capsula aborts when the HEAD commit is not pushed to the remote |
| `remote` | string | `"origin"` | Name of the remote to check when `require_pushed` is `true` |
| `tag_head` | boolean | `false` | If `true`, creates a lightweight tag `capsula/<run-name>` at the HEAD commit to prevent Git garbage collection |

### Example

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-project"
path = "."
allow_dirty = false
require_pushed = true
remote = "origin"
tag_head = true
```

## Output Example

### Clean Repository (pushed, with tag)

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "my-project",
      "path": ".",
      "allow_dirty": false,
      "require_pushed": true,
      "remote": "origin",
      "tag_head": true
    },
    "success": true
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": false,
  "is_pushed": true,
  "tag": "capsula/chubby-back"
}
```

### Dirty Repository (with `allow_dirty = true`)

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "my-project",
      "path": ".",
      "allow_dirty": true,
      "require_pushed": false,
      "remote": "origin"
    },
    "success": true
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": true,
  "is_pushed": true,
  "tag": null
}
```

!!! note "Patch file location"
    When the repository is dirty (and the hook is allowed to proceed), Capsula
    writes the diff to `<name>.patch` inside the hook's artifact directory,
    located at `{phase}-{index}-capture-git-repo/` under the run directory
    (e.g., `pre-0-capture-git-repo/my-project.patch`).

### Dirty Repository Failure (with `allow_dirty = false`)

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "my-project",
      "path": ".",
      "allow_dirty": false,
      "require_pushed": false,
      "remote": "origin"
    },
    "success": false,
    "failure_reason": "repository has uncommitted changes"
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": true,
  "is_pushed": true,
  "tag": null
}
```

!!! warning "Abort Behavior"
    When `allow_dirty = false` and the repository is dirty, Capsula saves the hook output showing the dirty state, runs the remaining pre-run hooks, then aborts before running your command.

!!! warning "Abort Behavior (push check)"
    When `require_pushed = true` and the HEAD commit is not reachable from any remote branch, Capsula saves the hook output, runs the remaining pre-run hooks, then aborts before running your command.

!!! note "Push check details"
    - The push check verifies that the HEAD commit is reachable from a remote-tracking branch of the configured remote. It does **not** require HEAD to be at the tip of a remote branch — ancestor commits are also considered pushed.
    - The check relies on local remote-tracking references. Run `git fetch` before `capsula run` if you need up-to-date remote state.

!!! warning "Squash merge and commit reachability"
    Even if `require_pushed = true` passes at the time of the run, the commit may later become unreachable if the branch is squash-merged and deleted. After a squash merge, the original commits are replaced by a single new commit on the target branch, and the original commit SHAs are no longer reachable. To preserve commit reachability, use **merge commits** (not squash merge) when merging branches that contain experiment runs.

!!! tip "Preventing garbage collection with `tag_head`"
    When `tag_head = true`, Capsula creates a lightweight Git tag `capsula/<run-name>` pointing to the HEAD commit. This prevents Git from garbage-collecting the commit even after branch deletion or history rewriting (e.g., rebase, squash merge). You can list all Capsula tags with `git tag -l 'capsula/*'` and clean them up with `git tag -d <tag-name>`.
