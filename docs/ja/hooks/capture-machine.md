---
icon: material/hook
---

# capture-machine { #capture-machine }

CPU、メモリ、OS、ホスト名などのシステム情報を記録します。

## ユースケース { #use-cases }

- ハードウェアの仕様を追跡する
- プラットフォーム固有の問題をデバッグする
- 結果とシステムの性能を関連付ける
- 必要なリソースを文書化する

## 設定 { #configuration }

設定オプションはありません。

```toml
[[pre-run.hooks]]
id = "capture-machine"
```

## 出力例 { #output-example }

```json
{
  "__meta": {
    "id": "capture-machine",
    "config": {},
    "success": true
  },
  "hostname": "macbook-pro.local",
  "os": "Darwin",
  "os_version": "25.2.0",
  "kernel_version": "25.0.0",
  "architecture": "aarch64",
  "total_memory": 68719476736,
  "cpus": [
    {
      "name": "1",
      "brand": "Apple M3 Max",
      "vendor_id": "Apple",
      "frequency_mhz": 4056
    },
    {
      "name": "2",
      "brand": "Apple M3 Max",
      "vendor_id": "Apple",
      "frequency_mhz": 4056
    }
  ]
}
```
