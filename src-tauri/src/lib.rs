mod ai_bridge;
// note_creator 等からの追加利用を予定（現在は task_parser のみ）
#[allow(dead_code)]
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

use task_parser::{VaultSummary, WeeklyTaskSummary, WeeklyTasks};
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

/// 指定週の 10_Projects/ タスクをプロジェクト別に返す
#[tauri::command]
async fn get_weekly_tasks(
    week_offset: i32,
    state: State<'_, AppState>,
) -> Result<WeeklyTasks, String> {
    let vault_root = state
        .vault_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Vaultが設定されていません")?;

    task_parser::build_weekly_tasks(&vault_root, week_offset).map_err(|e| e.to_string())
}

/// Apple Intelligence の利用可否を返す
#[tauri::command]
fn get_ai_availability() -> bool {
    ai_bridge::is_available()
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

// ---- AI 週次サマリ生成 ----

const WEEKLY_SUMMARY_SYSTEM_PROMPT: &str = "\
あなたはエンジニアの週次振り返りレポートを作成するアシスタントです。\
以下の完了・進行中タスクリストから、プロジェクトごとの週次サマリを作成してください。\
\n\
フォーマット:\n\
- プロジェクトごとに【プロジェクト名】の見出しで区切る\n\
- 各プロジェクトの成果を1〜2文で簡潔に述べる\n\
- 自己言及しない（「今週は…に焦点を当てました」のような書き出しは避ける）\n\
- 成果を直接述べる\n\
- 最後に全体を俯瞰した1文を追加する（任意）\n\
- 自然な日本語で記述する";

const NO_TASKS_MESSAGE: &str = "今週は記録されたタスクがありません。";

fn format_tasks_for_ai(summary: &WeeklyTaskSummary) -> String {
    use std::collections::BTreeMap;
    use std::fmt::Write;

    let mut by_project: BTreeMap<&str, Vec<String>> = BTreeMap::new();

    for task in &summary.completed {
        let entry = by_project.entry(&task.project).or_default();
        let date_str = task
            .done_date
            .map_or(String::new(), |d| format!(" (完了: {d})"));
        entry.push(format!("- [x] {}{date_str}", task.text));
    }

    for task in &summary.started {
        let entry = by_project.entry(&task.project).or_default();
        let date_str = task
            .start
            .map_or(String::new(), |d| format!(" (開始: {d})"));
        entry.push(format!("- [/] {}{date_str}", task.text));
    }

    let mut output = String::new();
    for (project, tasks) in &by_project {
        let _ = writeln!(output, "【{project}】");
        for task in tasks {
            let _ = writeln!(output, "{task}");
        }
        output.push('\n');
    }

    output
}

/// 指定週の AI 週次サマリを生成して返す（ノートへの書き込みは行わない）
#[tauri::command]
async fn generate_weekly_summary(
    week: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if !ai_bridge::is_available() {
        return Err("AI features are not available on this system".to_string());
    }

    let vault_root = state
        .vault_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Vaultが設定されていません")?;

    let (week_start, week_end) =
        task_parser::parse_iso_week(&week).map_err(|e| e.to_string())?;

    let note_path = note_creator::weekly_note_path(&vault_root, &week);
    if !note_path.exists() {
        return Err(format!("Weekly Note が見つかりません: {}", note_path.display()));
    }

    let task_summary =
        task_parser::collect_weekly_done_tasks(&vault_root, week_start, week_end)
            .map_err(|e| e.to_string())?;

    if task_summary.completed.is_empty() && task_summary.started.is_empty() {
        return Ok(NO_TASKS_MESSAGE.to_string());
    }

    let user_prompt = format_tasks_for_ai(&task_summary);
    let system = WEEKLY_SUMMARY_SYSTEM_PROMPT.to_string();

    tokio::task::spawn_blocking(move || ai_bridge::generate(&system, &user_prompt))
        .await
        .map_err(|e| e.to_string())?
}

/// 生成済みサマリを Weekly Note に追記する
#[tauri::command]
async fn save_weekly_summary(
    week: String,
    summary: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let vault_root = state
        .vault_root
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("Vaultが設定されていません")?;

    // ISO 週形式を検証してパストラバーサルを防止
    task_parser::parse_iso_week(&week).map_err(|e| e.to_string())?;

    let note_path = note_creator::weekly_note_path(&vault_root, &week);
    if !note_path.exists() {
        return Err(format!("Weekly Note が見つかりません: {}", note_path.display()));
    }

    let today = chrono::Local::now().format("%Y-%m-%d");
    let with_date = format!("*生成日: {today}*\n\n{summary}");

    note_creator::append_ai_summary(&note_path, &with_date)
        .map_err(|e| e.to_string())
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
            get_weekly_tasks,
            create_note,
            get_ai_availability,
            generate_weekly_summary,
            save_weekly_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
