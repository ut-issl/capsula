---
icon: material/hook
---

# capture-command { #capture-command }

シェルコマンドを実行し、その出力と終了コードを記録します。

## ユースケース { #use-cases }

- ツールのバージョンを記録する (例: Python、Node.js)
- 診断用のコマンドを実行する (例: `nvidia-smi`、`df`)
- 解析スクリプトを実行する
- メインのコマンドを実行する前に前提条件を検証する

## 設定 { #configuration }

### 必須オプション { #required-options }

| オプション | 型 | 説明 |
| -------- | ------ | ------------- |
| `command` | array | 配列で指定するコマンドと引数 (例: `["python", "--version"]`) |

### 任意オプション { #optional-options }

| オプション | 型 | デフォルト | 説明 |
| -------- | ------ | --------- | ------------- |
| `success_codes` | array of integers | `[0]` | フックの結果を成功とみなす終了ステータス |
| `abort_on_failure` | boolean | 未設定 | 互換性のための非推奨オプションです。明示的に `false` を設定すると、どの終了ステータスも受け入れます。新しい設定では `success_codes` を使用してください。 |

### 例 { #example }

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["python", "--version"]
```

コマンドが失敗することを意図的に確認するには、期待する 0 以外のステータスを設定します:

```toml
[[pre-run.hooks]]
id = "capture-command"
command = ["test", "-f", "missing-file.txt"]
success_codes = [1]
```

## 出力例 { #output-example }

### 成功したコマンド { #successful-command }

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["python", "--version"]
    },
    "success": true
  },
  "status": 0,
  "stdout": "Python 3.11.5\n",
  "stderr": ""
}
```

### 想定外のステータス { #unexpected-status }

```json
{
  "__meta": {
    "id": "capture-command",
    "config": {
      "command": ["test", "-f", "required-file.txt"]
    },
    "success": false,
    "failure_reason": "command exited with status 1; expected 0"
  },
  "status": 1,
  "stdout": "",
  "stderr": ""
}
```

!!! warning "失敗時の動作"
    実行前フックの失敗は記録され、残りの実行前フックはそのまま実行されたうえで、Capsula はメインのコマンドを実行する前に停止します。実行後フックの失敗はメインのコマンドの後に記録されます。メインのコマンドが成功していた場合は `capsula run` が失敗し、メインのコマンドがすでに失敗していた場合はその 0 以外の終了コードが維持されます。
