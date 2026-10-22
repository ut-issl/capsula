---
icon: material/download
---

# インストール { #installation }

このガイドでは、Capsula をシステムにインストールする方法を説明します。

## 要件 { #requirements }

- **Rust ツールチェーン** (バージョン 1.95.0 以降)

!!! tip "Rust がインストールされていない場合"
    [rustup.rs](https://rustup.rs/) から Rust をインストールしてください。数分で完了します。

## Capsula のインストール { #installing-capsula }

### 方法 1: crates.io からインストールする (推奨) { #method-1-install-from-cratesio-recommended }

Capsula をインストールする最も簡単な方法です。

```bash
cargo install capsula --locked
```

このコマンドは、Rust のパッケージレジストリから最新の安定版をダウンロードしてコンパイルします。

### 方法 2: GitHub からインストールする { #method-2-install-from-github }

最新の開発版をインストールするには、次のコマンドを実行します。

```bash
cargo install --git https://github.com/ut-issl/capsula --locked capsula
```

!!! warning "開発版"
    GitHub 上の開発版には新しい機能が含まれている場合がありますが、リリース版より安定性が低い可能性があります。

## インストールの確認 { #verify-installation }

インストール後、Capsula が動作することを確認します。

```bash
capsula --version
```

次のような出力が表示されます。

```
capsula 0.15.1
```

ヘルプコマンドも試してみてください。

```bash
capsula --help
```

## Capsula の更新 { #updating-capsula }

Capsula を最新バージョンに更新するには、同じインストールコマンドをもう一度実行します。

```bash
cargo install capsula --locked
```

Cargo が新しいバージョンを自動的にダウンロードしてインストールします。

## トラブルシューティング { #troubleshooting }

### 「cargo: command not found」と表示される { #cargo-command-not-found }

Rust がインストールされていないか、PATH に含まれていません。[rustup.rs](https://rustup.rs/) から Rust をインストールし、ターミナルを再起動してください。

### コンパイルエラー { #compilation-errors }

**Rust のバージョンが古い場合:**

Rust のバージョンを確認します。

```bash
rustc --version
```

Capsula には Rust 1.95.0 以降が必要です。Rust を更新してください。

```bash
rustup update
```

その後、もう一度インストールを試します。

```bash
cargo install capsula --locked
```

**システムライブラリが不足している場合:**

「linker」や「library not found」といったエラーでコンパイルに失敗する場合は、システムの開発用ライブラリのインストールが必要なことがあります。エラー出力をよく確認してください。通常は、どのライブラリが不足しているかが示されています。

## Capsula のアンインストール { #uninstalling-capsula }

Capsula をアンインストールするには、次のコマンドを実行します。

```bash
cargo uninstall capsula
```

このコマンドは Capsula のバイナリを削除しますが、`.capsula` ディレクトリに記録されたデータは削除しません。
