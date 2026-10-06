---
icon: material/hook
---

# capture-toml { #capture-toml }

1 つの TOML ファイルをパースし、その内容をラン出力の `content` フィールドに埋め込みます。

## ユースケース { #use-cases }

- 実験のハイパーパラメーターをラン間で検索できるようにする
- スクリプトが使った設定そのものを、構造化されたインデックス可能な形で記録する
- 複数の `capture-toml` エントリーを組み合わせて、複数の設定ファイルを記録する

## 設定 { #configuration }

### 必須オプション { #required-options }

| オプション | 型 | 説明 |
| ------ | ------ | ---------------------------------------------------------- |
| `path` | string | パースする TOML ファイルのパス。プロジェクトルートからの相対パスです。 |

### 例 { #example }

```toml
[[pre-run.hooks]]
id = "capture-toml"
path = "config/sat1/orbit.toml"
```

## 出力例 { #output-example }

`content` フィールドには、パースされて JSON に変換された TOML が入ります。設定したパスは出力本体には重複して含まれません。オーケストレーターが付与する標準の `__meta.config.path` フィールドにすでに保存されているためです。

`config/sat1/orbit.toml` が次の内容の場合:

```toml
[orbit]
a = 1.42
b = "LEO"
```

フックの出力は次のとおりです:

```json
{
  "__meta": {
    "id": "capture-toml",
    "config": {
      "path": "config/sat1/orbit.toml"
    },
    "success": true
  },
  "content": {
    "orbit": {
      "a": 1.42,
      "b": "LEO"
    }
  }
}
```

複数の `capture-toml` の出力を区別するには、`__meta.config.path` で絞り込んでください。

## 複数ファイルの組み合わせ { #composing-multiple-files }

このフックは、1 つのインスタンスにつきちょうど 1 つのファイルを記録します。すべての設定ファイルを記録するには、設定ファイルごとにエントリーを 1 つずつ登録してください:

```toml
[[pre-run.hooks]]
id = "capture-toml"
path = "config/sat1/orbit.toml"

[[pre-run.hooks]]
id = "capture-toml"
path = "config/sat2/orbit.toml"
```

各エントリーは `pre-run.json` にそれぞれ独自の行を生成し、それぞれが独自の `content` フィールドを持ちます。設定したパスは `__meta.config.path` から参照できます。

## TOML から JSON への変換 { #toml-json-conversion }

TOML には、JSON でそのままでは表現できない型がいくつかあります。このフックはそれらを次のように扱います:

| TOML | JSON |
| ---------------------------- | ----------------------------------------------- |
| 文字列 / 整数 / 真偽値 | 対応する JSON の型 |
| 浮動小数点数 | JSON の数値 (NaN / ±Inf は `null` になります) |
| 日時 (オフセット付き / ローカル) | JSON の文字列 (RFC 3339 表現) |
| 配列 | JSON の配列 |
| テーブル (`[section]`) | JSON のオブジェクト |

例: `created_at = 2026-01-08T10:20:00Z` は JSON 出力では `"created_at": "2026-01-08T10:20:00Z"` になり、文字列として検索できます。

## エラー時の動作 { #error-behaviour }

次の場合、フックは失敗します (`__meta.success: false` として記録されます):

- ファイルが存在しない、または読み取れない (`Io` エラー)
- ファイルの内容が有効な TOML ではない (`Toml` エラー)

`capture-toml` が失敗しても、他のフックの実行は止まりません。

## 関連項目 { #see-also }

- [`capture-file`](capture-file.md) — 任意のファイルをバイト単位で正確に保存します
- [`capture-json`](capture-json.md) — 同じ形式で JSON 入力を扱います
- [`capture-yaml`](capture-yaml.md) — 同じ形式で YAML 入力を扱います
