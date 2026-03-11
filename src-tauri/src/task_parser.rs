use chrono::{Datelike, Duration, NaiveDate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

use crate::frontmatter;

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

struct VaultFile {
    rel_path: String,
    tasks: Vec<Task>,
}

/// Vault 内の .md ファイルを走査し、除外・frontmatter 処理済みのタスク一覧を返す
fn walk_vault_tasks(vault_root: &Path) -> anyhow::Result<Vec<VaultFile>> {
    use walkdir::WalkDir;

    let mut files = Vec::new();

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

        // Templates / Archive / README は除外
        if rel.starts_with("Templates") || rel.starts_with("40_Archive") || rel == "README.md" {
            continue;
        }

        let content = std::fs::read_to_string(path)?;
        let doc = frontmatter::parse_document(&content);

        // archived フラグによるスキップ（YAML ブール型 `true`/`false` のみ認識。
        // 文字列 `"true"` や YAML 1.1 の `yes` はスキップ対象外）
        if let Some(fm) = &doc.frontmatter {
            if fm.get_bool("archived").unwrap_or(false) {
                continue;
            }
        }

        // フロントマターが有効にパースできた場合のみ本文を分離する。
        // デリミタ (---) は存在するが YAML パースが失敗した場合も frontmatter は None になる。
        // その場合は content 全体をフォールバックとして使い、タスクの取りこぼしを防ぐ
        // （誤検知より漏れを避ける優先設計）。
        let (body, body_line_offset) = if doc.frontmatter.is_some() {
            // フロントマター行数 + 開閉 `---` 分（2行）をオフセットとして使う
            let offset = doc
                .raw_yaml
                .as_ref()
                .map_or(0, |y| y.lines().count() + 2);
            (doc.body.as_str(), offset)
        } else {
            (content.as_str(), 0)
        };
        let mut tasks = parse_tasks(body, &rel);
        for task in &mut tasks {
            task.line += body_line_offset;
        }

        files.push(VaultFile {
            rel_path: rel,
            tasks,
        });
    }

    Ok(files)
}

/// Vaultを走査してサマリーを生成する
pub fn build_vault_summary(vault_root: &Path) -> anyhow::Result<VaultSummary> {
    let today = chrono::Local::now().date_naive();
    let vault_files = walk_vault_tasks(vault_root)?;

    let mut all_tasks: Vec<Task> = Vec::new();
    let mut inbox_count = 0usize;
    let mut projects: Vec<ProjectProgress> = Vec::new();

    for vf in &vault_files {
        // Inbox カウント
        if vf.rel_path.starts_with("00_Inbox") {
            inbox_count += vf
                .tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Todo)
                .count();
        }

        // プロジェクト進捗
        if vf.rel_path.starts_with("10_Projects") {
            let total = vf.tasks.len();
            if total > 0 {
                let completed = vf
                    .tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Done)
                    .count();
                let percent = ((completed as f64 / total as f64) * 100.0).round() as u32;
                let name = Path::new(&vf.rel_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                projects.push(ProjectProgress {
                    name,
                    file: vf.rel_path.clone(),
                    completed,
                    total,
                    percent,
                });
            }
        }

        all_tasks.extend(vf.tasks.clone());
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectTasks {
    pub name: String,
    pub file: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeeklyTasks {
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub projects: Vec<ProjectTasks>,
}

/// 指定週の `10_Projects/` タスクをプロジェクト別に返す
pub fn build_weekly_tasks(vault_root: &Path, week_offset: i32) -> anyhow::Result<WeeklyTasks> {
    let today = chrono::Local::now().date_naive();
    let days_since_monday = i64::from(today.weekday().num_days_from_monday());
    let this_monday = today - Duration::days(days_since_monday);
    let week_start = this_monday + Duration::weeks(i64::from(week_offset));
    let week_end = week_start + Duration::days(6);

    let vault_files = walk_vault_tasks(vault_root)?;

    let mut projects: Vec<ProjectTasks> = Vec::new();

    for vf in &vault_files {
        if !vf.rel_path.starts_with("10_Projects") {
            continue;
        }

        let week_tasks: Vec<Task> = vf
            .tasks
            .iter()
            .filter(|t| t.start.is_some_and(|s| s >= week_start && s <= week_end))
            .cloned()
            .collect();

        if week_tasks.is_empty() {
            continue;
        }

        let name = Path::new(&vf.rel_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        projects.push(ProjectTasks {
            name,
            file: vf.rel_path.clone(),
            tasks: week_tasks,
        });
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(WeeklyTasks {
        week_start,
        week_end,
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
        fs::write(dir.join("README.md"), "- [ ] Sample task\n").expect("write readme");
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
    fn build_summary_skips_archived_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let inbox = tmp.path().join("00_Inbox");
        fs::create_dir_all(&inbox).expect("create dir");

        fs::write(
            inbox.join("active.md"),
            "- [ ] Active task\n",
        )
        .expect("write active");
        fs::write(
            inbox.join("archived.md"),
            "---\narchived: true\n---\n- [ ] Should be skipped\n",
        )
        .expect("write archived");

        let summary = build_vault_summary(tmp.path()).expect("summary");
        assert_eq!(summary.inbox_count, 1);
    }

    #[test]
    fn build_summary_ignores_frontmatter_checkboxes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let inbox = tmp.path().join("00_Inbox");
        fs::create_dir_all(&inbox).expect("create dir");

        // YAML ブロックスカラー内の `- [ ]` がタスクとして誤検知されないこと
        fs::write(
            inbox.join("note.md"),
            "---\ndescription: |\n  - [ ] Not a task\n---\n- [ ] Real task\n",
        )
        .expect("write");

        let summary = build_vault_summary(tmp.path()).expect("summary");
        assert_eq!(summary.inbox_count, 1);
    }

    #[test]
    fn build_summary_correct_line_numbers_with_frontmatter() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let inbox = tmp.path().join("00_Inbox");
        fs::create_dir_all(&inbox).expect("create dir");

        // フロントマター4行 + "# Heading\n" = 5行目, "- [ ] Task\n" = 6行目
        fs::write(
            inbox.join("note.md"),
            "---\ntitle: My Note\ntags: [a]\n---\n# Heading\n- [ ] Task 📅 2020-01-01\n",
        )
        .expect("write");

        let summary = build_vault_summary(tmp.path()).expect("summary");
        assert_eq!(summary.overdue.len(), 1);
        assert_eq!(summary.overdue[0].line, 6);
    }

    #[test]
    fn build_summary_preserves_tasks_with_hr_start() {
        // 先頭 --- が水平線（フロントマターではない）のファイルでもタスクが失われないこと
        let tmp = tempfile::tempdir().expect("tempdir");
        let inbox = tmp.path().join("00_Inbox");
        fs::create_dir_all(&inbox).expect("create dir");

        fs::write(
            inbox.join("hr.md"),
            "---\n- [ ] Task after hr\n---\n- [ ] Task after second hr\n",
        )
        .expect("write");

        let summary = build_vault_summary(tmp.path()).expect("summary");
        // 両方のタスクが検出されるべき（フロントマターとして無効なため全体がパースされる）
        assert_eq!(summary.inbox_count, 2);
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

    // ---- build_weekly_tasks ----

    fn create_weekly_test_vault(dir: &Path, monday: NaiveDate) {
        let projects = dir.join("10_Projects");
        let inbox = dir.join("00_Inbox");
        fs::create_dir_all(&projects).expect("create dir");
        fs::create_dir_all(&inbox).expect("create dir");

        let wed = monday + Duration::days(2);
        let fri = monday + Duration::days(4);
        let prev_week = monday - Duration::days(3);

        fs::write(
            projects.join("Alpha.md"),
            format!(
                "- [ ] In-week task 🛫 {wed}\n- [x] Done task 🛫 {fri}\n- [ ] Out-of-range 🛫 {prev_week}\n- [ ] No start date\n"
            ),
        )
        .expect("write alpha");

        fs::write(
            projects.join("Beta.md"),
            format!("- [/] Beta task 🛫 {monday}\n"),
        )
        .expect("write beta");

        // Inbox tasks should not appear
        fs::write(
            inbox.join("inbox.md"),
            format!("- [ ] Inbox task 🛫 {wed}\n"),
        )
        .expect("write inbox");
    }

    #[test]
    fn weekly_tasks_filters_by_start_date() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let today = chrono::Local::now().date_naive();
        let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        create_weekly_test_vault(tmp.path(), monday);

        let weekly = build_weekly_tasks(tmp.path(), 0).expect("weekly");
        assert_eq!(weekly.week_start, monday);
        assert_eq!(weekly.week_end, monday + Duration::days(6));

        // Alpha: 2 tasks in range (wed, fri), Beta: 1 task (monday)
        assert_eq!(weekly.projects.len(), 2);

        let alpha = weekly.projects.iter().find(|p| p.name == "Alpha").expect("Alpha");
        assert_eq!(alpha.tasks.len(), 2);

        let beta = weekly.projects.iter().find(|p| p.name == "Beta").expect("Beta");
        assert_eq!(beta.tasks.len(), 1);
    }

    #[test]
    fn weekly_tasks_excludes_non_project_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let today = chrono::Local::now().date_naive();
        let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        create_weekly_test_vault(tmp.path(), monday);

        let weekly = build_weekly_tasks(tmp.path(), 0).expect("weekly");
        // Inbox task with start date in range should NOT appear
        let names: Vec<&str> = weekly.projects.iter().map(|p| p.name.as_str()).collect();
        assert!(!names.contains(&"inbox"));
    }

    #[test]
    fn weekly_tasks_excludes_tasks_without_start() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let today = chrono::Local::now().date_naive();
        let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        create_weekly_test_vault(tmp.path(), monday);

        let weekly = build_weekly_tasks(tmp.path(), 0).expect("weekly");
        let alpha = weekly.projects.iter().find(|p| p.name == "Alpha").expect("Alpha");
        // "No start date" task should not be included
        assert!(alpha.tasks.iter().all(|t| t.start.is_some()));
    }

    #[test]
    fn weekly_tasks_week_offset() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let today = chrono::Local::now().date_naive();
        let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        create_weekly_test_vault(tmp.path(), monday);

        // Previous week should not contain any of the current week's tasks
        let prev = build_weekly_tasks(tmp.path(), -1).expect("prev week");
        let prev_task_count: usize = prev.projects.iter().map(|p| p.tasks.len()).sum();
        // Only 1 task from Alpha has prev_week start date
        assert_eq!(prev_task_count, 1);
    }

    #[test]
    fn weekly_tasks_sorted_by_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let today = chrono::Local::now().date_naive();
        let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
        create_weekly_test_vault(tmp.path(), monday);

        let weekly = build_weekly_tasks(tmp.path(), 0).expect("weekly");
        let names: Vec<&str> = weekly.projects.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Beta"]);
    }

    #[test]
    fn weekly_tasks_empty_vault() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let weekly = build_weekly_tasks(tmp.path(), 0).expect("weekly");
        assert!(weekly.projects.is_empty());
    }
}
