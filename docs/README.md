# Maintaining English and Japanese documentation

## Layout and deployment

| Edition | Source | Configuration | Output | Public URL |
| --- | --- | --- | --- | --- |
| English (editorial source) | `docs/en/` | `zensical.toml` | `site/en/` | `https://www.space.t.u-tokyo.ac.jp/capsula/en/` |
| Japanese | `docs/ja/` | `zensical.ja.toml` | `site/ja/` | `https://www.space.t.u-tokyo.ac.jp/capsula/ja/` |

Each edition is a separate Zensical project. Zensical owns everything inside an
edition, including search, sitemaps, and the language selector. Keep this
repository to its own part: the configuration, the content, and the site root.
Do not add tooling for features that belong to Zensical (for example,
translation tracking or page-level redirects), even if Zensical lacks them.

The site root (`docs/site-root/`) is the only content outside the editions:
`index.html` redirects `/capsula/` to the English edition. `just docs-build`
also copies the English `404.html` to the root because GitHub Pages only serves
the root 404 page. Pull requests build the documentation; only pushes to `main`
deploy it.

Zensical's language selector opens the same page in the other edition: it
fetches the `sitemap.xml` below each `extra.alternate` root and looks the
current page up there. This requires that neither edition root be a prefix of
the other, which is why English lives under `en/` rather than at the site root,
and that both editions have the same page paths.

## Local commands

```sh
# Build both editions into site/.
just docs-build

# English or Japanese live preview (one edition only).
just docs-serve
just docs-serve-ja

# Preview of both editions and language switching.
just docs-preview
# http://127.0.0.1:8000/capsula/
```

## Updating a translation

1. Edit the English page and its Japanese counterpart in the same change. Keep
   all technical requirements, restrictions, error cases, examples, and table
   rows.
2. Keep code blocks identical, including comments and expected output. Prose,
   headings, admonition titles, and navigation labels are translated;
   identifiers, hook IDs, configuration keys, CLI flags, environment variables,
   and inline code are not.
3. Give every Japanese heading an explicit ASCII ID equal to the English
   page's generated ID (`## フックの設定 { #hook-configuration }`), so fragment
   links work in both editions.
4. For a new page (for example a new hook), add both translations and both
   navigation entries. For a deletion, remove both.
5. Run `just docs-build`; it fails on renderer warnings.
6. Reviewers check that both editions agree. AI translations are drafts until
   a human has reviewed them.

## Japanese editorial conventions

- Use natural, polite technical Japanese (`です`/`ます`). Avoid literal English
  sentence structure when it obscures the meaning.
- Put a half-width space between Japanese and Latin words or inline code
  (`Capsula のフック`), following the Graphcal documentation.
- Use these terms consistently; retain the English term on first use where
  useful:

  | English | Japanese |
  | --- | --- |
  | run (a recorded execution) | ラン |
  | run directory | ランディレクトリ |
  | hook | フック |
  | vault | ボールト |
  | pre-run / post-run (phase) | 実行前 / 実行後 (フェーズ) |
  | capture | 記録する / キャプチャ |
  | artifact | アーティファクト |
  | project root | プロジェクトルート |
  | abort | 中断 |
  | dirty (Git working tree) | ダーティ (未コミットの変更がある) |
  | glob pattern | glob パターン |
  | environment variable | 環境変数 |
  | server | サーバー |
  | parameter | パラメーター |

- Translate **must**, **cannot**, and **only** as requirements, not suggestions.
- Never translate hook IDs, configuration keys, CLI flags, environment
  variables, file names, or command output.
- If the English source appears wrong, report it and fix both editions in a
  reviewed change; do not silently give the two editions different semantics.
