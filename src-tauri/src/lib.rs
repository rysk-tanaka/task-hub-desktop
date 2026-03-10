mod frontmatter;
mod note_creator;
mod task_parser;
mod template;
mod vault_watcher;

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;

use task_parser::VaultSummary;
use vault_watcher::VaultDebouncer;

const STORE_FILE: &str = "settings.json";
const STORE_KEY_VAULT_ROOT: &str = "vault_root";

// ---- アプリ状態 ----

struct AppState {
    vault_root: Mutex<Option<PathBuf>>,
    watcher: Mutex<Option<VaultDebouncer>>,
}

// ---- Tauriコマンド ----

/// Vaultのルートパスを設定する
#[tauri::command]
async fn set_vault_root(
    path: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        return Err(format!("パスが存在しません: {}", path.display()));
    }

    // ファイル監視を先に開始し、失敗時は状態を変更しない
    let debouncer = vault_watcher::start_watching(&app, path.clone()).map_err(|e| e.to_string())?;

    {
        let mut guard = state.vault_root.lock().map_err(|e| e.to_string())?;
        *guard = Some(path.clone());
    }
    {
        let mut guard = state.watcher.lock().map_err(|e| e.to_string())?;
        *guard = Some(debouncer);
    }

    // store に永続化
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(STORE_KEY_VAULT_ROOT, path.to_string_lossy().to_string());

    Ok(())
}

/// 現在設定されているVaultルートを返す
#[tauri::command]
fn get_vault_root(state: State<'_, AppState>) -> Option<String> {
    state
        .vault_root
        .lock()
        .ok()?
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
}

/// GTDサマリー（Inbox件数・期限タスク・プロジェクト進捗）を返す
#[tauri::command]
async fn get_vault_summary(state: State<'_, AppState>) -> Result<VaultSummary, String> {
    let vault_root = state
        .vault_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Vaultが設定されていません")?;

    task_parser::build_vault_summary(&vault_root).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateNoteRequest {
    kind: NoteKind,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NoteKind {
    Daily,
    Weekly,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateNoteResponse {
    /// 作成または既存のファイルの絶対パス
    path: String,
    /// true = 新規作成, false = 既存ファイル
    created: bool,
}

/// Daily / Weekly Note を生成する（既存なら既存パスを返す）
#[tauri::command]
async fn create_note(
    request: CreateNoteRequest,
    state: State<'_, AppState>,
) -> Result<CreateNoteResponse, String> {
    let vault_root = state
        .vault_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Vaultが設定されていません")?;

    note_creator::create_note(&vault_root, &request.kind)
        .map_err(|e| e.to_string())
        .map(|(path, created)| CreateNoteResponse {
            path: path.to_string_lossy().to_string(),
            created,
        })
}

// ---- 起動時復元 ----

/// store から前回の Vault パスを復元し、存在すればメモリ + ファイル監視を開始する
fn restore_vault_root(app: &AppHandle) {
    let Ok(store) = app.store(STORE_FILE) else {
        return;
    };
    let Some(val) = store.get(STORE_KEY_VAULT_ROOT) else {
        return;
    };
    let Ok(path_str) = serde_json::from_value::<String>(val) else {
        return;
    };
    let path = PathBuf::from(&path_str);
    if !path.exists() {
        return;
    }

    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if let Ok(mut guard) = state.vault_root.lock() {
        *guard = Some(path.clone());
    }

    if let Ok(debouncer) = vault_watcher::start_watching(app, path) {
        if let Ok(mut guard) = state.watcher.lock() {
            *guard = Some(debouncer);
        }
    }
}

// ---- エントリーポイント ----

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            vault_root: Mutex::new(None),
            watcher: Mutex::new(None),
        })
        .setup(|app| {
            restore_vault_root(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_vault_root,
            get_vault_root,
            get_vault_summary,
            create_note,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
