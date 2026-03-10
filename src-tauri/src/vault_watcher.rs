// vault_watcher.rs
// notifyでVaultのMarkdownファイル変更を監視し、
// "vault:changed" イベントをフロントエンドにemitする。
//
// 返却される Debouncer を AppState に保持し、
// 新しいウォッチャー開始時に古いものを drop することで停止する。

use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use tauri::{AppHandle, Emitter};

pub type VaultDebouncer =
    notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>;

/// Vault ディレクトリの監視を開始し、Debouncer ハンドルを返す。
/// 呼び出し側はハンドルを保持し、不要になったら drop して監視を停止すること。
pub fn start_watching(app: &AppHandle, vault_root: PathBuf) -> anyhow::Result<VaultDebouncer> {
    let app = app.clone();
    let mut debouncer =
        new_debouncer(
            Duration::from_millis(500),
            move |result: DebounceEventResult| match result {
                Ok(events) => {
                    let md_changed = events.iter().any(|e| {
                        e.path.extension().and_then(|s| s.to_str()) == Some("md")
                    });
                    if md_changed {
                        let _ = app.emit("vault:changed", ());
                    }
                }
                Err(e) => eprintln!("watch error: {e:?}"),
            },
        )?;

    debouncer
        .watcher()
        .watch(&vault_root, RecursiveMode::Recursive)?;

    Ok(debouncer)
}
