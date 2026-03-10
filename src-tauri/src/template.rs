/// Templater構文のサブセットを展開する。
///
/// 対応する構文:
///   <% tp.date.now("FORMAT") %>
///   <% tp.date.weekday("FORMAT", WEEKDAY) %>
///   <% tp.date.weekday("FORMAT", WEEKDAY, `WEEK_OFFSET`) %>
///   <% tp.file.title %>
///
/// Dataview/Tasksクエリブロック内の構文はそのまま素通し。
use chrono::{Datelike, Duration, Local, NaiveDate};
use regex::Regex;
use std::sync::OnceLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("不明なTemplater構文: {0}")]
    UnknownSyntax(String),
    #[error("日付フォーマットエラー: {0}")]
    FormatError(String),
}

static TEMPLATE_RE: OnceLock<Regex> = OnceLock::new();
static CODE_BLOCK_RE: OnceLock<Regex> = OnceLock::new();

fn template_re() -> &'static Regex {
    TEMPLATE_RE.get_or_init(|| Regex::new(r"<%\s*(.+?)\s*%>").unwrap())
}

fn code_block_re() -> &'static Regex {
    CODE_BLOCK_RE.get_or_init(|| Regex::new(r"(?s)```[a-z]*\n.*?```").unwrap())
}

/// FORMAT文字列をchronoフォーマットに変換する。
/// Templaterは moment.js 形式なので最低限の変換を行う。
///
/// YYYY → %Y, MM → %m, DD → %d, ww → ISO週番号(%V)
/// [W] のようなリテラルブラケットは除去して文字として扱う
fn moment_to_chrono(fmt: &str) -> String {
    // [TEXT] → TEXT のリテラル展開を先に処理
    let re_literal = Regex::new(r"\[([^\]]*)\]").unwrap();
    let fmt = re_literal.replace_all(fmt, "$1").to_string();

    fmt.replace("YYYY", "%Y")
        .replace("MM", "%m")
        .replace("DD", "%d")
        .replace("ww", "%V")    // ISO週番号
        .replace("ddd", "%a")
        .replace("DDD", "%j")
}

/// 週番号・曜日オフセットから NaiveDate を計算する。
///
/// weekday: 0=日, 1=月 ... 6=土 (moment.js の weekday)
/// week_offset: 何週後か（デフォルト0）
fn calc_weekday(base: NaiveDate, weekday: i64, week_offset: i64) -> NaiveDate {
    // chrono: Monday=0 ... Sunday=6
    // moment weekday(locale=ja): 0=日, 1=月 ... 6=土
    // → chrono に合わせて変換
    let chrono_target = match weekday {
        0 => 6, // 日
        n => n - 1,
    };

    let base_weekday = i64::from(base.weekday().num_days_from_monday());
    let diff = chrono_target - base_weekday;
    base + Duration::days(diff + week_offset * 7)
}

/// 単一の Templater 式を評価して文字列に変換する。
fn eval_expr(expr: &str, file_title: &str, today: NaiveDate) -> Result<String, TemplateError> {
    let expr = expr.trim();

    // tp.file.title
    if expr == "tp.file.title" {
        return Ok(file_title.to_string());
    }

    // tp.date.now("FORMAT")
    if let Some(rest) = expr.strip_prefix("tp.date.now(") {
        let fmt_raw = rest
            .trim_end_matches(')')
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        let chrono_fmt = moment_to_chrono(fmt_raw);
        return Ok(today.format(&chrono_fmt).to_string());
    }

    // tp.date.weekday("FORMAT", WEEKDAY) または
    // tp.date.weekday("FORMAT", WEEKDAY, WEEK_OFFSET)
    if let Some(rest) = expr.strip_prefix("tp.date.weekday(") {
        let args_str = rest.trim_end_matches(')');

        // 引数をパース: "FORMAT", weekday [, week_offset]
        let re_args =
            Regex::new(r#"["']([^"']+)["'],\s*(-?\d+)(?:,\s*(-?\d+))?"#).unwrap();
        let Some(caps) = re_args.captures(args_str) else {
            return Err(TemplateError::UnknownSyntax(expr.to_string()));
        };

        let fmt_raw = &caps[1];
        let weekday: i64 = caps[2].parse().unwrap_or(1);
        let week_offset: i64 = caps.get(3).map_or(0, |m| m.as_str().parse().unwrap_or(0));

        let target_date = calc_weekday(today, weekday, week_offset);
        let chrono_fmt = moment_to_chrono(fmt_raw);
        return Ok(target_date.format(&chrono_fmt).to_string());
    }

    Err(TemplateError::UnknownSyntax(expr.to_string()))
}

/// テンプレート文字列全体を展開する。
///
/// コードブロック（```...```）内の構文はそのまま保持する。
pub fn expand(template: &str, file_title: &str) -> Result<String, TemplateError> {
    let today = Local::now().date_naive();

    // コードブロックの位置を記録して保護する
    let code_re = code_block_re();
    let tmpl_re = template_re();

    // コードブロックをプレースホルダに置換
    let mut placeholders: Vec<String> = Vec::new();
    let protected = code_re.replace_all(template, |caps: &regex::Captures| {
        let idx = placeholders.len();
        placeholders.push(caps[0].to_string());
        format!("\x00CODE_BLOCK_{idx}\x00")
    });

    // Templater式を展開
    let mut result = String::new();
    let mut last_end = 0;
    for caps in tmpl_re.captures_iter(&protected) {
        let Some(m) = caps.get(0) else {
            continue;
        };
        result.push_str(&protected[last_end..m.start()]);
        let expr = &caps[1];
        match eval_expr(expr, file_title, today) {
            Ok(expanded) => result.push_str(&expanded),
            Err(e) => {
                // 未知の構文はそのまま保持してエラーを無視
                eprintln!("template warning: {e}");
                result.push_str(m.as_str());
            }
        }
        last_end = m.end();
    }
    result.push_str(&protected[last_end..]);

    // プレースホルダを元に戻す
    for (idx, block) in placeholders.iter().enumerate() {
        result = result.replace(&format!("\x00CODE_BLOCK_{idx}\x00"), block);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_moment_to_chrono() {
        assert_eq!(moment_to_chrono("YYYY-MM-DD"), "%Y-%m-%d");
        assert_eq!(moment_to_chrono("YYYY-[W]ww"), "%Y-W%V");
    }

    #[test]
    fn test_calc_weekday_monday() {
        // 2026-03-07 は土曜日
        let base = date(2026, 3, 7);
        // weekday=1 (月曜) → 2026-03-02
        assert_eq!(calc_weekday(base, 1, 0), date(2026, 3, 2));
    }

    #[test]
    fn test_calc_weekday_next_sunday() {
        let base = date(2026, 3, 7);
        // weekday=0 (日曜), week_offset=1 → 翌週日曜 = 2026-03-15
        assert_eq!(calc_weekday(base, 0, 1), date(2026, 3, 15));
    }

    #[test]
    fn test_code_block_passthrough() {
        let tmpl = "# title\n```dataviewjs\n<% tp.date.now(\"YYYY\") %>\n```";
        let result = expand(tmpl, "test").unwrap();
        // コードブロック内は展開されない
        assert!(result.contains("<% tp.date.now(\"YYYY\") %>"));
    }

    // ---- eval_expr ----

    #[test]
    fn eval_file_title() {
        let today = date(2026, 3, 10);
        let result = eval_expr("tp.file.title", "2026-03-10", today).unwrap();
        assert_eq!(result, "2026-03-10");
    }

    #[test]
    fn eval_date_now() {
        let today = date(2026, 3, 10);
        let result = eval_expr("tp.date.now(\"YYYY-MM-DD\")", "title", today).unwrap();
        assert_eq!(result, "2026-03-10");
    }

    #[test]
    fn eval_date_now_with_literal_bracket() {
        let today = date(2026, 3, 10);
        let result = eval_expr("tp.date.now(\"YYYY-[W]ww\")", "title", today).unwrap();
        assert_eq!(result, "2026-W11");
    }

    #[test]
    fn eval_date_weekday_basic() {
        let today = date(2026, 3, 10); // Tuesday
        // weekday=1 (Monday) of current week
        let result = eval_expr("tp.date.weekday(\"YYYY-MM-DD\", 1)", "t", today).unwrap();
        assert_eq!(result, "2026-03-09");
    }

    #[test]
    fn eval_date_weekday_with_offset() {
        let today = date(2026, 3, 10); // Tuesday
        // weekday=1 (Monday), week_offset=1 → next Monday
        let result = eval_expr("tp.date.weekday(\"YYYY-MM-DD\", 1, 1)", "t", today).unwrap();
        assert_eq!(result, "2026-03-16");
    }

    #[test]
    fn eval_unknown_syntax_returns_error() {
        let today = date(2026, 3, 10);
        let result = eval_expr("tp.unknown()", "t", today);
        assert!(result.is_err());
    }

    // ---- expand ----

    #[test]
    fn expand_mixed_template() {
        // expand() uses Local::now(), so we just verify structural behavior
        let tmpl = "# <% tp.file.title %>\n\nBody text";
        let result = expand(tmpl, "MyNote").unwrap();
        assert!(result.starts_with("# MyNote\n"));
        assert!(result.contains("Body text"));
    }

    #[test]
    fn expand_no_template_expressions() {
        let tmpl = "# Plain markdown\n\n- item 1\n- item 2\n";
        let result = expand(tmpl, "title").unwrap();
        assert_eq!(result, tmpl);
    }

    #[test]
    fn expand_unknown_syntax_preserved() {
        let tmpl = "# <% tp.unknown.call() %>";
        let result = expand(tmpl, "t").unwrap();
        // Unknown syntax should be preserved as-is
        assert!(result.contains("<% tp.unknown.call() %>"));
    }

    #[test]
    fn expand_multiple_code_blocks() {
        let tmpl = "```tasks\n<% tp.file.title %>\n```\n\n<% tp.file.title %>\n\n```dataview\n<% tp.date.now(\"YYYY\") %>\n```";
        let result = expand(tmpl, "Note").unwrap();
        // Code blocks preserved
        assert!(result.contains("```tasks\n<% tp.file.title %>\n```"));
        assert!(result.contains("```dataview\n<% tp.date.now(\"YYYY\") %>\n```"));
        // Outside code blocks, expanded
        assert!(result.contains("\n\nNote\n\n"));
    }

    // ---- moment_to_chrono additional ----

    #[test]
    fn test_moment_ddd_weekday_abbr() {
        assert_eq!(moment_to_chrono("ddd"), "%a");
    }
}
