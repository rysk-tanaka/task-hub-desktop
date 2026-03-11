# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code)
when working with code in this repository.

## プロジェクト概要

Obsidian Vault の Markdown ファイルを読み書きするコンパニオンデスクトップアプリ。
Tauri v2 (Rust) + React (TypeScript) + Vite 構成。パッケージマネージャは pnpm。

## 開発コマンド

```bash
pnpm install          # 依存インストール
pnpm tauri dev        # 開発サーバー起動（フロント + Rust）
pnpm tauri build      # プロダクションビルド
pnpm lint             # Biome チェック（lint + format + import sort）
pnpm lint:fix         # Biome 自動修正
pnpm format           # Biome フォーマット
```

Rust のテスト・lint:

```bash
cd src-tauri && cargo test                                  # 全テスト
cd src-tauri && cargo test template::tests                  # 特定モジュールのテスト
cd src-tauri && cargo test template::tests::test_moment     # 単一テスト（前方一致）
cd src-tauri && cargo check                                 # 型チェックのみ（高速）
cd src-tauri && cargo clippy                                # Clippy lint（pedantic 有効）
cd src-tauri && cargo tarpaulin --skip-clean --out stdout    # カバレッジ計測
```

TypeScript の型チェック:

```bash
pnpm exec tsc --noEmit -p tsconfig.app.json
```

ルートの `tsconfig.json` はプロジェクト参照構成（`files: []`）のため、`-p tsconfig.app.json` を指定しないと `src/` の型チェックが実行されない。

## アーキテクチャ

フロントエンド (React) とバックエンド (Rust/Tauri) は Tauri IPC (`invoke()` / `listen()`) で通信する。

### 通信規約

- `invoke()` 呼び出しは `src/hooks/useVault.ts` に集約する。コンポーネントから直接 `invoke()` を呼ばない
- Vault のファイル変更は `vault_watcher` が検知し、`emit("vault:changed")` でフロントに通知する
- 型定義は Rust 側 (`task_parser.rs` の struct) と TypeScript 側 (`src/types.ts`) で手動同期が必要

### Tauri コマンド（`lib.rs` で定義）

| コマンド | 機能 |
| --- | --- |
| `set_vault_root` | Vault パス設定 + ファイル監視開始 |
| `get_vault_root` | 現在の Vault パス取得 |
| `get_vault_summary` | GTD サマリー（Inbox 件数・期限タスク・プロジェクト進捗） |
| `create_note` | Daily/Weekly Note 生成（テンプレート展開、既存なら既存パスを返す） |

コマンド関数は `lib.rs` 内で非 `pub` にすること（lib/bin 間のマクロ重複を回避するため）。

### Rust モジュール構成

| モジュール | 責務 |
| --- | --- |
| `task_parser` | Markdown タスク行パース、Vault 走査・集計（frontmatter で本文分離） |
| `frontmatter` | YAML フロントマター解析・操作・シリアライズ |
| `template` | Obsidian Templater 構文のサブセット展開 |
| `note_creator` | Daily/Weekly Note ファイル生成・重複チェック |
| `vault_watcher` | notify による Vault ファイル監視・イベント emit |

### データフロー例: Daily Note 作成

```text
Dashboard → useVault.createNote("daily")
  → invoke("create_note") → lib.rs → note_creator.rs
    → Templates/daily-template.md 読み込み
    → template::expand() で Templater 構文展開
    → 50_Daily/{YYYY-MM-DD}.md に書き込み
  ← CreateNoteResponse { path, created }
→ dialog plugin の ask() で確認ダイアログ表示
→ 承認時に shell plugin の open() で obsidian://open?path=... を起動
```

### frontmatter と task_parser の連携

`build_vault_summary` は `frontmatter::parse_document` で本文を分離してからタスクをパースする。

- **フロントマター内の誤検知防止**: YAML 内のチェックボックス風テキストをタスクとして検出しない
- **`archived: true` スキップ**: フロントマターに `archived: true` があるファイルは集計対象外
- **行番号オフセット**: フロントマター分の行数を加算して正確なタスク行番号を維持
- **パース失敗時の安全策**: フロントマターとして無効（水平線 `---` 等）な場合は元の content 全体をパースする

## Vault ディレクトリ規約

タスク集計はディレクトリ接頭辞で判定する。

- `00_Inbox/` → Inbox 未完了タスクをカウント
- `10_Projects/` → プロジェクト進捗を集計
- `Templates/`, `40_Archive/`, `README.md` → 走査対象から除外
- `50_Daily/` → Daily Note 出力先
- `60_Weekly/` → Weekly Note 出力先

## タスク記法

Obsidian Tasks プラグイン互換のチェックボックス記法。

- `[ ]` Todo, `[x]` Done, `[/]` InProgress, `[?]` Waiting, `[-]` Cancelled
- 日付メタデータ: `📅`(due), `✅`(done), `🛫`(start), `⏳`(scheduled)

## Templater 対応範囲

`template.rs` は moment.js 形式のフォーマット文字列を chrono に変換する。
対応構文: `tp.date.now()`, `tp.date.weekday()`, `tp.file.title`。
Dataview/Tasks のコードブロックはそのまま素通し。

`moment_to_chrono()` の replace チェーンは順序依存。長いトークン（`DDD`）を短いトークン（`DD`）より先に処理すること。

## CI

GitHub Actions で `lint.yml`（Frontend + Backend）と `test.yml`（Backend）を実行する。
Backend の clippy は `-- -D warnings` 付きで全警告をエラー扱いにしている。

Clippy の `unwrap_used` / `expect_used` は Cargo.toml で `warn` レベルに設定しているため、
プロダクションコードで使用する場合は `#[allow(clippy::unwrap_used)]` 等で個別許容すること。
正規表現リテラルの初期化や `run()` のエントリーポイントなど、パニックが許容される箇所に限定する。

## 依存管理

Renovate（GitHub App）で依存の自動更新 PR を作成する。設定は `renovate.json`。
対象マネージャ: `npm`, `cargo`, `github-actions`, `pre-commit`。
マイナー・パッチは automerge、メジャーは手動レビュー。

## 注意事項

- Tauri プラグイン: `shell`, `fs`, `store`（Vault パス永続化）, `dialog`（フォルダ選択・確認ダイアログ）を使用。capabilities/default.json で権限管理
- shell プラグインの open スコープは `tauri.conf.json` の `plugins.shell.open` で正規表現制御（`obsidian://` 等を許可）
- エラーハンドリング: Rust 側は `anyhow::Result` → `.map_err(|e| e.to_string())` で文字列化して IPC 返却
- アイコン: `src-tauri/icons/` に RGBA PNG が必要（`tauri::generate_context!()` がコンパイル時に検証する）

## テスト方針

- `task_parser`, `template`, `note_creator`, `frontmatter` はビジネスロジック中心のため単体テスト対象
- `lib.rs`（Tauri コマンド層）、`vault_watcher.rs` は `AppHandle`/`State` 依存のため単体テスト困難。E2E で担保する想定
- テスト内のファイルシステム操作には `tempfile` crate（dev-dependencies）を使用
- テストコード内では `expect("reason")` を許容（Clippy `expect_used` lint はプロダクションコード向け）
