---
icon: material/console
---

# CLI リファレンス { #cli-reference }

Capsula のコマンドラインインターフェースの完全なリファレンスです。

## グローバルオプション { #global-options }

### `--config <PATH>` { #-config-path }

独自の設定ファイルを指定します。

```bash
capsula --config /path/to/custom.toml run python script.py
```

デフォルト: カレントディレクトリと親ディレクトリから `capsula.toml` を探します。

### `--vault-path <PATH>` { #-vault-path-path }

ボールトのパスを上書きします。`CAPSULA_VAULT_PATH` 環境変数でも指定できます (dotenv の読み込み後に参照されます)。

```bash
capsula --vault-path /custom/vault/path list
```

### `--version` { #-version }

Capsula のバージョンを表示します。

```bash
capsula --version
```

### `--help` { #-help }

ヘルプを表示します。

```bash
capsula --help
capsula run --help
```

## `capsula run` { #capsula-run }

フックによる記録を行いながらコマンドを実行します。

### 使い方 { #usage }

```bash
capsula run <COMMAND> [ARGS...]
```

### 例 { #examples }

```bash
capsula run echo "Hello, World!"
capsula run python train.py --epochs 100
capsula run bash -c 'python generate.py | grep result > output.txt'
```

### 環境変数 { #environment-variables }

コマンドは次の環境変数が設定された状態で実行されます。

| 変数 | 説明 | 例 |
| ---------- | ------------- | --------- |
| `CAPSULA_RUN_ID` | ランの一意な識別子 (ULID) | `01K8WSYC91YAE21R7CWHQ4KYN2` |
| `CAPSULA_RUN_NAME` | 人が読みやすいランの名前 | `happy-river` |
| `CAPSULA_RUN_DIRECTORY` | ランディレクトリの絶対パス | `/path/.capsula/vault/2025-01-09/143022-happy-river` |
| `CAPSULA_RUN_TIMESTAMP` | ISO 8601 形式のタイムスタンプ (UTC) | `2025-01-09T14:30:22.473+00:00` |
| `CAPSULA_RUN_COMMAND` | シェル向けにクォートされたコマンド文字列 | `python train.py --epochs 100` |
| `CAPSULA_PRE_RUN_OUTPUT_PATH` | pre-run.json のパス | `/path/.capsula/.../pre-run.json` |
| `CAPSULA_PROJECT_ROOT` | プロジェクトルートのディレクトリ | `/path/to/project` |

環境変数を使う例です。

```bash
capsula run bash -c 'echo "Run: $CAPSULA_RUN_NAME"'
```

## `capsula run-start` { #capsula-run-start }

手動のランを開始します。ランディレクトリを作成し、実行前フックを実行します。

capsula が直接管理しない外部プロセス (GUI から起動する解析など) の前後でコンテキストを記録したいときに使用します。
自動生成されたランの名前が標準出力に出力されるので、呼び出し側はそれを受け取り、
後で `run-end` によってランを完了させます。

### 使い方 { #usage_1 }

```bash
capsula run-start
```

### 例 { #example }

```bash
# Start a manual run and capture the name
name=$(capsula run-start)

# ... perform your external process ...

# Finalize the run
capsula run-end "$name"
```

## `capsula run-end` { #capsula-run-end }

手動のランを終了します。既存のランに対して実行後フックを実行します。

`run-start` で開始したランを完了させます。このコマンドは、ランがすでに完了している場合
(つまり `post-run.json` がすでに存在する場合)、または `post-run.json` の書き出し後に
いずれかの実行後フックが失敗した場合に失敗します。

### 使い方 { #usage_2 }

```bash
capsula run-end <RUN_NAME>
```

### 例 { #example_1 }

```bash
capsula run-end happy-river
```

## `capsula list` { #capsula-list }

記録されたすべてのランを一覧表示します。

### 使い方 { #usage_3 }

```bash
capsula list
```

### 出力形式 { #output-format }

```
TIMESTAMP (UTC)      NAME                  COMMAND
---------------------------------------------------------------------------------------------
2025-01-09 14:30:29  happy-river           echo hello
2025-01-09 14:30:28  clever-mountain       python script.py
2025-01-09 14:30:26  quiet-lake            cargo build --release
```

## `capsula show` { #capsula-show }

特定のランの詳細情報を表示します。メタデータ、コマンドの結果、フックの概要が含まれます。

### 使い方 { #usage_4 }

```bash
capsula show <RUN_NAME>
capsula show <RUN_NAME> --json
```

### オプション { #options }

| オプション | 説明 |
| -------- | ------------- |
| `--json` | ランの全データを JSON で出力します |

### 例 { #example_2 }

```bash
$ capsula show happy-river
Run:       happy-river
ID:        01K8WSYC91YAE21R7CWHQ4KYN2
Timestamp: 2025-01-09 14:30:22 UTC
Command:   python train.py --epochs 100
Directory: /path/.capsula/vault/2025-01-09/143022-happy-river
Result:    exit 0 (1.23s)

Pre-run hooks:
  [ok]     capture-git-repo
  [ok]     capture-env
Post-run hooks:
  [ok]     capture-file
```

## `capsula run-dir` { #capsula-run-dir }

ランの名前に対応するランディレクトリを出力します。

### 使い方 { #usage_5 }

```bash
capsula run-dir <RUN_NAME>
```

### 例 { #example_3 }

```bash
capsula run-dir happy-river
```

## `capsula push` { #capsula-push }

ランのデータを capsula サーバーにアップロードします。

[`capsula pull`](#capsula-pull) で復元したラン (`_capsula/pulled.json` で印が付いたもの) は、
情報の一部が失われた再構成です。そのようなランを直接 push しようとすると拒否され、
`push --all` ではスキップされます。

### 使い方 { #usage_6 }

```bash
capsula push <RUN_ID_OR_NAME>
capsula push --all
```

### オプション { #options_1 }

| オプション | 説明 |
| -------- | ------------- |
| `--all` | ボールト内のすべてのランを push します |
| `--server <URL>` | サーバーの URL (`CAPSULA_SERVER_URL` 環境変数や `capsula.toml` の `server` フィールドでも指定できます) |

### 例 { #examples_1 }

```bash
# Push a single run by name
capsula push happy-river

# Push a single run by ID
capsula push 01K8WSYC91YAE21R7CWHQ4KYN2

# Push all runs
capsula push --all

# Push to a specific server
capsula push happy-river --server https://capsula.example.com
```

## `capsula pull` { #capsula-pull }

capsula サーバーからランをダウンロードし、ローカルのボールトに復元します。

ランは {YYYY-MM-DD}/{HHMMSS}-{name} に、ローカルで作成したランと同じレイアウトで復元されます。ダウンロードしたファイルは、記録されたサイズと SHA-256 ハッシュで検証されます。絶対パスや .. を含むパスは拒否されます。

ランの復元先は、そのランのボールトによって決まります。

- 設定済みのボールトのランは、設定済みのボールトディレクトリ (`capsula.toml` の `[vault] path`) に復元されます。
- 別のボールトのランは、そのボールトのデフォルトの場所であるプロジェクトルート配下の `.capsula/<VAULT>/` に復元されます。設定済みのパスは、設定済みのボールトだけを表すためです。
- `--vault-path <PATH>` による上書き (または `CAPSULA_VAULT_PATH`) は常に優先され、どちらのボールトでも復元先として使用されます。

_capsula のメタデータは、サーバーの構造化データから再構成されます。_capsula/pulled.json マーカーには、取得元のサーバーと pull した時刻が記録されます。

--force が置き換えるのは、_capsula/pulled.json によって識別される、capsula pull で以前に作成されたランだけです。ローカルで作成したランが上書きされることはありません。pull したランを capsula push で再びアップロードすることはできません。

### 使い方 { #usage_7 }

```bash
capsula pull <RUN_ID>
```

### オプション { #options_2 }

| オプション | 説明 |
| -------- | ------------- |
| `--vault <NAME>` | ランが属しているはずのボールト (デフォルトは `capsula.toml` のボールト)。ランが別のボールトに属している場合、pull は失敗します。 |
| `--vault-path <PATH>` | グローバルオプションです。ボールトのデフォルトの場所ではなく、このディレクトリにランを復元します。 |
| `--server <URL>` | サーバーの URL (`CAPSULA_SERVER_URL` 環境変数や `capsula.toml` の `server` フィールドでも指定できます) |
| `--force` | 以前に pull したランのコピーがすでに存在する場合に、それを置き換えます。ローカルで作成したランが置き換えられることはありません。 |

### 例 { #examples_2 }

```bash
# Pull a run by ID
capsula pull 01K8WSYC91YAE21R7CWHQ4KYN2

# Pull a run that belongs to another vault (restored into .capsula/other-vault/)
capsula pull 01K8WSYC91YAE21R7CWHQ4KYN2 --vault other-vault

# Pull a run into an explicit directory
capsula --vault-path /custom/vault/path pull 01K8WSYC91YAE21R7CWHQ4KYN2 --vault other-vault

# Pull from a specific server, replacing a previously pulled copy
capsula pull 01K8WSYC91YAE21R7CWHQ4KYN2 --server https://capsula.example.com --force
```

### 制限事項 { #limitations }

pull したランは、元のローカルのランの一部にすぎません。capsula push でアップロードされなかったデータは復元できません。これには次のものが含まれます。

- シンボリックリンク
- 空のディレクトリ
- push 後に追加されたファイル
- 元の _capsula/*.json ファイルの内容
- ミリ秒未満のコマンド実行時間

そのため、再構成されたメタデータは元のファイルと異なる場合があります。フックの出力がないフェーズでは、pre-run.json や post-run.json は作成されません。古いバージョンのサーバーに保存されたコマンド文字列は、JSON 配列に正規化されます。

## `capsula tui` { #capsula-tui }

ランの開始と終了を行うための対話型ターミナル UI を起動します。

TUI は、コマンドラインから `run-start` と `run-end` を使う代わりに、
手動のランを視覚的に管理するためのインターフェースです。

### 使い方 { #usage_8 }

```bash
capsula tui
```

## `capsula vaults` { #capsula-vaults }

サーバー上のボールトを管理します。

### `capsula vaults list` { #capsula-vaults-list }

サーバー上のすべてのボールトを一覧表示します。

```bash
capsula vaults list
capsula vaults list --server https://capsula.example.com
```

#### オプション { #options_3 }

| オプション | 説明 |
| -------- | ------------- |
| `--server <URL>` | サーバーの URL (`CAPSULA_SERVER_URL` 環境変数や `capsula.toml` の `server` フィールドでも指定できます) |
