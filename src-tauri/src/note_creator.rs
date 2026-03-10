// note_creator.rs
// Daily / Weekly Note を生成する。
// 既存ファイルが存在する場合は作成せずそのパスを返す。

use std::path::{Path, PathBuf};

use chrono::{Datelike, IsoWeek, Local};

use crate::template;

/// (出力ファイルパス, 新規作成フラグ)
pub fn create_note(
    vault_root: &Path,
    kind: &crate::NoteKind,
) -> anyhow::Result<(PathBuf, bool)> {
    let (output_path, template_path, file_title) = match *kind {
        crate::NoteKind::Daily => {
            let today = Local::now().format("%Y-%m-%d").to_string();
            let out = vault_root.join("50_Daily").join(format!("{today}.md"));
            let tmpl = vault_root.join("Templates").join("daily-template.md");
            (out, tmpl, today)
        }
        crate::NoteKind::Weekly => {
            let now = Local::now();
            let iso: IsoWeek = now.iso_week();
            let title = format!("{:04}-W{:02}", iso.year(), iso.week());
            let out = vault_root.join("60_Weekly").join(format!("{title}.md"));
            let tmpl = vault_root.join("Templates").join("weekly-template.md");
            (out, tmpl, title)
        }
    };

    // 既存ファイルがあればそのまま返す
    if output_path.exists() {
        return Ok((output_path, false));
    }

    // 出力先ディレクトリを作成
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // テンプレートを読み込んで展開
    let template_content = std::fs::read_to_string(&template_path)
        .unwrap_or_else(|_| default_template(kind, &file_title));

    let expanded = template::expand(&template_content, &file_title)?;
    std::fs::write(&output_path, expanded.as_bytes())?;

    Ok((output_path, true))
}

/// テンプレートファイルが見つからない場合のフォールバック
fn default_template(kind: &crate::NoteKind, title: &str) -> String {
    match kind {
        crate::NoteKind::Daily => format!(
            "# {title}\n\n## ログ\n\n-\n\n## 振り返り\n\n-\n"
        ),
        crate::NoteKind::Weekly => format!(
            "# {title}\n\n## 振り返り\n\n-\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_template_daily() {
        let result = default_template(&crate::NoteKind::Daily, "2026-03-10");
        assert!(result.starts_with("# 2026-03-10"));
        assert!(result.contains("## ログ"));
        assert!(result.contains("## 振り返り"));
    }

    #[test]
    fn default_template_weekly() {
        let result = default_template(&crate::NoteKind::Weekly, "2026-W11");
        assert!(result.starts_with("# 2026-W11"));
        assert!(result.contains("## 振り返り"));
        // Weekly should not have ログ section
        assert!(!result.contains("## ログ"));
    }

    #[test]
    fn create_daily_note_without_template() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let vault = tmp.path();

        // No template file → uses default_template
        let (path, created) = create_note(vault, &crate::NoteKind::Daily).expect("create");
        assert!(created);
        assert!(path.exists());
        assert!(path.starts_with(vault.join("50_Daily")));
        assert!(path.extension().and_then(|s| s.to_str()) == Some("md"));

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("## ログ"));
    }

    #[test]
    fn create_daily_note_returns_existing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let vault = tmp.path();

        let (path1, created1) = create_note(vault, &crate::NoteKind::Daily).expect("create");
        assert!(created1);

        let (path2, created2) = create_note(vault, &crate::NoteKind::Daily).expect("create");
        assert!(!created2);
        assert_eq!(path1, path2);
    }

    #[test]
    fn create_weekly_note_without_template() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let vault = tmp.path();

        let (path, created) = create_note(vault, &crate::NoteKind::Weekly).expect("create");
        assert!(created);
        assert!(path.starts_with(vault.join("60_Weekly")));

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("## 振り返り"));
    }

    #[test]
    fn create_note_with_template_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let vault = tmp.path();

        // Create a template
        let tmpl_dir = vault.join("Templates");
        fs::create_dir_all(&tmpl_dir).expect("mkdir");
        fs::write(
            tmpl_dir.join("daily-template.md"),
            "# <% tp.file.title %>\n\nCustom template\n",
        )
        .expect("write template");

        let (path, created) = create_note(vault, &crate::NoteKind::Daily).expect("create");
        assert!(created);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("Custom template"));
        // tp.file.title should have been expanded to today's date
        assert!(!content.contains("tp.file.title"));
    }
}
