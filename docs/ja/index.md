---
icon: material/home
---

# Capsula へようこそ { #welcome-to-capsula }

Capsula は、コマンドの実行に関する情報を自動的に記録して保存するコマンドラインツールです。何が起きたのか、いつ起きたのか、どのような環境で起きたのかを記録します。

## Capsula でできること { #what-does-capsula-do }

Capsula を通してコマンドを実行すると、Capsula は次の処理を行います。

1. **環境を記録する** - Git の状態、環境変数、ファイルの内容、システム情報を記録します
2. **コマンドを実行する** - コマンドを通常どおり実行し、その出力を記録します
3. **すべてを保存する** - 記録したデータをすべて、整理されたディレクトリ構造に保存します

## 簡単な例 { #quick-example }

`capsula.toml` ファイルを作成します。

```toml
[vault]
name = "my-project"

[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-project"
path = "."

[[post-run.hooks]]
id = "capture-file"
glob = "output.txt"
mode = "copy"
```

コマンドを実行します。

```bash
capsula run python train_model.py
```

Capsula は次のように整理されたディレクトリを作成します。

```
.capsula/my-project/2025-01-09/143022-happy-river/
├── _capsula/
│   ├── metadata.json           # What ran, when, and where
│   ├── pre-run.json            # Environment before
│   ├── command.json            # Command output
│   └── post-run.json           # Results after
├── pre-0-capture-git-repo/     # Per-hook artifact directory
└── post-0-capture-file/        # Per-hook artifact directory
    └── output.txt              # Your output file
```

ファイルのアーティファクトを生成するフック (`capture-file` や `capture-git-repo` など) には、
それぞれ `{phase}-{index}-{hook_id}/` という名前の専用サブディレクトリが割り当てられます。
これにより、フック間でファイル名が衝突することを防ぎます。

## Capsula を使う理由 { #why-use-capsula }

- **再現性** - 実行ごとに、正確な環境と入力を記録します
- **トレーサビリティ** - どのバージョンのコードがどの結果を生み出したのかを把握できます
- **監査** - 完全な実行記録を作成できます
- **デバッグ** - 完全なコンテキストを確認して、何が問題だったのかを把握できます

## はじめに { #getting-started }

<div class="grid cards" markdown>

- :material-download:{ .lg .middle } **インストール**

    ---

    Capsula をシステムにインストールします。

    [:octicons-arrow-right-24: Capsula をインストールする](installation.md)

- :material-rocket-launch:{ .lg .middle } **はじめての Capsula**

    ---

    Capsula で最初のコマンドを実行します。

    [:octicons-arrow-right-24: クイックスタートチュートリアル](getting-started.md)

- :material-cog:{ .lg .middle } **設定**

    ---

    Capsula の設定方法を学びます。

    [:octicons-arrow-right-24: 設定ガイド](configuration.md)

- :material-hook:{ .lg .middle } **フック**

    ---

    Capsula で記録できる内容を確認します。

    [:octicons-arrow-right-24: 利用できるフック](getting-started.md#available-hooks)

</div>
