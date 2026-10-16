---
icon: material/hook
---

# capture-git-repo { #capture-git-repo }

コミットハッシュ、ブランチ、未コミットの変更の有無など、Git リポジトリの状態を記録します。

## ユースケース { #use-cases }

- 再現性のために、使用したコードの正確なバージョンを記録する
- 未コミットの変更がある場合に実行を防ぐ
- 他の人がアクセスできるよう、コミットがリモートにプッシュされていることを保証する
- どのコードがどの結果を生み出したかを追跡する
- どのバージョンのコードが使われたかを監査する

## 設定 { #configuration }

### 必須オプション { #required-options }

| オプション | 型 | 説明 |
| -------- | ------ | ------------- |
| `name` | string | リポジトリに未コミットの変更がある場合に作成するパッチファイル (`<name>.patch`) のベース名 |
| `path` | string | Git リポジトリへのパス (カレントディレクトリの場合は `.`) |

### 任意オプション { #optional-options }

| オプション | 型 | デフォルト | 説明 |
| -------- | ------ | --------- | ------------- |
| `allow_dirty` | boolean | `false` | `false` の場合、リポジトリに未コミットの変更があると Capsula は中断します |
| `require_pushed` | boolean | `false` | `true` の場合、HEAD コミットがリモートにプッシュされていないと Capsula は中断します |
| `remote` | string | `"origin"` | `require_pushed` が `true` のときに確認するリモートの名前 |
| `tag_head` | boolean | `false` | `true` の場合、Git のガベージコレクションを防ぐため、HEAD コミットに軽量タグ `capsula/<run-name>` を作成します |

### 例 { #example }

```toml
[[pre-run.hooks]]
id = "capture-git-repo"
name = "my-project"
path = "."
allow_dirty = false
require_pushed = true
remote = "origin"
tag_head = true
```

## 出力例 { #output-example }

### クリーンなリポジトリ (プッシュ済み、タグあり) { #clean-repository-pushed-with-tag }

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "my-project",
      "path": ".",
      "allow_dirty": false,
      "require_pushed": true,
      "remote": "origin",
      "tag_head": true
    },
    "success": true
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": false,
  "is_pushed": true,
  "tag": "capsula/chubby-back"
}
```

### ダーティなリポジトリ (`allow_dirty = true` の場合) { #dirty-repository-with-allow_dirty-true }

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "my-project",
      "path": ".",
      "allow_dirty": true,
      "require_pushed": false,
      "remote": "origin"
    },
    "success": true
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": true,
  "is_pushed": true,
  "tag": null
}
```

!!! note "パッチファイルの場所"
    リポジトリがダーティで (かつフックが処理の続行を許可されている) 場合、Capsula は
    差分をフックのアーティファクトディレクトリ内の `<name>.patch` に書き出します。
    このディレクトリはランディレクトリ配下の `{phase}-{index}-capture-git-repo/` にあります
    (例: `pre-0-capture-git-repo/my-project.patch`)。

### ダーティなリポジトリによる失敗 (`allow_dirty = false` の場合) { #dirty-repository-failure-with-allow_dirty-false }

```json
{
  "__meta": {
    "id": "capture-git-repo",
    "config": {
      "name": "my-project",
      "path": ".",
      "allow_dirty": false,
      "require_pushed": false,
      "remote": "origin"
    },
    "success": false,
    "failure_reason": "repository has uncommitted changes"
  },
  "working_dir": "/Users/username/projects/experiment",
  "sha": "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0",
  "is_dirty": true,
  "is_pushed": true,
  "tag": null
}
```

!!! warning "中断時の動作"
    `allow_dirty = false` でリポジトリがダーティな場合、Capsula はダーティな状態を示すフックの出力を保存し、残りの実行前フックを実行してから、コマンドを実行する前に中断します。

!!! warning "中断時の動作 (プッシュの確認)"
    `require_pushed = true` で HEAD コミットがどのリモートブランチからも到達できない場合、Capsula はフックの出力を保存し、残りの実行前フックを実行してから、コマンドを実行する前に中断します。

!!! note "プッシュ確認の詳細"
    - プッシュの確認では、設定したリモートのリモート追跡ブランチから HEAD コミットに到達できるかを検証します。HEAD がリモートブランチの先端にあることは**必要ありません**。祖先のコミットもプッシュ済みとみなされます。
    - この確認はローカルのリモート追跡参照に依存します。最新のリモートの状態が必要な場合は、`capsula run` の前に `git fetch` を実行してください。

!!! warning "スカッシュマージとコミットの到達可能性"
    ランの時点で `require_pushed = true` の確認に通っても、ブランチがスカッシュマージされて削除されると、後でそのコミットに到達できなくなることがあります。スカッシュマージ後は、元のコミットはターゲットブランチ上の新しい 1 つのコミットに置き換えられ、元のコミット SHA には到達できなくなります。コミットの到達可能性を保つには、実験のランを含むブランチをマージするときに (スカッシュマージではなく) **マージコミット**を使用してください。

!!! tip "`tag_head` によるガベージコレクションの防止"
    `tag_head = true` の場合、Capsula は HEAD コミットを指す軽量 Git タグ `capsula/<run-name>` を作成します。これにより、ブランチの削除や履歴の書き換え (例: リベース、スカッシュマージ) の後でも、Git がそのコミットをガベージコレクションしなくなります。Capsula のタグは `git tag -l 'capsula/*'` ですべて一覧表示でき、`git tag -d <tag-name>` で削除できます。
