// vault_watcher.rs
// notifyでVaultのMarkdownファイル変更を監視し、
// "vault:changed" イベントをフロントエンドにemitする。

use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use tauri::{AppHandle, Emitter};

pub fn start_watching(app: AppHandle, vault_root: PathBuf) -> anyhow::Result<()> {
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<DebounceEventResult>();

        let mut debouncer = new_debouncer(Duration::from_millis(500), tx)
            .expect("failed to create debouncer");

        debouncer
            .watcher()
            .watch(&vault_root, RecursiveMode::Recursive)
            .expect("failed to watch vault");

        for result in rx {
            match result {
                Ok(events) => {
                    // Markdownファイルの変更のみフロントに通知
                    let md_changed = events.iter().any(|e| {
                        e.path
                            .extension()
                            .and_then(|s| s.to_str())
                            .map_or(false, |ext| ext == "md")
                    });
                    if md_changed {
                        let _ = app.emit("vault:changed", ());
                    }
                }
                Err(e) => eprintln!("watch error: {e:?}"),
            }
        }
    });

    Ok(())
}
