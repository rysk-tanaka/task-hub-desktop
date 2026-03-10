# task-hub-desktop

Obsidian Vaultと同じMarkdownファイルを読み書きするコンパニオンデスクトップアプリ。
**Tauri v2 + React + Rust** 構成。

## 機能

- **GTDダッシュボード**: Inbox件数・期限タスク・プロジェクト進捗をリアルタイム表示
- **Daily/Weekly Note生成**: 既存Templater構文を展開してVaultに保存

## ディレクトリ構成

```tree
task-hub-desktop/
├── src/                        # React フロントエンド
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── Dashboard.tsx       # GTDダッシュボード
│   │   ├── TaskList.tsx        # タスク一覧
│   │   └── NoteCreator.tsx     # Daily/Weekly Note生成UI
│   ├── hooks/
│   │   └── useVault.ts         # Vault監視・データ取得
│   └── types.ts                # 共有型定義
│
├── src-tauri/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Tauriコマンド定義（invoke entrypoints）
│       ├── task_parser.rs      # Markdownタスク行のパース・集計
│       ├── frontmatter.rs      # YAMLフロントマターのパース
│       ├── template.rs         # Templater構文の展開
│       ├── note_creator.rs     # Daily/Weekly Note生成・重複チェック
│       └── vault_watcher.rs    # notifyによるファイル監視
│
├── package.json
├── vite.config.ts
└── tauri.conf.json
```

## セットアップ

```bash
# 依存インストール
pnpm install

# 開発サーバー起動
pnpm tauri dev

# ビルド
pnpm tauri build
```

## Rustクレート

| クレート | 用途 |
| --- | --- |
| `serde_yaml` | YAMLフロントマターのパース |
| `notify` | Vaultのファイル変更監視 |
| `chrono` | 日付処理・ISO週番号計算 |
| `walkdir` | Vault内Markdownの再帰スキャン |
| `regex` | タスク行・Templater構文の抽出 |
| `tauri-plugin-shell` | sidecar呼び出し（将来拡張用） |

## Templater構文の対応範囲

Daily/Weeklyテンプレートで使われている構文のみ実装。

| 構文 | 展開例 |
| --- | --- |
| `<% tp.date.now("YYYY-MM-DD") %>` | `2026-03-07` |
| `<% tp.date.now("YYYY-[W]ww") %>` | `2026-W10` |
| `<% tp.date.weekday("YYYY-MM-DD", 1) %>` | `2026-03-02`（当週月曜） |
| `<% tp.date.weekday("YYYY-MM-DD", 0, 7) %>` | `2026-03-08`（翌週日曜） |
| `<% tp.file.title %>` | ファイル名（拡張子なし） |

Dataview/Tasksクエリブロックはそのまま素通し（Obsidianが評価する）。

## アーキテクチャ原則

```text
React Component
    ↕ invoke() / listen()
Tauri Command (lib.rs)
    ├── task_parser
    ├── frontmatter
    ├── template
    ├── note_creator
    └── vault_watcher  →  emit("vault:changed") → React
```

- `invoke()` は `src/hooks/useVault.ts` にのみ記述（Componentから直接呼ばない）
- ファイル変更は `vault_watcher` が検知し、イベントでフロントに通知
- Vaultパスはアプリ設定に保存し、起動時に復元
