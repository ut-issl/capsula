---
icon: material/cog
---

# 設定 { #configuration }

Capsula は `capsula.toml` ファイルで設定します。このページでは、すべての設定項目を説明します。

## 設定ファイルの場所 { #configuration-file-location }

Capsula は次の順序で `capsula.toml` を探します。

1. `--config` フラグで指定されたパス
2. カレントディレクトリ (`./capsula.toml`)
3. 親ディレクトリ (ディレクトリツリーを上にたどります)

!!! tip "ヒント"
    `capsula.toml` をプロジェクトルートに置くと、どのサブディレクトリからでも Capsula を使えます。

## 基本構造 { #basic-structure }

```toml
[vault]
name = "vault-name"

[[pre-run.hooks]]
id = "hook-type"
# hook configuration...

[[post-run.hooks]]
id = "hook-type"
# hook configuration...
```

## トップレベルの設定項目 { #top-level-options }

### `dotenv` (省略可能) { #dotenv-optional }

`.env` ファイルから環境変数を読み込みます。

```toml
dotenv = ".env"
```

相対パスは、`capsula.toml` があるディレクトリを基準に解決されます。

### `server` (省略可能) { #server-optional }

`capsula push` と `capsula vaults` が使用する Capsula サーバーです。最も簡単な形式は URL をそのまま書く方法です。

```toml
server = "https://capsula.example.com"
```

URL は `--server` フラグや `CAPSULA_SERVER_URL` 環境変数でも指定でき、これらは設定ファイルより優先されます (優先順位はこの記載順です)。`[server.headers]` を設定している場合、上書きする URL は設定済みの `url` と同じオリジン (スキーム、ホスト、ポート) を指している必要があります。認証情報が別のオリジンに送られることはなく、異なるオリジンへの上書きはエラーとして拒否されます。

#### `server.headers` (省略可能) { #serverheaders-optional }

サーバーが認証付きリバースプロキシ (Cloudflare Access、oauth2-proxy など) の背後にある場合は、テーブル形式を使ってすべてのリクエストに HTTP ヘッダーを付加できます。

```toml
[server]
url = "https://capsula.example.com"

[server.headers]
Authorization = { env = "CAPSULA_TOKEN", prefix = "Bearer " }
```

各エントリーが 1 行の HTTP ヘッダーになります。環境に `CAPSULA_TOKEN=abc123` が設定されていれば、上の例では `Authorization: Bearer abc123` が送信されます。

各ヘッダーの値は、次の 3 種類のいずれかから取得します。

| 形式 | 意味 |
|------|---------|
| `"literal string"` | 文字列をそのまま使用します |
| `{ env = "VAR", prefix = "Bearer " }` | 環境変数から読み取ります。省略可能な `prefix` を先頭に付加します |
| `{ command = "..." }` | コマンドを実行し、前後の空白を除いた標準出力を値として使用します |

よくある設定例です。

```toml
[server.headers]
# Plain bearer token against any authenticating reverse proxy
Authorization = { env = "CAPSULA_TOKEN", prefix = "Bearer " }

# Cloudflare Access, interactive user (JWT from cloudflared login session)
cf-access-token = { command = "cloudflared access token --app=https://capsula.example.com" }

# Cloudflare Access service token (CI)
CF-Access-Client-Id = { env = "CF_ACCESS_CLIENT_ID" }
CF-Access-Client-Secret = { env = "CF_ACCESS_CLIENT_SECRET" }
```

コマンドはシェルと同様の規則で単語に分割され、シェルを介さずにプロジェクトルート (`capsula.toml` があるディレクトリ) で直接実行されます。標準出力の末尾の改行は取り除かれます。コマンドが失敗した場合 (ログインセッションの期限切れなど)、push はそのコマンドの標準エラー出力とともに中断されます。

HTTP リダイレクトには一切従いません。リダイレクト (ログインページへのリダイレクトなど) は、認証情報をリダイレクト先に転送せず、エラーとして報告されます。

!!! warning "信頼モデル"
    `command` エントリーは、`capsula push` や `capsula vaults` を実行したときに任意のコマンドを実行します。信頼できる `capsula.toml` でのみ使用してください。これは `capture-command` フックにすでに当てはまる注意と同じです。

!!! tip "シークレット"
    シークレットの値を `capsula.toml` に直接書かないでください。`env` で参照するか (必要に応じてトップレベルの `dotenv` 設定と組み合わせます)、認証情報を外部ツール側 (`~/.cloudflared/` など) で保持する `command` を使用してください。

## ボールトの設定 { #vault-configuration }

`[vault]` セクションでは、Capsula が記録したデータを保存する場所を定義します。

### `name` (必須) { #name-required }

ボールトの名前です。`.capsula/` の下にサブディレクトリが作成されます。

```toml
[vault]
name = "ml-experiments"
```

作成されるディレクトリ: `.capsula/ml-experiments/`

### `path` (省略可能) { #path-optional }

ボールトのパスを独自に指定します。絶対パスと相対パスのどちらも指定できます。

```toml
[vault]
name = "experiments"
path = "/absolute/path/to/vault"
```

## フックの設定 { #hook-configuration }

フックは `pre-run` と `post-run` の 2 つのセクションで設定します。
各フックの設定については、それぞれのドキュメントを参照してください。

### 実行前フック { #pre-run-hooks }

実行前フックは、コマンドの**前に**、記載された順序で実行されます。いずれかの実行前フックが失敗またはエラーになった場合でも、Capsula は残りの実行前フックを実行して `pre-run.json` を書き出し、その後コマンドを実行せずに停止します。

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-project"
path = "."

[[pre-run.hooks]]
id = "capture-cwd"
```

### 実行後フック { #post-run-hooks }

実行後フックは、コマンドの**後に**、記載された順序で実行されます。いずれかの実行後フックが失敗またはエラーになった場合でも、Capsula は残りの実行後フックを実行して `post-run.json` を書き出します。`capsula run` では、ラップしたコマンドが成功していれば、実行後フックの失敗によって CLI が失敗します。ラップしたコマンドがすでに失敗している場合、Capsula はそのコマンドの終了コードを維持します。`capsula run-end` では、実行後フックが失敗すると、`post-run.json` を記録した後にコマンドが失敗します。

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
```

## 完全な例 { #complete-example }

```toml title="capsula.toml"
dotenv = ".env"

[vault]
name = "research-experiments"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "research"
path = "."
allow_dirty = false

[[pre-run.hooks]]
id = "capture-cwd"

[[pre-run.hooks]]
id = "capture-env"
name = "PATH"

[[pre-run.hooks]]
id = "capture-machine"

[[pre-run.hooks]]
id = "capture-file"
glob = "config.yaml"
mode = "copy"
hash = "sha256"

[[post-run.hooks]]
id = "capture-file"
glob = "results/*.json"
mode = "copy"

[[post-run.hooks]]
id = "notify-slack"
channel = "#experiments"
attachment_globs = ["results/*.png"]
```

## 複数の設定ファイル { #multiple-configurations }

用途ごとに別の設定ファイルを使い分けられます。

```bash
capsula --config experiments.toml run python train.py
capsula --config builds.toml run cargo build
```
