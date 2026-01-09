# notify-slack

Sends notifications to Slack when runs start (pre-run) or complete (post-run), with optional file attachments.

## Use Cases

- **Long-running experiments** - Get notified when training completes
- **Team collaboration** - Share results automatically with team channels
- **Monitoring** - Track experiment progress across machines
- **Result sharing** - Attach plots and results to notifications

## Configuration

### Required Options

| Option | Type | Description |
|--------|------|-------------|
| `channel` | string | Slack channel name (e.g., `"#general"`) or channel ID (e.g., `"C01234567"`) |

### Optional Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `token` | string | `SLACK_BOT_TOKEN` env var | Slack bot token (starts with `xoxb-`) |
| `attachment_globs` | array of strings | `[]` | File patterns to attach (up to 10 files) |

### Example

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png"]
```

## Setup Requirements

Before using this hook, you need to set up a Slack app:

### 1. Create a Slack App

1. Go to [api.slack.com/apps](https://api.slack.com/apps)
2. Click "Create New App"
3. Choose "From scratch"
4. Give it a name (e.g., "Capsula Bot")
5. Select your workspace

### 2. Add Permissions

1. Go to "OAuth & Permissions"
2. Under "Scopes" → "Bot Token Scopes", add:
   - `chat:write` - Post messages
   - `files:write` - Upload files (if using attachments)
3. Click "Install to Workspace"
4. Copy the "Bot User OAuth Token" (starts with `xoxb-`)

### 3. Invite Bot to Channel

For channel notifications:

1. Go to the Slack channel
2. Type `/invite @YourBotName`

For DM notifications:

1. Open a DM with the bot
2. Click the bot's name to see the channel ID (e.g., `D01234ABCD`)

### 4. Set Environment Variable

Store the token securely:

```bash
export SLACK_BOT_TOKEN="xoxb-your-token-here"
```

Or use a `.env` file:

```bash title=".env"
SLACK_BOT_TOKEN=xoxb-your-token-here
```

```toml title="capsula.toml"
dotenv = ".env"

[vault]
name = "experiments"

[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
```

!!! warning "Security"
    Never commit your Slack token to version control! Add `.env` to `.gitignore`.

## Output Example

### Successful Notification

```json
{
  "__meta": {
    "id": "notify-slack",
    "config": {
      "channel": "C01234567",
      "attachment_globs": ["results/*.png"]
    },
    "success": true
  },
  "message": "Slack notification sent successfully",
  "response": "{\"ok\":true,\"channel\":\"C01234567\",\"ts\":\"1234567890.123456\",\"files\":[...]}",
  "attachments": [
    "/path/to/results/plot1.png",
    "/path/to/results/plot2.png"
  ]
}
```

### Failed Notification

```json
{
  "__meta": {
    "id": "notify-slack",
    "success": false,
    "error": "Slack API error: channel_not_found"
  }
}
```

## Message Format

Notifications use Slack's Block Kit for rich formatting:

### Pre-Run Message

```
🚀 Capsula Run Starting

Run Name: happy-river
Run ID: 01K8WSYC91YAE21R7CWHQ4KYN2
Timestamp: Jan 9, 2025 at 2:30 PM (UTC)
Command: python train.py --epochs 100
```

### Post-Run Message

```
✅ Capsula Run Completed

Run Name: happy-river
Run ID: 01K8WSYC91YAE21R7CWHQ4KYN2
Timestamp: Jan 9, 2025 at 2:30 PM (UTC)
Command: python train.py --epochs 100
```

Messages appear similar to GitHub's Slack notifications for better readability.

## File Attachments

### Basic Attachment

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#results"
attachment_globs = ["output.txt"]
```

### Multiple Patterns

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png", "plots/*.pdf", "summary.txt"]
```

### Attachment Limits

- **Maximum 10 files** per message (Slack API limit)
- If more than 10 files match, only the first 10 are attached
- Files are uploaded and shared to the channel

### Glob Patterns

Same as [capture-file](capture-file.md) glob patterns:

```toml
attachment_globs = [
  "*.png",              # All PNGs in current dir
  "results/**/*.csv",   # All CSVs in results/ tree
  "plot_?.pdf"          # plot_1.pdf, plot_2.pdf, etc.
]
```

## Complete Examples

### Notify on Completion

```toml title="capsula.toml"
[vault]
name = "experiments"

[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
```

### Notify with Attachments

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#results"
attachment_globs = [
  "results/*.png",
  "summary.txt",
  "metrics.json"
]
```

### Notify Start and End

```toml
# Notify when starting
[[pre-run.hooks]]
id = "notify-slack"
channel = "#experiments"

# Notify when done (with results)
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png"]
```

### Direct Message (DM)

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "D01234ABCD"  # DM channel ID
```

To find your DM channel ID:

1. Open a DM with the bot
2. Click the bot's name
3. Look for "Channel ID" in the details

## Hook Order with Attachments

!!! warning "Important: Hook Order Matters"
    If using `capture-file` with `mode = "move"`, the Slack hook must come **before** the file hook.

### ❌ Wrong Order

```toml
# File is moved before Slack can attach it
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "move"

[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]  # File already moved!
```

### ✅ Correct Order

```toml
# Slack attaches file first
[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]

# Then file is moved
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "move"
```

### ✅ Alternative: Use Copy

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "copy"  # File stays available

[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]
```

## Common Patterns

### Pattern: ML Training Notification

```toml
dotenv = ".env"

[vault]
name = "ml-experiments"

[[pre-run.hooks]]
id = "notify-slack"
channel = "#ml-training"

[[post-run.hooks]]
id = "notify-slack"
channel = "#ml-training"
attachment_globs = [
  "training_curves.png",
  "metrics.json",
  "model_summary.txt"
]
```

### Pattern: Daily Build Notification

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#builds"
attachment_globs = ["build_log.txt"]
```

### Pattern: Error Reports

```toml
# Only send if there are errors
[[post-run.hooks]]
id = "capture-command"
command = ["test", "-f", "error.log"]

[[post-run.hooks]]
id = "notify-slack"
channel = "#errors"
attachment_globs = ["error.log"]
```

### Pattern: Multiple Channels

```toml
# Notify team channel
[[post-run.hooks]]
id = "notify-slack"
channel = "#team-experiments"

# Also notify yourself
[[post-run.hooks]]
id = "notify-slack"
channel = "D01234ABCD"  # Your DM
attachment_globs = ["results/*.png"]
```

## Tips

### Use Environment Variables

Store tokens in environment variables, not in config:

```toml
# Good: token from environment
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"

# Bad: token in config
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
token = "xoxb-..."  # Don't do this!
```

### Test in DMs First

Test notifications in a DM before sending to team channels:

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "D01234ABCD"  # Your DM
```

### Include Useful Attachments

Attach summaries and visualizations, not raw data:

```toml
attachment_globs = [
  "summary.txt",      # ✅ Good
  "plots/*.png",      # ✅ Good
  "data.csv"          # ⚠️ Maybe too large
]
```

### Limit File Size

Slack has file size limits (~1GB per file, but smaller is better):

- Attach summaries, not full datasets
- Use compressed formats (PNG not BMP)
- Consider attaching links instead of large files

## Troubleshooting

### "channel_not_found"

**Problem:** Bot can't access the channel.

**Solution:**

- Invite the bot to the channel: `/invite @YourBotName`
- Check channel name spelling
- Use channel ID instead of name

### "invalid_auth" or "not_authed"

**Problem:** Token is invalid or missing.

**Solution:**

- Check `SLACK_BOT_TOKEN` environment variable
- Verify token starts with `xoxb-`
- Regenerate token if needed

### "missing_scope"

**Problem:** Bot doesn't have required permissions.

**Solution:**

- Add `chat:write` scope
- Add `files:write` scope (for attachments)
- Reinstall app to workspace

### Files Not Attaching

**Problem:** Files specified in `attachment_globs` aren't attached.

**Solution:**

- Check file paths are relative to project root
- Verify files exist when hook runs
- Check hook order (files might have been moved)
- Look at `attachments` field in hook output to see what was found

### Messages Not Formatted

**Problem:** Messages appear as plain text.

**Solution:** This is normal - Capsula uses Block Kit which Slack renders automatically. The formatting appears in Slack, not in the raw JSON.

## Common Questions

**Q: Can I customize the message text?**

Not currently. The message format is fixed to ensure consistency. However, the message includes run name, ID, timestamp, and command.

**Q: Can I send to multiple channels?**

Yes, use multiple hooks:

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"

[[post-run.hooks]]
id = "notify-slack"
channel = "#team"
```

**Q: What if my bot token expires?**

Bot tokens from Slack apps don't expire unless you explicitly revoke them. If revoked, regenerate the token and update your environment variable.

**Q: Can I @mention people in notifications?**

Not currently. The message format is fixed and doesn't support mentions.

**Q: Does this work with Slack Enterprise Grid?**

Yes, as long as your bot has the required permissions.

**Q: What if files are too large?**

Slack's file size limit is around 1GB per file, but uploading very large files will slow down your run. Consider:

- Compressing files before uploading
- Uploading summaries instead of full data
- Using separate file upload services for large files

**Q: Can I use webhooks instead of a bot token?**

Not currently. This hook uses the Slack Web API which requires a bot token.

## Related Hooks

- [capture-file](capture-file.md) - Capture files before attaching
- [capture-command](capture-command.md) - Generate summaries to attach

[:octicons-arrow-left-24: Back to Hooks](../hooks.md)
