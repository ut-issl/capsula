# notify-slack

Sends notifications to Slack channels using Block Kit with optional file attachments.

## Configuration

```toml
# Simple notification
[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"

# With file attachments
[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
attachment_globs = ["*.png", "results/*.jpg"]

# With custom token
[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
token = "xoxb-your-bot-token"
attachment_globs = ["outputs/**/*.pdf"]
```

## Parameters

- `channel` (required): Slack channel ID (e.g., "C1234567890")
- `token` (optional): Slack bot token (defaults to `SLACK_BOT_TOKEN` environment variable)
- `attachment_globs` (optional): Array of glob patterns for files to attach (up to 10 files)

## Phases

- ❌ Pre-run (not recommended)
- ✅ Post-run (typical usage)

## Output

```json
{
  "__meta": {
    "id": "notify-slack",
    "config": {
      "channel": "C1234567890",
      "token": "xoxb-***",
      "attachment_globs": ["*.png"]
    },
    "success": true
  },
  "message": "Slack notification sent successfully",
  "response": "{\"ok\":true,\"channel\":\"C1234567890\",\"ts\":\"1234567890.123456\"}",
  "attached_files": ["plot.png", "results.png"]
}
```

### Fields

- `message` (string): Status message
- `response` (string, optional): Slack API response JSON
- `attached_files` (array, optional): List of files that were attached

## Setup

### 1. Create Slack App

1. Go to <https://api.slack.com/apps>
2. Click "Create New App"
3. Choose "From scratch"
4. Name your app (e.g., "Capsula Bot")
5. Select your workspace

### 2. Add Bot Token Scopes

1. Navigate to "OAuth & Permissions"
2. Under "Scopes" → "Bot Token Scopes", add:
   - `chat:write` - Required for sending messages
   - `files:write` - Required for uploading files (if using attachments)
3. Click "Install to Workspace"
4. Copy the "Bot User OAuth Token" (starts with `xoxb-`)

### 3. Get Channel ID

To find your channel ID:

1. Open Slack in a web browser
2. Navigate to the channel
3. Copy the ID from the URL: `https://app.slack.com/client/T.../C1234567890`
   - The part after the last slash is your channel ID (e.g., `C1234567890`)

Alternatively, right-click the channel → "View channel details" → at the bottom you'll see the channel ID.

### 4. Store Bot Token

Store the bot token in an environment variable:

```bash
export SLACK_BOT_TOKEN="xoxb-your-bot-token-here"
```

For persistent storage, add to your shell profile:

```bash
# ~/.bashrc or ~/.zshrc
export SLACK_BOT_TOKEN="xoxb-..."
```

### 5. Invite Bot to Channel

In the Slack channel, type:

```
/invite @Capsula Bot
```

Replace "Capsula Bot" with your bot's name.

### 6. Configure Capsula

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"  # Your channel ID
```

## Message Format

The hook automatically creates a rich Slack message using Block Kit with the following information:

**Pre-run phase:**

- 🚀 Header: "Capsula Run Starting"
- Run name, ID, timestamp, and command

**Post-run phase:**

- ✅ Header: "Capsula Run Completed"
- Run name, ID, timestamp, and command

You don't need to configure the message content - it's automatically generated from the run metadata.

## Use Cases

### Long-Running Experiments

Get notified when experiments complete:

```toml
[vault]
name = "training-runs"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "training-repo"
path = "."

[[post-run.hooks]]
id = "capture-file"
glob = "results/metrics.json"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
```

### Share Results with Attachments

Automatically share plots and results:

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "plots/*.png"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
attachment_globs = ["plots/*.png", "results/summary.txt"]
```

### Multiple Channel Notifications

Notify different channels for different purposes:

```toml
# Notify team channel
[[post-run.hooks]]
id = "notify-slack"
channel = "C1111111111"  # #team-experiments

# Notify personal channel with attachments
[[post-run.hooks]]
id = "notify-slack"
channel = "C2222222222"  # #my-experiments
attachment_globs = ["*.png"]
```

## Examples

### Basic Notification

```toml
[vault]
name = "experiments"

[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
```

```bash
capsula run python experiment.py
```

### Notification with Attachments

```toml
[vault]
name = "ml-experiments"

[[post-run.hooks]]
id = "capture-file"
glob = "plots/*.png"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
attachment_globs = ["plots/*.png", "results/*.csv"]
```

### Development vs Production Channels

Use different channels for different environments:

```toml
# Development experiments - personal channel
[[post-run.hooks]]
id = "notify-slack"
channel = "C1111111111"  # Your personal channel

# Production runs - team channel with attachments
[[post-run.hooks]]
id = "notify-slack"
channel = "C2222222222"  # Team channel
attachment_globs = ["metrics.json", "model_summary.txt"]
```

## File Attachments

### Glob Patterns

The `attachment_globs` parameter accepts glob patterns:

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "C1234567890"
attachment_globs = [
    "*.png",              # All PNG files in project root
    "results/*.json",     # All JSON files in results/
    "outputs/**/*.pdf",   # All PDF files under outputs/ recursively
]
```

### File Limit

- Maximum of 10 files per notification (Slack API limit)
- If more than 10 files match, only the first 10 are attached
- Files are resolved relative to the project root

### Attachment Behavior

When attachments are specified:

1. Files matching the glob patterns are uploaded to Slack
2. A main message is posted with run details
3. File links are posted as a threaded reply
4. The thread reply is broadcast to the channel so all members can see it

## Error Handling

### Bot Token Not Found

```json
{
  "__meta": {
    "id": "notify-slack",
    "success": false,
    "error": "Missing Slack bot token: SLACK_BOT_TOKEN environment variable not set"
  }
}
```

### Network Failure

```json
{
  "__meta": {
    "id": "notify-slack",
    "success": false,
    "error": "Failed to send notification: Connection refused"
  }
}
```

### Invalid Token or Missing Scopes

```json
{
  "__meta": {
    "id": "notify-slack",
    "success": false,
    "error": "Failed to upload file: missing_scope. The Slack bot needs the 'files:write' OAuth scope."
  }
}
```

If you see a `missing_scope` error, go to your Slack app settings → OAuth & Permissions → Bot Token Scopes and add the required scope (`chat:write` or `files:write`), then reinstall the app to the workspace.

### Bot Not in Channel

If the bot hasn't been invited to the channel, you'll see an error. Make sure to invite the bot:

```
/invite @Capsula Bot
```

### Non-Fatal Behavior

Notification failures are non-fatal:

1. Error is logged
2. Error recorded in JSON output
3. Execution continues

Your command results are still captured even if notifications fail.

## Security Considerations

!!! warning "Bot Token Security"
    Bot tokens are sensitive credentials. Anyone with a bot token can act as your bot.

Best practices:

1. **Never commit bot tokens to version control**
2. **Use environment variables** (not hardcoded values in config)
3. **Use workspace-specific tokens** - don't share tokens across workspaces
4. **Rotate tokens** if compromised
5. **Use separate bots** for dev/prod environments
6. **Limit bot permissions** - only add the scopes you need

### Example .gitignore

```
# .gitignore
.env
capsula.local.toml
**/SLACK_BOT_TOKEN
```

### Token Storage

```bash
# Good - environment variable
export SLACK_BOT_TOKEN="xoxb-..."

# Bad - in config file (committed to git)
token = "xoxb-..."  # DON'T DO THIS
```

## Rate Limiting

Slack imposes rate limits on API calls:

- **Tier 1 methods**: 1+ requests per minute
- **File uploads**: Can be slower for large files

If you're running many experiments rapidly, consider:

1. Only notifying important runs
2. Using separate channels for high-frequency notifications
3. Implementing delay between experiments

## See Also

- [Slack API Documentation](https://api.slack.com/)
- [Slack Block Kit](https://api.slack.com/block-kit)
- [Slack File Uploads](https://api.slack.com/methods/files.upload)
- [capture-file](capture-file.md) - Capture files for attachments
- [capture-command](capture-command.md) - Run commands before notifications
