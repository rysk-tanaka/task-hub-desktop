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

#[allow(clippy::unwrap_used)]
fn task_regex() -> &'static Regex {
    TASK_RE.get_or_init(|| {
        // - [x] タスク名 📅 2026-03-07 ✅ 2026-03-07
        Regex::new(r"^(\s*)-\s*\[(.)\]\s+(.+)$").unwrap()
    })
}

#[allow(clippy::unwrap_used)]
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
        .filter_map(Result::ok)
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
            inbox_count +=
                tasks.iter().filter(|t| t.status == TaskStatus::Todo).count();
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
        .filter(|t| t.status == TaskStatus::Todo && t.due.is_some_and(|d| d < today))
        .cloned()
        .collect();

    Ok(VaultSummary {
        inbox_count,
        due_today,
        overdue,
        projects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ---- parse_tasks ----

    #[test]
    fn parse_simple_todo() {
        let md = "- [ ] Buy milk\n";
        let tasks = parse_tasks(md, "test.md");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].text, "Buy milk");
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(tasks[0].line, 1);
        assert_eq!(tasks[0].source_file, "test.md");
    }

    #[test]
    fn parse_all_statuses() {
        let md = "\
- [ ] todo
- [x] done
- [X] done upper
- [/] in progress
- [?] waiting
- [-] cancelled
";
        let tasks = parse_tasks(md, "s.md");
        assert_eq!(tasks.len(), 6);
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(tasks[1].status, TaskStatus::Done);
        assert_eq!(tasks[2].status, TaskStatus::Done);
        assert_eq!(tasks[3].status, TaskStatus::InProgress);
        assert_eq!(tasks[4].status, TaskStatus::Waiting);
        assert_eq!(tasks[5].status, TaskStatus::Cancelled);
    }

    #[test]
    fn parse_due_and_done_dates() {
        let md = "- [x] Task 📅 2026-03-10 ✅ 2026-03-10\n";
        let tasks = parse_tasks(md, "d.md");
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].due,
            Some(NaiveDate::from_ymd_opt(2026, 3, 10).expect("valid date"))
        );
        assert_eq!(
            tasks[0].done_date,
            Some(NaiveDate::from_ymd_opt(2026, 3, 10).expect("valid date"))
        );
        assert!(tasks[0].start.is_none());
    }

    #[test]
    fn parse_start_date() {
        let md = "- [ ] Task 🛫 2026-04-01\n";
        let tasks = parse_tasks(md, "s.md");
        assert_eq!(
            tasks[0].start,
            Some(NaiveDate::from_ymd_opt(2026, 4, 1).expect("valid date"))
        );
    }

    #[test]
    fn parse_scheduled_date_not_assigned() {
        let md = "- [ ] Task ⏳ 2026-05-01\n";
        let tasks = parse_tasks(md, "s.md");
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].due.is_none());
        assert!(tasks[0].start.is_none());
    }

    #[test]
    fn parse_text_strips_date_metadata() {
        let md = "- [ ] Buy milk 📅 2026-03-10\n";
        let tasks = parse_tasks(md, "t.md");
        assert_eq!(tasks[0].text, "Buy milk");
    }

    #[test]
    fn parse_indented_task() {
        let md = "    - [ ] Sub task\n";
        let tasks = parse_tasks(md, "t.md");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].text, "Sub task");
    }

    #[test]
    fn parse_no_tasks() {
        let md = "# Heading\n\nJust text\n";
        let tasks = parse_tasks(md, "t.md");
        assert!(tasks.is_empty());
    }

    #[test]
    fn parse_multiple_lines_with_line_numbers() {
        let md = "# Project\n- [ ] First\n- [x] Second\nSome text\n- [/] Third\n";
        let tasks = parse_tasks(md, "p.md");
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].line, 2);
        assert_eq!(tasks[1].line, 3);
        assert_eq!(tasks[2].line, 5);
    }

    // ---- build_vault_summary ----

    fn create_test_vault(dir: &Path) {
        let inbox = dir.join("00_Inbox");
        let projects = dir.join("10_Projects");
        let templates = dir.join("Templates");
        let archive = dir.join("40_Archive");

        for d in [&inbox, &projects, &templates, &archive] {
            fs::create_dir_all(d).expect("create dir");
        }

        fs::write(
            inbox.join("inbox.md"),
            "- [ ] Inbox task 1\n- [ ] Inbox task 2\n- [x] Inbox done\n",
        )
        .expect("write inbox");

        fs::write(
            projects.join("MyProject.md"),
            "- [x] Step 1\n- [x] Step 2\n- [ ] Step 3\n- [ ] Step 4\n",
        )
        .expect("write project");

        // Should be excluded from scanning
        fs::write(templates.join("tmpl.md"), "- [ ] Template task\n").expect("write template");
        fs::write(archive.join("old.md"), "- [ ] Archived task\n").expect("write archive");
    }

    #[test]
    fn build_summary_inbox_count() {
        let tmp = tempfile::tempdir().expect("tempdir");
        create_test_vault(tmp.path());

        let summary = build_vault_summary(tmp.path()).expect("summary");
        assert_eq!(summary.inbox_count, 2);
    }

    #[test]
    fn build_summary_projects() {
        let tmp = tempfile::tempdir().expect("tempdir");
        create_test_vault(tmp.path());

        let summary = build_vault_summary(tmp.path()).expect("summary");
        assert_eq!(summary.projects.len(), 1);
        let proj = &summary.projects[0];
        assert_eq!(proj.name, "MyProject");
        assert_eq!(proj.completed, 2);
        assert_eq!(proj.total, 4);
        assert_eq!(proj.percent, 50);
    }

    #[test]
    fn build_summary_excludes_templates_and_archive() {
        let tmp = tempfile::tempdir().expect("tempdir");
        create_test_vault(tmp.path());

        let summary = build_vault_summary(tmp.path()).expect("summary");
        // inbox_count should not include Templates/Archive tasks
        // (only 2 from 00_Inbox, not 4)
        assert_eq!(summary.inbox_count, 2);
    }

    #[test]
    fn build_summary_empty_vault() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let summary = build_vault_summary(tmp.path()).expect("summary");
        assert_eq!(summary.inbox_count, 0);
        assert!(summary.projects.is_empty());
        assert!(summary.due_today.is_empty());
        assert!(summary.overdue.is_empty());
    }

    #[test]
    fn build_summary_overdue_tasks() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("00_Inbox");
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(
            dir.join("tasks.md"),
            "- [ ] Overdue 📅 2020-01-01\n- [ ] Future 📅 2099-12-31\n",
        )
        .expect("write");

        let summary = build_vault_summary(tmp.path()).expect("summary");
        assert_eq!(summary.overdue.len(), 1);
        assert_eq!(summary.overdue[0].text, "Overdue");
    }
}
