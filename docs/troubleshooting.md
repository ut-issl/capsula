# Troubleshooting

This page covers common issues you might encounter when using Capsula and how to solve them.

## Installation Issues

### "cargo: command not found"

**Problem:** Rust is not installed or not in your PATH.

**Solution:**

1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Restart your terminal
3. Verify: `cargo --version`

---

### Compilation Errors During Installation

**Problem:** Errors while compiling Capsula.

**Solution:**

1. Update Rust:
   ```bash
   rustup update
   ```

2. Clear cargo cache and try again:
   ```bash
   rm -rf ~/.cargo/registry
   cargo install capsula-cli --locked
   ```

3. Check Rust version (must be 1.90 or higher):
   ```bash
   rustc --version
   ```

---

### "permission denied" During Installation

**Problem:** Cargo is trying to install to a system directory.

**Solution:**

Make sure Rust was installed with rustup (not a system package manager). Rustup installs to your home directory automatically.

## Configuration Issues

### "Configuration file not found"

**Problem:** Capsula can't find `capsula.toml`.

**Solution:**

1. Create `capsula.toml` in your project directory:
   ```toml
   [vault]
   name = "my-vault"

   [[pre-run.hooks]]
   id = "capture-cwd"
   ```

2. Or specify the config file location:
   ```bash
   capsula --config /path/to/capsula.toml run echo test
   ```

---

### "Failed to parse configuration"

**Problem:** TOML syntax error in `capsula.toml`.

**Solution:**

Check the error message for the line number and fix the syntax. Common issues:

- Missing closing brackets: `[[pre-run.hooks]` should be `[[pre-run.hooks]]`
- Invalid TOML syntax
- Typos in hook IDs

**Example error:**
```
Error: Failed to parse configuration file
  --> capsula.toml:5:1
  |
5 | [[pre-run.hooks]
  | ^^^^^^^^^^^^^^^^ Missing closing bracket
```

Add the missing `]`:
```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

---

### "Unknown hook ID"

**Problem:** Hook ID not recognized.

**Solution:**

Check the hook ID spelling. Valid IDs:

- `capture-cwd`
- `capture-env`
- `capture-git-repo`
- `capture-file`
- `capture-machine`
- `capture-command`
- `notify-slack`

Note the hyphens (`-`) not underscores (`_`).

## Runtime Issues

### Command Not Executing

**Problem:** `capsula run` doesn't execute your command.

**Solution:**

1. Check if a hook requested abort:
   ```bash
   cat .capsula/*/latest/_capsula/pre-run.json | grep "abort_requested"
   ```

2. Look for error messages in the output

3. Common causes:
   - Git repository is dirty with `allow_dirty = false`
   - A hook with `abort_on_failure = true` failed

---

### "Repository has uncommitted changes"

**Problem:** Git hook aborts because repository is dirty.

**Solution:**

Either:

1. **Commit your changes:**
   ```bash
   git add .
   git commit -m "Ready to run"
   ```

2. **Allow dirty repository:**
   ```toml
   [[pre-run.hooks]]
   id = "capture-git-repo"
   path = "."
   allow_dirty = true  # Add this
   ```

---

### Hooks Failing Silently

**Problem:** Hooks fail but you don't see errors.

**Solution:**

Check the hook output files:

```bash
# Check pre-run hooks
cat .capsula/*/latest/_capsula/pre-run.json | jq '.[] | select(.__ meta.success == false)'

# Check post-run hooks
cat .capsula/*/latest/_capsula/post-run.json | jq '.[] | select(.__meta.success == false)'
```

---

### Files Not Being Captured

**Problem:** `capture-file` hook isn't capturing files.

**Solution:**

1. **Check glob pattern:**
   ```bash
   # Test your glob pattern
   ls results/*.txt
   ```

2. **Check file exists when hook runs:**
   - Pre-run hooks run **before** your command
   - Post-run hooks run **after** your command

3. **Check hook output:**
   ```bash
   cat .capsula/*/latest/_capsula/post-run.json | jq '.[] | select(.__meta.id == "capture-file")'
   ```

4. **Check hook order** (if using `mode = "move"`):
   - Files might have been moved by an earlier hook

---

### Environment Variables Not Set

**Problem:** `capture-env` shows `null` for a variable.

**Solution:**

1. **Check the variable is set:**
   ```bash
   echo $MY_VAR
   ```

2. **Export the variable:**
   ```bash
   export MY_VAR="value"
   capsula run python script.py
   ```

3. **Use a `.env` file:**
   ```toml
   dotenv = ".env"
   ```

   ```bash title=".env"
   MY_VAR=value
   ```

## Slack Notification Issues

### "invalid_auth" or "not_authed"

**Problem:** Slack token is invalid.

**Solution:**

1. **Check token is set:**
   ```bash
   echo $SLACK_BOT_TOKEN
   ```

2. **Verify token format** (should start with `xoxb-`)

3. **Regenerate token:**
   - Go to [api.slack.com/apps](https://api.slack.com/apps)
   - Select your app
   - Go to "OAuth & Permissions"
   - Reinstall app to get new token

---

### "channel_not_found"

**Problem:** Bot can't access the channel.

**Solution:**

1. **Invite bot to channel:**
   ```
   /invite @YourBotName
   ```

2. **Use channel ID instead of name:**
   - Click channel name → View channel details
   - Copy the channel ID (e.g., `C01234567`)

   ```toml
   [[post-run.hooks]]
   id = "notify-slack"
   channel = "C01234567"  # Use ID instead of "#channel-name"
   ```

---

### "missing_scope"

**Problem:** Bot doesn't have required permissions.

**Solution:**

1. Go to [api.slack.com/apps](https://api.slack.com/apps)
2. Select your app
3. Go to "OAuth & Permissions"
4. Under "Scopes" → "Bot Token Scopes", add:
   - `chat:write`
   - `files:write` (if using attachments)
5. Click "Reinstall App"

---

### Files Not Attaching to Slack

**Problem:** `attachment_globs` specified but files not attached.

**Solution:**

1. **Check files exist:**
   ```bash
   ls results/*.png
   ```

2. **Check hook order:**
   - If using `capture-file` with `mode = "move"`, Slack hook must come **before**:

   ```toml
   # Correct order
   [[post-run.hooks]]
   id = "notify-slack"
   channel = "#results"
   attachment_globs = ["results/*.png"]

   [[post-run.hooks]]
   id = "capture-file"
   glob = "results/*.png"
   mode = "move"
   ```

3. **Check attachment limit** (max 10 files):
   ```bash
   # Count matching files
   ls results/*.png | wc -l
   ```

4. **Check hook output:**
   ```bash
   cat .capsula/*/latest/_capsula/post-run.json | jq '.[] | select(.__meta.id == "notify-slack") | .attachments'
   ```

## Performance Issues

### Slow Execution

**Problem:** Capsula runs slowly.

**Possible causes and solutions:**

1. **Large files being copied:**
   - Use `mode = "none"` with `hash = "sha256"` for large files
   - Or use `mode = "move"` instead of `mode = "copy"`

2. **Many hooks:**
   - Hooks run sequentially
   - Remove unnecessary hooks

3. **Slow commands in `capture-command` hooks:**
   - Avoid long-running commands in hooks
   - Run them as your main command instead

---

### Large Vault Directories

**Problem:** `.capsula/` directory is taking up too much space.

**Solution:**

1. **Use `mode = "none"` for large files:**
   ```toml
   [[post-run.hooks]]
   id = "capture-file"
   glob = "model.bin"
   mode = "none"  # Don't copy
   hash = "sha256"  # Just hash
   ```

2. **Clean old runs:**
   ```bash
   # Remove runs older than 30 days
   find .capsula/ -type d -mtime +30 -exec rm -rf {} +
   ```

3. **Move vaults to different location:**
   ```toml
   [vault]
   name = "experiments"
   path = "/mnt/large-disk/capsula-vaults"
   ```

## Git Hook Issues

### "Not a git repository"

**Problem:** `capture-git-repo` fails because directory isn't a git repo.

**Solution:**

1. **Initialize git:**
   ```bash
   git init
   git add .
   git commit -m "Initial commit"
   ```

2. **Or remove the hook** if you don't need git tracking

---

### Untracked Files Causing Dirty State

**Problem:** Repository is dirty because of untracked files.

**Solution:**

1. **Add files to `.gitignore`:**
   ```bash
   echo "temp_files/" >> .gitignore
   ```

2. **Commit or delete the files:**
   ```bash
   git add file.txt
   git commit -m "Add file"
   ```

3. **Allow dirty state:**
   ```toml
   [[pre-run.hooks]]
   id = "capture-git-repo"
   path = "."
   allow_dirty = true
   ```

## Server Issues

See the [Server Setup](server-setup.md#troubleshooting) page for server-specific troubleshooting.

## Getting More Help

If you're still stuck:

1. **Check debug output:**
   ```bash
   RUST_LOG=debug capsula run echo test
   ```

2. **Look at captured data:**
   ```bash
   # View pre-run hooks
   cat .capsula/*/latest/_capsula/pre-run.json | jq .

   # View command output
   cat .capsula/*/latest/_capsula/command.json | jq .

   # View post-run hooks
   cat .capsula/*/latest/_capsula/post-run.json | jq .
   ```

3. **Check the FAQ:**
   [Frequently Asked Questions](faq.md)

4. **Report an issue:**
   [GitHub Issues](https://github.com/ut-issl/capsula/issues)

   Include:
   - Capsula version: `capsula --version`
   - Your `capsula.toml` (remove sensitive data)
   - Error messages
   - Steps to reproduce

## Common Error Messages

### "No such file or directory"

```
Error: No such file or directory: config.yaml
```

**Cause:** File doesn't exist when hook runs.

**Solution:** Check file path and whether it exists at hook execution time.

---

### "Permission denied"

```
Error: Permission denied: /path/to/file
```

**Cause:** Capsula can't read/write the file.

**Solution:** Check file permissions:
```bash
ls -la /path/to/file
chmod +r /path/to/file  # Make readable
```

---

### "Address already in use" (Server)

```
Error: Address already in use (os error 48)
```

**Cause:** Port 3000 is already in use.

**Solution:** Use a different port:
```bash
PORT=8080 cargo run -p capsula-server
```

---

### "Command not found"

```
Error: Command not found: python
```

**Cause:** Command isn't in PATH.

**Solution:**

1. Use full path:
   ```toml
   command = ["/usr/bin/python", "--version"]
   ```

2. Or fix PATH:
   ```bash
   export PATH="/usr/local/bin:$PATH"
   capsula run python script.py
   ```

## Best Practices to Avoid Issues

1. **Start simple** - Add hooks gradually

2. **Test configuration** - Run simple commands first:
   ```bash
   capsula run echo "test"
   ```

3. **Check captured data** - Review output after each run

4. **Use descriptive vault names** - Makes debugging easier

5. **Keep `.env` out of git** - Add to `.gitignore`

6. **Review hook order** - Especially when using `mode = "move"`

7. **Use `allow_dirty = true` during development** - Switch to `false` for production

8. **Check disk space** - Vaults can grow large

## What's Next?

<div class="grid cards" markdown>

-   :material-frequently-asked-questions:{ .lg .middle } **FAQ**

    ---

    Frequently asked questions.

    [:octicons-arrow-right-24: Read FAQ](faq.md)

-   :material-book-open-variant:{ .lg .middle } **Examples**

    ---

    See working examples.

    [:octicons-arrow-right-24: View examples](examples.md)

-   :material-github:{ .lg .middle } **Report an Issue**

    ---

    Found a bug? Let us know!

    [:octicons-arrow-right-24: GitHub Issues](https://github.com/ut-issl/capsula/issues)

</div>
