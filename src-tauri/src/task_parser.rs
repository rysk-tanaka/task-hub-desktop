use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

// CLAUDE.md の記法に準拠したタスクステータス
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,        // [ ]
    Done,        // [x]
    InProgress,  // [/]
    Waiting,     // [?]
    Cancelled,   // [-]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub text: String,
    pub status: TaskStatus,
    pub due: Option<NaiveDate>,
    pub done_date: Option<NaiveDate>,
    pub start: Option<NaiveDate>,
    pub source_file: String, // 相対パス（Vault root からの）
    pub line: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultSummary {
    pub inbox_count: usize,
    pub due_today: Vec<Task>,
    pub overdue: Vec<Task>,
    pub projects: Vec<ProjectProgress>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectProgress {
    pub name: String,
    pub file: String,
    pub completed: usize,
    pub total: usize,
    pub percent: u32,
}

static TASK_RE: OnceLock<Regex> = OnceLock::new();
static DATE_RE: OnceLock<Regex> = OnceLock::new();

fn task_regex() -> &'static Regex {
    TASK_RE.get_or_init(|| {
        // - [x] タスク名 📅 2026-03-07 ✅ 2026-03-07
        Regex::new(r"^(\s*)-\s*\[(.)\]\s+(.+)$").unwrap()
    })
}

fn date_regex() -> &'static Regex {
    DATE_RE.get_or_init(|| {
        // 📅 2026-03-07 / ✅ 2026-03-07 / 🛫 2026-03-07
        Regex::new(r"(📅|✅|🛫|⏳)\s*(\d{4}-\d{2}-\d{2})").unwrap()
    })
}

fn parse_status(c: char) -> TaskStatus {
    match c {
        'x' | 'X' => TaskStatus::Done,
        '/' => TaskStatus::InProgress,
        '?' => TaskStatus::Waiting,
        '-' => TaskStatus::Cancelled,
        _ => TaskStatus::Todo,
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Markdownファイルの内容からタスク一覧を抽出する
pub fn parse_tasks(content: &str, source_file: &str) -> Vec<Task> {
    let task_re = task_regex();
    let date_re = date_regex();
    let mut tasks = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let Some(caps) = task_re.captures(line) else {
            continue;
        };
        let status_char = caps[2].chars().next().unwrap_or(' ');
        let text_raw = caps[3].trim().to_string();

        let mut due = None;
        let mut done_date = None;
        let mut start = None;

        for date_cap in date_re.captures_iter(&text_raw) {
            let emoji = &date_cap[1];
            let date = parse_date(&date_cap[2]);
            match emoji {
                "📅" => due = date,
                "✅" => done_date = date,
                "🛫" => start = date,
                _ => {}
            }
        }

        // 絵文字とメタデータを除いたテキスト
        let text = date_re.replace_all(&text_raw, "").trim().to_string();

        tasks.push(Task {
            text,
            status: parse_status(status_char),
            due,
            done_date,
            start,
            source_file: source_file.to_string(),
            line: line_idx + 1,
        });
    }

    tasks
}

/// Vaultを走査してサマリーを生成する
pub fn build_vault_summary(vault_root: &Path) -> anyhow::Result<VaultSummary> {
    use walkdir::WalkDir;

    let today = chrono::Local::now().date_naive();
    let mut all_tasks: Vec<Task> = Vec::new();
    let mut inbox_count = 0usize;
    let mut projects: Vec<ProjectProgress> = Vec::new();

    for entry in WalkDir::new(vault_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(vault_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Templates / Archive は除外
        if rel.starts_with("Templates") || rel.starts_with("40_Archive") {
            continue;
        }

        let content = std::fs::read_to_string(path)?;
        let tasks = parse_tasks(&content, &rel);

        // Inbox カウント
        if rel.starts_with("00_Inbox") {
            inbox_count += tasks.iter().filter(|t| t.status == TaskStatus::Todo).count();
        }

        // プロジェクト進捗
        if rel.starts_with("10_Projects") {
            let total = tasks.len();
            if total > 0 {
                let completed = tasks.iter().filter(|t| t.status == TaskStatus::Done).count();
                let percent = ((completed as f64 / total as f64) * 100.0).round() as u32;
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                projects.push(ProjectProgress {
                    name,
                    file: rel.clone(),
                    completed,
                    total,
                    percent,
                });
            }
        }

        all_tasks.extend(tasks);
    }

    let due_today: Vec<Task> = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Todo && t.due == Some(today))
        .cloned()
        .collect();

    let overdue: Vec<Task> = all_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Todo && t.due.map_or(false, |d| d < today))
        .cloned()
        .collect();

    Ok(VaultSummary {
        inbox_count,
        due_today,
        overdue,
        projects,
    })
}
