# TODO

## 未使用モジュールの活用

- [x] ~~`frontmatter.rs` が `#[allow(dead_code)]` のまま未使用~~ → `task_parser.rs` で本文分離・`archived` スキップに活用済み
- [ ] `frontmatter.rs` のタグベースフィルタリングやメタデータ表示への拡張

## パフォーマンス改善

- [ ] `walk_vault_tasks` の二重パース・2回走査を最適化 (#6)
- [x] ~~日付パースロジックの重複をヘルパー関数に切り出す (#7)~~ → `extract_date_metadata` ヘルパー関数に集約

## バグ修正

- [ ] 期限超過判定のタイムゾーン問題を修正 (#8)

## 機能拡張候補

- [ ] タスクのフィルタリング・検索（ステータス、タグ、日付範囲）
- [ ] タスクの編集・ステータス変更（現在は読み取り専用）
- [ ] サイドカー呼び出し対応（Cargo.toml に「将来拡張用」コメントあり）
- [ ] Templater 構文の拡張（`tp.date.now` / `tp.date.weekday` / `tp.file.title` 以外）

## Apple Intelligence 連携

- [x] ~~Foundation Models Swift Bridge 基盤（#17）~~ → `swift-rs` + SwiftPM で Rust↔Swift FFI 実装済み
- [ ] スタンドアップ報告文の自動生成（#18）
- [ ] Weekly Note サマリの自動生成（#19）
- [ ] 自然言語 Vault 検索（#20）

## テスト

- [ ] フロントエンド E2E テスト（Playwright / WebdriverIO 等）
- [ ] `lib.rs` コマンド層の統合テスト（`AppHandle` モック or Tauri test utilities）
- [ ] `vault_watcher.rs` の統合テスト

## CI / リリース

- [ ] フロントエンド E2E テストの CI ジョブ追加
- [x] ~~Tauri アプリのリリースビルド・配布パイプライン（GitHub Releases 等）~~ → `auto-release.yml` で macOS / Linux ビルド・配布を自動化
- [x] ~~配布物にアイコンライセンスファイルを同梱する (#12)~~ → `bundle.resources` で `assets/LICENSE` を `ICON_LICENSE` として同梱
- [ ] Homebrew tap による自動インストール対応 (#14)
