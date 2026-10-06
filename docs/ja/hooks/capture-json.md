---
icon: material/hook
---

# capture-json { #capture-json }

1 つの JSON ファイルをパースし、その内容をラン出力の `content` フィールドに埋め込みます。

## ユースケース { #use-cases }

- 実験のハイパーパラメーターをラン間で検索できるようにする
- スクリプトが使った設定そのものを、構造化されたインデックス可能な形で記録する
- 複数の `capture-json` エントリーを組み合わせて、複数の設定ファイルを記録する

## 設定 { #configuration }

### 必須オプション { #required-options }

| オプション | 型 | 説明 |
| ------ | ------ | ---------------------------------------------------------- |
| `path` | string | パースする JSON ファイルのパス。プロジェクトルートからの相対パスです。 |

### 例 { #example }

```toml
[[pre-run.hooks]]
id = "capture-json"
path = "config/sat1/orbit.json"
```

## 出力例 { #output-example }

`content` フィールドにはパースされた JSON が入ります。設定したパスは出力本体には重複して含まれません。オーケストレーターが付与する標準の `__meta.config.path` フィールドにすでに保存されているためです。

```json
{
  "__meta": {
    "id": "capture-json",
    "config": {
      "path": "config/sat1/orbit.json"
    },
    "success": true
  },
  "content": {
    "a": 1.42,
    "b": "LEO"
  }
}
```

複数の `capture-json` の出力 (たとえばベース名が同じ 2 つのファイル) を区別するには、`__meta.config.path` で絞り込んでください。

## 複数ファイルの組み合わせ { #composing-multiple-files }

このフックは、1 つのインスタンスにつきちょうど 1 つのファイルを記録します。すべての設定ファイルを記録するには、設定ファイルごとにエントリーを 1 つずつ登録してください:

```toml
[[pre-run.hooks]]
id = "capture-json"
path = "config/sat1/orbit.json"

[[pre-run.hooks]]
id = "capture-json"
path = "config/sat2/orbit.json"
```

各エントリーは `pre-run.json` にそれぞれ独自の行を生成し、それぞれが独自の `content` フィールドを持ちます。設定したパスは `__meta.config.path` から参照できます。

## エラー時の動作 { #error-behaviour }

次の場合、フックは失敗します (`__meta.success: false` として記録されます):

- ファイルが存在しない、または読み取れない (`Io` エラー)
- ファイルの内容が有効な JSON ではない (`Json` エラー)

`capture-json` が失敗しても、他のフックの実行は止まりません。

## 関連項目 { #see-also }

- [`capture-file`](capture-file.md) — 任意のファイルをバイト単位で正確に保存します
- [`capture-toml`](capture-toml.md) — 同じ形式で TOML 入力を扱います
- [`capture-yaml`](capture-yaml.md) — 同じ形式で YAML 入力を扱います
