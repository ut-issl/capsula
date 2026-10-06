---
icon: material/rocket-launch
---

# はじめての Capsula { #getting-started }

このガイドでは、Capsula で最初のランを作成する手順を通して、Capsula の仕組みを説明します。

!!! info "始める前に"
    [Capsula をインストール](installation.md)し、`capsula --version` を実行できることを確認してください。

## Capsula の仕組み { #how-capsula-works }

Capsula を通してコマンドを実行すると、次の順に処理が進みます。

1. **実行前フックが順に実行される** - 設定した実行前 (pre-run) フックを Capsula が順番に実行します
2. **コマンドが実行される** - Capsula がコマンドを通常どおり実行します
3. **実行後フックが順に実行される** - 設定した実行後 (post-run) フックを Capsula が順番に実行します

あるフックが失敗しても、Capsula はそのフェーズの残りのフックを実行し、すべての結果を記録します。実行前フックが失敗した場合、メインのコマンドは実行されません。実行後フックの失敗は、メインのコマンドが終了した後に報告されます。

すべての出力は、「ボールト (vault)」と呼ばれる構造化されたディレクトリに保存されます。

## 最初のラン { #your-first-run }

`capsula.toml` ファイルを作成します。

```toml title="capsula.toml"
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-project"
path = "."
allow_dirty = true

[[pre-run.hooks]]
id = "capture-cwd"

[[post-run.hooks]]
id = "capture-file"
glob = "*.txt"
mode = "copy"
```

コマンドを実行します。

```bash
capsula run echo "Hello, Capsula!" > output.txt
```

ランの一覧を表示します。

```bash
capsula list
```

出力:

```
TIMESTAMP (UTC)      NAME                  COMMAND
---------------------------------------------------------------------------------------------
2025-01-09 14:30:22  happy-river           echo "Hello, Capsula!" > output.txt
```

各ランには、識別しやすいようにタイムスタンプ (UTC) とランダムに生成された名前が付けられます。

## ボールトの中身を確認する { #exploring-the-vault }

Capsula はすべてのデータを `.capsula/` に保存します。

```
.capsula/my-project/2025-01-09/143022-happy-river/
├── _capsula/                   # Metadata directory
│   ├── metadata.json           # Run info (ID, name, command, timestamp)
│   ├── pre-run.json            # Pre-run hook outputs
│   ├── command.json            # Command execution results
│   └── post-run.json           # Post-run hook outputs
├── pre-0-capture-git-repo/     # Artifacts from the capture-git-repo hook
└── post-0-capture-file/        # Artifacts from the capture-file hook
    └── output.txt              # Your captured file
```

ファイルのアーティファクトを生成するフック (`capture-file` や `capture-git-repo` など) には、
それぞれ `{phase}-{index}-{hook_id}/` という名前の専用サブディレクトリが割り当てられます。
これにより、フック間でファイル名が衝突することを防ぎ、各フックの出力を分離して保持します。

実行前フックが記録した内容を確認します。

```bash
cat .capsula/my-project/2025-01-09/143022-happy-river/_capsula/pre-run.json
```

## 利用できるフック { #available-hooks }

| フック | 説明 | 主なフェーズ |
| ------ | ------------- | --------------- |
| [capture-cwd](hooks/capture-cwd.md) | カレントディレクトリを記録します | 実行前 |
| [capture-env](hooks/capture-env.md) | 環境変数を記録します | 実行前 |
| [capture-git-repo](hooks/capture-git-repo.md) | Git リポジトリの状態を記録します | 実行前 |
| [capture-file](hooks/capture-file.md) | ファイルを記録します (コピー/移動/ハッシュ) | 両方 |
| [capture-machine](hooks/capture-machine.md) | システム情報を記録します | 実行前 |
| [capture-command](hooks/capture-command.md) | コマンドを実行して出力を記録します | 両方 |
| [notify-slack](hooks/notify-slack.md) | Slack に通知を送信します | 両方 |

各フックの名前をクリックすると、詳しい設定オプションと例を確認できます。

## 環境変数 { #environment-variables }

Capsula はコマンドの実行時に次の環境変数を設定します。

- `CAPSULA_RUN_ID` - 一意な識別子
- `CAPSULA_RUN_NAME` - 人間が読みやすい名前 (例: 「happy-river」)
- `CAPSULA_RUN_DIRECTORY` - ランディレクトリのパス
- `CAPSULA_RUN_TIMESTAMP` - ISO 8601 形式のタイムスタンプ (UTC)
- `CAPSULA_RUN_COMMAND` - 実行されるコマンド

これらの環境変数はコマンドの中で使用できます。

```bash
capsula run bash -c 'echo "Run: $CAPSULA_RUN_NAME"'
```
