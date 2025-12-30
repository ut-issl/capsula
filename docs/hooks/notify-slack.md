# notify_slack

Sends notifications to Slack channels via webhooks.

## Configuration

```toml
# Simple notification
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = "Experiment completed!"

# With template variables
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = "Run {run_name} (ID: {run_id}) completed at {timestamp}"
```

## Parameters

- `webhook_url_env` (required): Name of environment variable containing the Slack webhook URL
- `message` (optional): Message text to send (supports template variables)

## Phases

- ❌ Pre-run (not recommended)
- ✅ Post-run (typical usage)

## Output

```json
{
  "__meta": {
    "id": "notify_slack",
    "config": {
      "webhook_url_env": "SLACK_WEBHOOK_URL",
      "message": "Run chubby-back completed!"
    },
    "success": true
  },
  "status": "sent",
  "message": "Run chubby-back completed!",
  "response_code": 200
}
```

### Fields

- `status` (string): Notification status ("sent" or "failed")
- `message` (string): Actual message that was sent
- `response_code` (number): HTTP response code from Slack API

## Setup

### 1. Create Slack Webhook

1. Go to <https://api.slack.com/messaging/webhooks>
2. Click "Create your Slack app"
3. Choose "From scratch"
4. Name your app (e.g., "Capsula Notifications")
5. Select your workspace
6. Navigate to "Incoming Webhooks"
7. Activate incoming webhooks
8. Click "Add New Webhook to Workspace"
9. Select the channel to post to
10. Copy the webhook URL

### 2. Store Webhook URL

Store the webhook URL in an environment variable:

```bash
export SLACK_WEBHOOK_URL="https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXX"
```

For persistent storage, add to your shell profile:

```bash
# ~/.bashrc or ~/.zshrc
export SLACK_WEBHOOK_URL="https://hooks.slack.com/services/..."
```

Or use a `.env` file with `dotenv`:

```bash
# .env
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/...
```

### 3. Configure Capsula

```toml
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = "Experiment completed!"
```

## Template Variables

The `message` field supports template variables:

| Variable | Description | Example |
| ---------- | ------------- | --------- |
| `{run_id}` | Unique run identifier (ULID) | `01HQZX3Y2J8K9M6N7P8Q9R` |
| `{run_name}` | Human-readable run name | `chubby-back` |
| `{command}` | Command that was executed | `python train.py` |
| `{timestamp}` | ISO 8601 timestamp | `2025-12-30T14:30:22Z` |

### Example with Variables

```toml
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = """
:rocket: Experiment Complete!
• Run: {run_name}
• ID: {run_id}
• Command: {command}
• Completed: {timestamp}
"""
```

Slack message:

```
🚀 Experiment Complete!
• Run: chubby-back
• ID: 01HQZX3Y2J8K9M6N7P8Q9R
• Command: python train.py --epochs 100
• Completed: 2025-12-30T14:30:22Z
```

## Use Cases

### Long-Running Experiments

Get notified when experiments complete:

```toml
[vault]
name = "training-runs"

[[pre_run]]
type = "capture_git_repo"

[[post_run]]
type = "capture_file"
path = "results/metrics.json"
copy = true

[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = ":white_check_mark: Training run {run_name} completed!"
```

### Team Notifications

Alert team members about important runs:

```toml
[[post_run]]
type = "notify_slack"
webhook_url_env = "TEAM_SLACK_WEBHOOK"
message = "@channel Production model training completed: {run_name}"
```

### Experiment Tracking

Track experiment progress remotely:

```toml
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = """
Experiment {run_name} finished
Command: {command}
Time: {timestamp}
Check results at: .capsula/experiments/{run_name}/
"""
```

## Examples

### Basic Notification

```toml
[vault]
name = "experiments"

[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = "Experiment completed successfully!"
```

```bash
capsula run python experiment.py
```

### Rich Notification

```toml
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_WEBHOOK_URL"
message = """
:chart_with_upwards_trend: Experiment Results

*Run*: {run_name}
*Command*: `{command}`
*Timestamp*: {timestamp}

View details: `.capsula/vault/2025-12-30/{run_name}/`
"""
```

### Conditional Notifications

Use multiple webhook URLs for different channels:

```toml
# Success notification
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_SUCCESS_WEBHOOK"
message = ":white_check_mark: Run {run_name} succeeded"

# Also notify failures channel (you would check logs)
[[post_run]]
type = "notify_slack"
webhook_url_env = "SLACK_MONITORING_WEBHOOK"
message = "Run {run_name} completed at {timestamp}"
```

## Slack Formatting

Slack supports markdown-like formatting:

```toml
message = """
*Bold text*
_Italic text_
~Strikethrough~
`Code`
```code block```
> Quote
"""
```

### Emoji

Use emoji shortcodes:

```toml
message = ":rocket: Launch successful :white_check_mark:"
```

Common emoji:

- `:white_check_mark:` ✅
- `:x:` ❌
- `:rocket:` 🚀
- `:chart_with_upwards_trend:` 📈
- `:warning:` ⚠️
- `:tada:` 🎉

### Mentions

Mention users or channels:

```toml
message = "<@U123456> Your experiment completed!"
message = "<!channel> Production model ready"
message = "<!here> Results available"
```

## Error Handling

### Webhook URL Not Found

```json
{
  "__meta": {
    "id": "notify_slack",
    "success": false,
    "error": "Environment variable not found: SLACK_WEBHOOK_URL"
  }
}
```

### Network Failure

```json
{
  "__meta": {
    "id": "notify_slack",
    "success": false,
    "error": "Failed to send notification: Connection refused"
  }
}
```

### Invalid Webhook

```json
{
  "__meta": {
    "id": "notify_slack",
    "success": false,
    "error": "Slack API error: invalid_token (HTTP 403)"
  },
  "status": "failed",
  "response_code": 403
}
```

### Non-Fatal Behavior

Notification failures are non-fatal:

1. Error is logged
2. Error recorded in JSON output
3. Execution continues

Your command results are still captured even if notifications fail.

## Security Considerations

!!! warning "Webhook URL Security"
    Webhook URLs are sensitive. Anyone with a webhook URL can post to your Slack channel.

Best practices:

1. **Never commit webhook URLs to version control**
2. **Use environment variables** (not hardcoded values)
3. **Restrict webhook permissions** to specific channels
4. **Rotate webhooks** if compromised
5. **Use separate webhooks** for dev/prod environments

### Example .gitignore

```
# .gitignore
.env
capsula.local.toml
**/SLACK_WEBHOOK_URL
```

## Rate Limiting

Slack imposes rate limits on webhook calls:

- **1 message per second** per webhook URL
- Exceeding limits results in `429 Too Many Requests`

If you're running many experiments rapidly, consider:

1. Batching notifications
2. Using separate webhook URLs
3. Implementing delay between experiments

## See Also

- [Slack Webhook Documentation](https://api.slack.com/messaging/webhooks)
- [Slack Message Formatting](https://api.slack.com/reference/surfaces/formatting)
- [capture_command](capture-command.md) - Run commands for more complex notifications
