# Frequently Asked Questions

## General Questions

### What is Capsula?

Capsula is a command-line tool that captures the context of your command executions. It records what command you ran, when you ran it, what the environment was like, and what the results were.

Think of it as a "time capsule" for your work - you can always go back and see exactly what happened.

### Do I need to be a programmer to use Capsula?

You need basic command-line skills and the ability to edit text files (TOML format). If you can run commands in a terminal and edit configuration files, you can use Capsula.

### Is Capsula free?

Yes! Capsula is open source and free to use under the MIT or Apache 2.0 licenses.

### What platforms does Capsula support?

Capsula works on:

- Linux
- macOS
- Windows

### Do I need the server?

No, the server is completely optional. By default, Capsula stores runs locally in `.capsula/` directories. The server is useful if you want to:

- Share runs with a team
- Access runs from multiple machines
- Browse runs in a web interface

## Installation and Setup

### How do I install Capsula?

```bash
cargo install capsula-cli --locked
```

See the [Installation guide](installation.md) for details.

### How do I update Capsula?

Run the same install command again:

```bash
cargo install capsula-cli --locked
```

### Where does Capsula store data?

By default, in `.capsula/` directories in your project. You can customize this in `capsula.toml`:

```toml
[vault]
name = "my-vault"
path = "/custom/path"  # Optional
```

### Can I use Capsula without changing my workflow?

Yes! Just add `capsula run` before your normal commands:

```bash
# Normal
python train.py

# With Capsula
capsula run python train.py
```

## Configuration

### Do I need a configuration file?

Yes, you need a `capsula.toml` file. Here's a minimal example:

```toml
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-cwd"
```

### Where should I put `capsula.toml`?

In your project root directory. Capsula will find it automatically from subdirectories.

### Can I have multiple configuration files?

Yes! Use the `--config` flag:

```bash
capsula --config dev.toml run python test.py
capsula --config prod.toml run python deploy.py
```

### What's the difference between pre-run and post-run hooks?

- **Pre-run hooks** capture information **before** your command runs (initial state, inputs)
- **Post-run hooks** capture information **after** your command completes (results, outputs)

## Usage

### Does Capsula slow down my commands?

Minimally. The overhead is typically:

- A few milliseconds for hook execution
- File I/O time if copying large files

You can reduce overhead by:

- Using `mode = "none"` for large files
- Using `hash = "none"` if you don't need hashes
- Removing unnecessary hooks

### Can I use Capsula with any command?

Yes! Capsula works with any command:

```bash
capsula run python script.py
capsula run cargo build
capsula run bash my-script.sh
capsula run make
```

### How do I pass arguments to my command?

Just include them after the command:

```bash
capsula run python train.py --epochs 100 --lr 0.01
```

### Can I use pipes and redirects?

Yes, but wrap them in a shell command:

```bash
capsula run bash -c 'python generate.py | grep "result" > output.txt'
```

### How do I see what was captured?

```bash
# List runs
capsula list

# View captured data
cat .capsula/my-vault/2025-01-09/143022-happy-river/_capsula/metadata.json
cat .capsula/my-vault/2025-01-09/143022-happy-river/_capsula/pre-run.json
cat .capsula/my-vault/2025-01-09/143022-happy-river/_capsula/command.json
cat .capsula/my-vault/2025-01-09/143022-happy-river/_capsula/post-run.json
```

## Hooks

### Which hooks should I use?

Start with the basics:

```toml
[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
```

Add more as needed. See [Examples](examples.md) for common patterns.

### Can I use the same hook multiple times?

Yes! For example, capturing multiple environment variables:

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-env"
name = "HOME"

[[pre-run.hooks]]
id = "capture-env"
name = "USER"
```

### What if a hook fails?

Most hook failures are non-fatal - they're logged and saved, but other hooks continue. Only some hooks (like `capture-git-repo` with `allow_dirty = false`) can abort execution.

### Can I create custom hooks?

Not without modifying Capsula's source code. Capsula is designed with a fixed set of hooks. However, you can use `capture-command` to run custom scripts:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["./my-custom-script.sh"]
```

## Git Integration

### Why does Capsula abort when my repository is dirty?

By default, `capture-git-repo` with `allow_dirty = false` ensures reproducibility by requiring a clean git state. This means you can always recreate results from a specific commit.

To allow uncommitted changes:

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
path = "."
allow_dirty = true
```

### Do I need to use git?

No, git is optional. You can use Capsula without the `capture-git-repo` hook.

### What if I'm not in a git repository?

The `capture-git-repo` hook will fail (non-fatally) and the error will be logged. Other hooks will continue normally.

## Files and Storage

### Can Capsula handle large files?

Yes, but:

- Copying large files takes time and space
- Consider using `mode = "none"` with `hash = "sha256"` to just verify integrity:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "large-model.bin"
mode = "none"
hash = "sha256"
```

### How much disk space does Capsula use?

It depends on what you capture. Each run creates:

- Metadata files (a few KB)
- Copied files (if using `mode = "copy"` or `mode = "move"`)
- Command output

To save space:

- Use `mode = "none"` for large files
- Clean old runs periodically
- Use a separate disk for vaults

### How do I clean up old runs?

```bash
# Remove runs older than 30 days
find .capsula/ -type d -mtime +30 -exec rm -rf {} +

# Or manually delete vault directories
rm -rf .capsula/my-vault/2025-01-01
```

### Where should I not write output files?

Don't write files directly to `CAPSULA_RUN_DIRECTORY`. Instead:

1. Write files to your project directory
2. Capture them with a `capture-file` hook

This ensures proper file management.

## Slack Notifications

### Do I need a Slack workspace?

Yes, you need access to a Slack workspace where you can install apps.

### How do I get a Slack token?

1. Go to [api.slack.com/apps](https://api.slack.com/apps)
2. Create a new app
3. Add `chat:write` scope (and `files:write` for attachments)
4. Install to workspace
5. Copy the bot token (starts with `xoxb-`)

See [notify-slack hook documentation](hooks/notify-slack.md) for detailed steps.

### Can I send to multiple channels?

Yes, use multiple hooks:

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#team"

[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
```

### Why aren't my files attaching to Slack?

Check:

1. Files exist when the hook runs
2. Hook order (Slack hook before `capture-file` with `mode = "move"`)
3. You have `files:write` scope
4. File count doesn't exceed 10 (Slack limit)

See [Troubleshooting](troubleshooting.md#files-not-attaching-to-slack).

## Environment Variables

### What environment variables does Capsula set?

When running your command, Capsula sets:

- `CAPSULA_RUN_ID` - Unique run identifier
- `CAPSULA_RUN_NAME` - Human-readable name
- `CAPSULA_RUN_DIRECTORY` - Run directory path
- `CAPSULA_RUN_TIMESTAMP` - ISO 8601 timestamp
- `CAPSULA_RUN_COMMAND` - Command being executed
- `CAPSULA_PRE_RUN_OUTPUT_PATH` - Path to pre-run.json
- `CAPSULA_PROJECT_ROOT` - Project root directory

### Can I use these variables in my scripts?

Yes!

```python
import os

run_name = os.environ['CAPSULA_RUN_NAME']
print(f"Running experiment: {run_name}")
```

```bash
echo "Run ID: $CAPSULA_RUN_ID"
echo "Run name: $CAPSULA_RUN_NAME"
```

### How do I load environment variables from a file?

Use the `dotenv` option:

```toml
dotenv = ".env"

[vault]
name = "my-project"
```

```bash title=".env"
API_KEY=secret
DATABASE_URL=postgresql://localhost/db
```

## Reproducibility

### How does Capsula ensure reproducibility?

Capsula captures:

- Exact code version (git commit)
- System specifications
- Environment variables
- Input files and configurations
- Command that was run

With this information, you can recreate the exact conditions of a run.

### What if I can't reproduce a result?

Check the captured data to see if anything differs:

- Git commit (different code?)
- Environment variables (different settings?)
- Input files (different data?)
- System specs (different hardware?)

### Can Capsula help with scientific reproducibility?

Yes! Capsula is designed for this. It captures everything needed to reproduce scientific experiments. See the [Research Paper example](examples.md#research-paper-experiments).

## Server

### Do I need PostgreSQL?

Only if you want to run the Capsula server. The CLI works without it.

### Can multiple people use the same server?

Yes! The server is designed for team use. Everyone can push runs to the same server and browse each other's runs.

### Is the server production-ready?

The server is functional but consider it alpha/beta quality. For production use:

- Use a reverse proxy (nginx)
- Enable HTTPS
- Set up proper database backups
- Use a process manager (systemd)

See [Server Setup](server-setup.md#production-deployment).

## Troubleshooting

### Where can I get help?

1. Check the [Troubleshooting guide](troubleshooting.md)
2. Read this FAQ
3. Check [GitHub Issues](https://github.com/ut-issl/capsula/issues)
4. Open a new issue if needed

### How do I report a bug?

Go to [GitHub Issues](https://github.com/ut-issl/capsula/issues) and include:

- Capsula version: `capsula --version`
- Your `capsula.toml` (remove sensitive data)
- Steps to reproduce
- Error messages
- Expected vs actual behavior

### How do I request a feature?

Open an issue on [GitHub](https://github.com/ut-issl/capsula/issues) describing:

- What you want to do
- Why current features don't work for you
- How the feature would help

## Advanced Usage

### Can I run Capsula in CI/CD?

Yes! Capsula works great in CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Run tests with Capsula
  run: capsula run pytest

- name: Upload test results
  uses: actions/upload-artifact@v2
  with:
    name: capsula-run
    path: .capsula/
```

### Can I nest Capsula runs?

Yes, but it's not recommended. Each run creates its own directory, so nesting doesn't provide additional benefit.

### Can I programmatically access captured data?

Yes! All captured data is in JSON format:

```python
import json

# Load metadata
with open('.capsula/vault/2025-01-09/143022-happy-river/_capsula/metadata.json') as f:
    metadata = json.load(f)
    print(f"Run ID: {metadata['id']}")
    print(f"Command: {metadata['command']}")
```

Or use the server's REST API.

## What's Next?

<div class="grid cards" markdown>

-   :material-rocket-launch:{ .lg .middle } **Getting Started**

    ---

    New to Capsula? Start here.

    [:octicons-arrow-right-24: Get started](getting-started.md)

-   :material-book-open-variant:{ .lg .middle } **Examples**

    ---

    See real-world examples.

    [:octicons-arrow-right-24: View examples](examples.md)

-   :material-help-circle:{ .lg .middle } **Troubleshooting**

    ---

    Common issues and solutions.

    [:octicons-arrow-right-24: Troubleshooting](troubleshooting.md)

</div>
