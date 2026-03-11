# TODO

## 未使用モジュールの活用

- [x] ~~`frontmatter.rs` が `#[allow(dead_code)]` のまま未使用~~ → `task_parser.rs` で本文分離・`archived` スキップに活用済み
- [ ] `frontmatter.rs` のタグベースフィルタリングやメタデータ表示への拡張

## 機能拡張候補

- [ ] タスクのフィルタリング・検索（ステータス、タグ、日付範囲）
- [ ] タスクの編集・ステータス変更（現在は読み取り専用）
- [ ] サイドカー呼び出し対応（Cargo.toml に「将来拡張用」コメントあり）
- [ ] Templater 構文の拡張（`tp.date.now` / `tp.date.weekday` / `tp.file.title` 以外）

## テスト

- [ ] フロントエンド E2E テスト（Playwright / WebdriverIO 等）
- [ ] `lib.rs` コマンド層の統合テスト（`AppHandle` モック or Tauri test utilities）
- [ ] `vault_watcher.rs` の統合テスト

## CI / リリース

- [ ] フロントエンド E2E テストの CI ジョブ追加
- [ ] Tauri アプリのリリースビルド・配布パイプライン（GitHub Releases 等）
