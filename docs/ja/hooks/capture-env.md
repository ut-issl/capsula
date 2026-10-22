---
icon: material/hook
---

# capture-env { #capture-env }

環境変数の値を記録します。

## ユースケース { #use-cases }

- 利用可能だった実行ファイルを記録する (PATH)
- 環境変数による設定を保存する (例: `CUDA_VISIBLE_DEVICES`)
- 環境に起因する問題をデバッグする
- ユーザーのコンテキストを追跡する (例: `USER`、`HOME`)

## 設定 { #configuration }

### 必須オプション { #required-options }

| オプション | 型 | 説明 |
| -------- | ------ | ------------- |
| `name` | string | 記録する環境変数の名前 |

### 例 { #example }

```toml
[[pre-run.hooks]]
id = "capture-env"
name = "PATH"
```

## 出力例 { #output-example }

### 変数が存在する場合 { #when-variable-exists }

```json
{
  "__meta": {
    "id": "capture-env",
    "config": {
      "name": "PATH"
    },
    "success": true
  },
  "value": "/usr/local/bin:/usr/bin:/bin"
}
```

### 変数が存在しない場合 { #when-variable-doesnt-exist }

```json
{
  "__meta": {
    "id": "capture-env",
    "config": {
      "name": "MY_VAR"
    },
    "success": true
  },
  "value": null
}
```
