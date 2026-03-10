// note_creator.rs
// Daily / Weekly Note を生成する。
// 既存ファイルが存在する場合は作成せずそのパスを返す。

use std::path::{Path, PathBuf};

use chrono::{Datelike, IsoWeek, Local};

use crate::template;

/// (出力ファイルパス, 新規作成フラグ)
pub fn create_note(
    vault_root: &Path,
    kind: crate::NoteKind,
) -> anyhow::Result<(PathBuf, bool)> {
    let (output_path, template_path, file_title) = match kind {
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
        .unwrap_or_else(|_| default_template(&kind, &file_title));

    let expanded = template::expand(&template_content, &file_title)?;
    std::fs::write(&output_path, &expanded)?;

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
