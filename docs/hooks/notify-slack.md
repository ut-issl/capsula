# notify-slack

Sends notifications to Slack when runs start (pre-run) or complete (post-run), with optional file attachments.

## Use Cases

- Get notified when long-running experiments complete
- Share results automatically with team channels
- Monitor experiment progress across machines
- Attach plots and results to notifications

## Setup Requirements

Before using this hook, you need to set up a Slack app:

### 1. Create a Slack App

1. Go to [api.slack.com/apps](https://api.slack.com/apps)
2. Click "Create New App" → "From scratch"
3. Give it a name (e.g., "Capsula Bot") and select your workspace

### 2. Add Permissions

1. Go to "OAuth & Permissions"
2. Under "Bot Token Scopes", add:
   - `chat:write` - Post messages
   - `files:write` - Upload files (if using attachments)
3. Click "Install to Workspace"
4. Copy the "Bot User OAuth Token" (starts with `xoxb-`)

### 3. Invite Bot to Channel

In your Slack channel, type:
```
/invite @YourBotName
```

### 4. Set Environment Variable

```bash
export SLACK_BOT_TOKEN="xoxb-your-token-here"
```

Or use a `.env` file:

```bash title=".env"
SLACK_BOT_TOKEN=xoxb-your-token-here
```

```toml title="capsula.toml"
dotenv = ".env"
```

!!! warning "Security"
    Never commit your Slack token to version control! Add `.env` to `.gitignore`.

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
  "response": "{\"ok\":true,\"channel\":\"C01234567\",\"ts\":\"1234567890.123456\"}",
  "attachments": [
    "/path/to/results/plot1.png",
    "/path/to/results/plot2.png"
  ]
}
```

## Message Format

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

## File Attachments

### Example

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#results"
attachment_globs = ["results/*.png", "plots/*.pdf", "summary.txt"]
```

### Limits

- Maximum 10 files per message (Slack API limit)
- If more than 10 files match, only the first 10 are attached

### Glob Patterns

Same as [capture-file](capture-file.md):

- `*.png` - All PNGs in current directory
- `results/**/*.csv` - All CSVs in results/ tree
- `plot_?.pdf` - plot_1.pdf, plot_2.pdf, etc.

## Hook Order with Attachments

!!! warning "Important"
    If using `capture-file` with `mode = "move"`, the Slack hook must come **before** the file hook.

### ❌ Wrong Order

```toml
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
[[post-run.hooks]]
id = "notify-slack"
channel = "#reports"
attachment_globs = ["report.pdf"]

[[post-run.hooks]]
id = "capture-file"
glob = "report.pdf"
mode = "move"
```

## Troubleshooting

### "channel_not_found"

Invite the bot to the channel: `/invite @YourBotName`

### "invalid_auth" or "not_authed"

Check `SLACK_BOT_TOKEN` environment variable is set and starts with `xoxb-`

### "missing_scope"

Add `chat:write` and `files:write` scopes, then reinstall the app

### Files Not Attaching

- Check file paths are correct
- Verify files exist when hook runs
- Check hook order if using `capture-file` with `mode = "move"`
