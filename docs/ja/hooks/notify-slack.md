---
icon: material/hook
---

# notify-slack { #notify-slack }

ランの開始時 (実行前) または完了時 (実行後) に Slack へ通知を送ります。ファイルを添付することもできます。

## ユースケース { #use-cases }

- 長時間かかる実験の完了通知を受け取る
- 結果をチームのチャンネルに自動で共有する
- 複数のマシンにまたがる実験の進行状況を監視する
- グラフや結果を通知に添付する

## セットアップ要件 { #setup-requirements }

このフックを使う前に、Slack アプリをセットアップする必要があります:

### 1. Slack アプリを作成する { #1-create-a-slack-app }

1. [api.slack.com/apps](https://api.slack.com/apps) を開きます
2. 「Create New App」→「From scratch」をクリックします
3. 名前 (例: 「Capsula Bot」) を付け、ワークスペースを選択します

### 2. 権限を追加する { #2-add-permissions }

1. 「OAuth & Permissions」を開きます
2. 「Bot Token Scopes」に次のスコープを追加します:
   - `chat:write` - メッセージの投稿
   - `files:write` - ファイルのアップロード (添付を使う場合)
3. 「Install to Workspace」をクリックします
4. 「Bot User OAuth Token」(`xoxb-` で始まります) をコピーします

### 3. ボットをチャンネルに招待する { #3-invite-bot-to-channel }

Slack のチャンネルで次のように入力します:

```
/invite @YourBotName
```

### 4. 環境変数を設定する { #4-set-environment-variable }

```bash
export SLACK_BOT_TOKEN="xoxb-your-token-here"
```

または `.env` ファイルを使います:

```bash title=".env"
SLACK_BOT_TOKEN=xoxb-your-token-here
```

```toml title="capsula.toml"
dotenv = ".env"
```

!!! warning "セキュリティ"
    Slack のトークンは決してバージョン管理にコミットしないでください。`.env` を `.gitignore` に追加してください。

## 設定 { #configuration }

### 必須オプション { #required-options }

| オプション | 型 | 説明 |
| -------- | ------ | ------------- |
| `channel` | string | Slack のチャンネル名 (例: `"#general"`) またはチャンネル ID (例: `"C01234567"`) |

### 任意オプション { #optional-options }

| オプション | 型 | デフォルト | 説明 |
| -------- | ------ | --------- | ------------- |
| `token` | string | 環境変数 `SLACK_BOT_TOKEN` | Slack のボットトークン (`xoxb-` で始まります) |
| `attachment_globs` | 文字列の配列 | `[]` | 添付するファイルのパターン (最大 10 ファイル) |

### 例 { #example }

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png"]
```

## 出力例 { #output-example }

### 通知に成功した場合 { #successful-notification }

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

## メッセージの形式 { #message-format }

### 実行前のメッセージ { #pre-run-message }

```
🚀 Capsula Run Starting

Run Name: happy-river
Run ID: 01K8WSYC91YAE21R7CWHQ4KYN2
Timestamp: Jan 9, 2025 at 2:30 PM (UTC)
Command: python train.py --epochs 100
```

### 実行後のメッセージ { #post-run-message }

```
✅ Capsula Run Completed

Run Name: happy-river
Run ID: 01K8WSYC91YAE21R7CWHQ4KYN2
Timestamp: Jan 9, 2025 at 2:30 PM (UTC)
Command: python train.py --epochs 100
```

## ファイルの添付 { #file-attachments }

### 例 { #example_1 }

```toml
[[post-run.hooks]]
id = "notify-slack"
channel = "#results"
attachment_globs = ["results/*.png", "plots/*.pdf", "summary.txt"]
```

### 制限 { #limits }

- 1 メッセージあたり最大 10 ファイル (Slack API の制限)
- 10 を超えるファイルが一致した場合は、最初の 10 ファイルだけが添付されます

### glob パターン { #glob-patterns }

[capture-file](capture-file.md) と同じです:

- `*.png` - カレントディレクトリ内のすべての `.png` ファイル
- `results/**/*.csv` - results/ 以下のすべての CSV
- `plot_?.pdf` - plot_1.pdf、plot_2.pdf など

## 添付を使う場合のフックの順序 { #hook-order-with-attachments }

!!! warning "重要"
    `capture-file` を `mode = "move"` で使う場合、Slack フックはファイルのフックより**前**に置く必要があります。

### ❌ 誤った順序 { #wrong-order }

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

### ✅ 正しい順序 { #correct-order }

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

## トラブルシューティング { #troubleshooting }

### "channel_not_found" { #channel_not_found }

ボットをチャンネルに招待してください: `/invite @YourBotName`

### "invalid_auth" または "not_authed" { #invalid_auth-or-not_authed }

環境変数 `SLACK_BOT_TOKEN` が設定されていて、`xoxb-` で始まっていることを確認してください。

### "missing_scope" { #missing_scope }

`chat:write` と `files:write` のスコープを追加してから、アプリを再インストールしてください。

### ファイルが添付されない { #files-not-attaching }

- ファイルパスが正しいか確認してください
- フックの実行時にファイルが存在するか確認してください
- `capture-file` を `mode = "move"` で使っている場合は、フックの順序を確認してください
