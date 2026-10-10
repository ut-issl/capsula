---
icon: material/hook
---

# capture-cwd { #capture-cwd }

Capsula を実行しているカレントワーキングディレクトリを記録します。

## ユースケース { #use-cases }

- 実行場所を記録する
- 相対パスに関する問題をデバッグする
- プロセスがどこで実行されたかを追跡する

## 設定 { #configuration }

設定オプションはありません。

```toml
[[pre-run.hooks]]
id = "capture-cwd"
```

## 出力例 { #output-example }

```json
{
  "__meta": {
    "id": "capture-cwd",
    "config": {},
    "success": true
  },
  "cwd": "/Users/username/projects/my-experiment"
}
```
