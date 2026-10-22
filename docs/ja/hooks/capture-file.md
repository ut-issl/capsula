---
icon: material/hook
---

# capture-file { #capture-file }

ファイルをコピーまたは移動したり、ハッシュを計算したりして記録します。

## ユースケース { #use-cases }

- 入力の設定ファイルを保存する
- 出力結果をアーカイブする
- 大きなファイルをコピーせずにファイルの完全性を検証する
- 実験のアーティファクトを整理する

## 設定 { #configuration }

### 必須オプション { #required-options }

| オプション | 型 | 説明 |
| -------- | ------ | ------------- |
| `glob` | string | 一致させるファイルのパターン (例: `"*.txt"`、`"results/**/*.png"`) |

### 任意オプション { #optional-options }

| オプション | 型 | デフォルト | 説明 |
| -------- | ------ | --------- | ------------- |
| `mode` | string | `"copy"` | ファイルの扱い方: `"copy"`、`"move"`、`"none"` のいずれか |
| `hash` | string | `"sha256"` | ハッシュアルゴリズム: `"sha256"` または `"none"` |

### モードの選択肢 { #mode-options }

- `"copy"` - ファイルをフックのアーティファクトディレクトリにコピーし、元のファイルは残します
- `"move"` - ファイルをフックのアーティファクトディレクトリに移動し、元のファイルを削除します
- `"none"` - ハッシュの計算のみを行い、コピーや移動はしません

!!! note "フックごとのアーティファクトディレクトリ"
    `mode` が `"copy"` または `"move"` の場合、ファイルはランディレクトリ配下の
    `{phase}-{index}-capture-file/` という名前のフックごとのアーティファクトディレクトリ
    (例: `post-0-capture-file/`) に配置されます。これにより、複数の `capture-file`
    フックの間でファイル名が衝突するのを防ぎます。

### 例 { #example }

```toml
[[post-run.hooks]]
id = "capture-file"
glob = "results/*.csv"
mode = "copy"
hash = "sha256"
```

## 出力例 { #output-example }

```json
{
  "__meta": {
    "id": "capture-file",
    "config": {
      "glob": "results/*.csv",
      "mode": "copy",
      "hash": "sha256"
    },
    "success": true
  },
  "files": [
    {
      "src": "results/data.csv",
      "dst": ".capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/data.csv",
      "hash": "a1b2c3d4e5f6...",
      "copied": true
    },
    {
      "src": "results/summary.csv",
      "dst": ".capsula/my-vault/2025-01-09/143022-happy-river/post-0-capture-file/summary.csv",
      "hash": "b2c3d4e5f6g7...",
      "copied": true
    }
  ]
}
```
