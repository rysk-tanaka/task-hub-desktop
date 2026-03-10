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

        let Ok(mut debouncer) = new_debouncer(Duration::from_millis(500), tx) else {
            eprintln!("failed to create debouncer");
            return;
        };

        if debouncer
            .watcher()
            .watch(&vault_root, RecursiveMode::Recursive)
            .is_err()
        {
            eprintln!("failed to watch vault: {}", vault_root.display());
            return;
        }

        for result in rx {
            match result {
                Ok(events) => {
                    let md_changed = events.iter().any(|e| {
                        e.path.extension().and_then(|s| s.to_str()) == Some("md")
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
