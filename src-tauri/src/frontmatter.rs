use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};

/// Markdownファイルの YAML フロントマターを表す
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(flatten)]
    pub fields: IndexMap<String, serde_yaml::Value>,
}

/// フロントマターの解析結果。本文との分離も含む。
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// フロントマター（存在しない場合は None）
    pub frontmatter: Option<Frontmatter>,
    /// フロントマター以降の本文（先頭改行を除く）
    pub body: String,
    /// フロントマターの生 YAML 文字列（解析時点のそのままの内容）
    pub raw_yaml: Option<String>,
}

// ────────────────────── パース ──────────────────────

/// Markdownテキストからフロントマターを抽出しパースする。
/// フロントマターが存在しない場合、または YAML のパースに失敗した場合は None を返す。
pub fn parse(content: &str) -> Option<Frontmatter> {
    let (yaml_str, _) = split_raw(content)?;
    serde_yaml::from_str(yaml_str).ok()
}

/// Markdown テキストをフロントマター部と本文開始位置に分離する。
/// 返り値は `(yaml 部分の &str, 閉じ "---" 直後のバイトオフセット)` 。
///
/// フロントマターとして認識する条件:
///   - BOM を除いた先頭が `---` で始まり、その直後が LF または CRLF（インデント不可）
///   - 行頭の `---` で閉じられている
fn split_raw(content: &str) -> Option<(&str, usize)> {
    // BOM 除去（バイトオフセットを保持するため長さの差で計算）
    let without_bom = content.trim_start_matches('\u{feff}');
    let bom_len = content.len() - without_bom.len();

    // 先頭が `---` でなければフロントマターなし
    // NOTE: trim_start() すると空白インデントも許容してしまうので行わない
    if !without_bom.starts_with("---") {
        return None;
    }

    // `---` 直後に改行が必要
    let after_dashes = &without_bom[3..];
    let newline_len = if after_dashes.starts_with("\r\n") {
        2
    } else if after_dashes.starts_with('\n') {
        1
    } else {
        return None; // `---abc` のようなケースは不正
    };

    let yaml_start = bom_len + 3 + newline_len;
    let yaml_content = &content[yaml_start..];

    // 閉じ `---` を探す
    let closing_offset = find_closing_delimiter(yaml_content)?;
    let yaml_str = &yaml_content[..closing_offset];

    // 閉じ `---` のあとのバイト位置（content 全体に対するオフセット）
    let after_closing = yaml_start + closing_offset + 3; // `---` の 3 バイト

    Some((yaml_str, after_closing))
}

/// 閉じ区切り `---` の行頭位置を探す。
/// `content` は YAML 部分の先頭から始まるとする。
fn find_closing_delimiter(content: &str) -> Option<usize> {
    let mut pos = 0;
    for line in content.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed == "---" {
            return Some(pos);
        }
        pos += line.len();
    }
    // 最終行が改行なしで `---` で終わる場合
    if content[pos..].trim_end_matches(['\n', '\r']) == "---" {
        return Some(pos);
    }
    None
}

/// Markdown テキストをフロントマターと本文に分離する。
pub fn parse_document(content: &str) -> ParsedDocument {
    let Some((yaml_str, end_pos)) = split_raw(content) else {
        return ParsedDocument {
            frontmatter: None,
            body: content.to_string(),
            raw_yaml: None,
        };
    };

    let frontmatter: Option<Frontmatter> = serde_yaml::from_str(yaml_str).ok();

    // 閉じ区切り以降の本文を抽出
    let body = if end_pos < content.len() {
        let rest = &content[end_pos..];
        // 先頭の改行を1つだけ除去
        rest.strip_prefix("\r\n")
            .or_else(|| rest.strip_prefix('\n'))
            .unwrap_or(rest)
            .to_string()
    } else {
        String::new()
    };

    ParsedDocument {
        frontmatter,
        body,
        raw_yaml: Some(yaml_str.to_string()),
    }
}

// ────────────────────── アクセサ ──────────────────────

impl Frontmatter {
    /// フィールドを文字列として取得する
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.fields.get(key).and_then(|v| v.as_str())
    }

    /// フィールドを文字列のリストとして取得する（tags, aliases 等）
    pub fn get_string_list(&self, key: &str) -> Vec<String> {
        let Some(val) = self.fields.get(key) else {
            return Vec::new();
        };
        match val {
            serde_yaml::Value::Sequence(seq) => seq
                .iter()
                .map(|v| match v {
                    serde_yaml::Value::String(s) => s.clone(),
                    other => serde_yaml::to_string(other)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                })
                .collect(),
            serde_yaml::Value::String(s) => {
                // カンマ区切りの単一文字列を分割
                s.split(',').map(|part| part.trim().to_string()).collect()
            }
            _ => Vec::new(),
        }
    }

    /// フィールドを bool として取得する
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.fields.get(key).and_then(serde_yaml::Value::as_bool)
    }

    /// フィールドが存在するか
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// フィールドを設定する
    pub fn set(&mut self, key: impl Into<String>, value: serde_yaml::Value) {
        self.fields.insert(key.into(), value);
    }

    /// フィールドを文字列値で設定する
    pub fn set_str(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.fields
            .insert(key.into(), serde_yaml::Value::String(value.into()));
    }

    /// フィールドを削除する
    pub fn remove(&mut self, key: &str) -> Option<serde_yaml::Value> {
        self.fields.shift_remove(key)
    }
}

// ────────────────────── シリアライズ ──────────────────────

/// フロントマターを YAML 文字列にシリアライズする
pub fn serialize(fm: &Frontmatter) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(&fm.fields)
}

/// フロントマターと本文を結合して Markdown テキストを生成する
pub fn to_markdown(fm: &Frontmatter, body: &str) -> Result<String, serde_yaml::Error> {
    if fm.fields.is_empty() {
        return Ok(format!("---\n---\n{body}"));
    }
    let yaml = serialize(fm)?;
    // serde_yaml は末尾に改行を付けるので trim してから組み立てる
    let yaml = yaml.trim_end();
    Ok(format!("---\n{yaml}\n---\n{body}"))
}

/// 既存の Markdown テキストのフロントマターを更新する。
/// フロントマターが存在しなければ先頭に追加する。
///
/// フロントマターの区切り行は存在するが YAML として壊れている場合は、
/// 元の内容を保護するために Err を返す。
pub fn update_frontmatter(
    content: &str,
    updater: impl FnOnce(&mut Frontmatter),
) -> Result<String, serde_yaml::Error> {
    let doc = parse_document(content);
    let mut fm = match doc.frontmatter {
        Some(fm) => fm,
        None if doc.raw_yaml.is_some() => {
            return Err(serde_yaml::Error::custom(
                "invalid YAML frontmatter detected; update_frontmatter will not overwrite it",
            ));
        }
        None => Frontmatter::default(),
    };
    updater(&mut fm);
    to_markdown(&fm, &doc.body)
}

// ────────────────────── テスト ──────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse ──

    #[test]
    fn parse_valid_frontmatter() {
        let md = "---\ntitle: Test\ntags: [a, b]\n---\n# Body";
        let fm = parse(md).expect("should parse");
        assert_eq!(
            fm.fields.get("title").expect("title"),
            &serde_yaml::Value::String("Test".into())
        );
    }

    #[test]
    fn parse_no_frontmatter() {
        let md = "# Just a heading";
        assert!(parse(md).is_none());
    }

    #[test]
    fn parse_with_bom() {
        let md = "\u{feff}---\ntitle: BOM Test\n---\n# Body";
        let fm = parse(md).expect("should parse BOM content");
        assert_eq!(fm.get_str("title"), Some("BOM Test"));
    }

    #[test]
    fn parse_empty_frontmatter() {
        let md = "---\n---\n# Body";
        let fm = parse(md).expect("should parse empty frontmatter");
        assert!(fm.fields.is_empty());
    }

    #[test]
    fn parse_no_closing_delimiter() {
        let md = "---\ntitle: Broken\n# Body without closing";
        assert!(parse(md).is_none());
    }

    #[test]
    fn parse_not_at_start() {
        let md = "Some text\n---\ntitle: Not FM\n---\n";
        assert!(parse(md).is_none());
    }

    #[test]
    fn parse_dashes_without_newline() {
        // `---abc` のようなケースはフロントマターではない
        let md = "---title: Bad\n---\n";
        assert!(parse(md).is_none());
    }

    // ── parse_document ──

    #[test]
    fn parse_document_with_fm() {
        let md = "---\ntitle: Hello\n---\n# Body\n\nContent here";
        let doc = parse_document(md);
        assert!(doc.frontmatter.is_some());
        assert_eq!(
            doc.frontmatter.as_ref().and_then(|f| f.get_str("title")),
            Some("Hello")
        );
        assert_eq!(doc.body, "# Body\n\nContent here");
    }

    #[test]
    fn parse_document_without_fm() {
        let md = "# No frontmatter\n\nJust text";
        let doc = parse_document(md);
        assert!(doc.frontmatter.is_none());
        assert_eq!(doc.body, md);
    }

    #[test]
    fn parse_document_body_preserves_content() {
        let md = "---\nkey: val\n---\nLine 1\nLine 2\nLine 3";
        let doc = parse_document(md);
        assert_eq!(doc.body, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn parse_document_empty_body() {
        let md = "---\ntitle: No Body\n---\n";
        let doc = parse_document(md);
        assert!(doc.frontmatter.is_some());
        assert!(doc.body.is_empty());
    }

    #[test]
    fn parse_document_raw_yaml_preserved() {
        let md = "---\ntitle: Raw\n---\n# Body";
        let doc = parse_document(md);
        assert!(doc.raw_yaml.is_some());
        assert!(doc.raw_yaml.as_ref().is_some_and(|y| y.contains("title: Raw")));
    }

    // ── アクセサ ──

    #[test]
    fn get_str_accessor() {
        let md = "---\ntitle: My Note\n---\n";
        let fm = parse(md).expect("parse");
        assert_eq!(fm.get_str("title"), Some("My Note"));
        assert_eq!(fm.get_str("missing"), None);
    }

    #[test]
    fn get_string_list_from_sequence() {
        let md = "---\ntags: [rust, tauri, obsidian]\n---\n";
        let fm = parse(md).expect("parse");
        let tags = fm.get_string_list("tags");
        assert_eq!(tags, vec!["rust", "tauri", "obsidian"]);
    }

    #[test]
    fn get_string_list_from_csv_string() {
        let md = "---\ntags: \"rust, tauri\"\n---\n";
        let fm = parse(md).expect("parse");
        let tags = fm.get_string_list("tags");
        assert_eq!(tags, vec!["rust", "tauri"]);
    }

    #[test]
    fn get_string_list_missing_key() {
        let md = "---\ntitle: X\n---\n";
        let fm = parse(md).expect("parse");
        assert!(fm.get_string_list("tags").is_empty());
    }

    #[test]
    fn get_bool_accessor() {
        let md = "---\npublished: true\ndraft: false\n---\n";
        let fm = parse(md).expect("parse");
        assert_eq!(fm.get_bool("published"), Some(true));
        assert_eq!(fm.get_bool("draft"), Some(false));
        assert_eq!(fm.get_bool("missing"), None);
    }

    #[test]
    fn has_field() {
        let md = "---\ntitle: X\n---\n";
        let fm = parse(md).expect("parse");
        assert!(fm.has("title"));
        assert!(!fm.has("missing"));
    }

    // ── 変更操作 ──

    #[test]
    fn set_and_remove_fields() {
        let mut fm = Frontmatter::default();
        fm.set_str("title", "New Note");
        assert_eq!(fm.get_str("title"), Some("New Note"));

        fm.remove("title");
        assert!(!fm.has("title"));
    }

    #[test]
    fn set_with_yaml_value() {
        let mut fm = Frontmatter::default();
        fm.set(
            "tags",
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("a".into()),
                serde_yaml::Value::String("b".into()),
            ]),
        );
        assert_eq!(fm.get_string_list("tags"), vec!["a", "b"]);
    }

    // ── シリアライズ ──

    #[test]
    fn serialize_roundtrip() {
        let md = "---\ntitle: Round Trip\ntags:\n- a\n- b\n---\n# Body";
        let fm = parse(md).expect("parse");
        let yaml = serialize(&fm).expect("serialize");
        // 再パースして同一であることを確認
        let fm2: Frontmatter = serde_yaml::from_str(&yaml).expect("re-parse");
        assert_eq!(fm2.get_str("title"), Some("Round Trip"));
        assert_eq!(fm2.get_string_list("tags"), vec!["a", "b"]);
    }

    #[test]
    fn to_markdown_format() {
        let mut fm = Frontmatter::default();
        fm.set_str("title", "Test");
        let result = to_markdown(&fm, "# Body\n").expect("to_markdown");
        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test"));
        assert!(result.contains("---\n# Body\n"));
    }

    // ── update_frontmatter ──

    #[test]
    fn update_existing_frontmatter() {
        let md = "---\ntitle: Old\n---\n# Body";
        let result = update_frontmatter(md, |fm| {
            fm.set_str("title", "New");
        })
        .expect("update");
        assert!(result.contains("title: New"));
        assert!(result.contains("# Body"));
        assert!(!result.contains("title: Old"));
    }

    #[test]
    fn update_adds_frontmatter_when_missing() {
        let md = "# No FM";
        let result = update_frontmatter(md, |fm| {
            fm.set_str("title", "Added");
        })
        .expect("update");
        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Added"));
        assert!(result.contains("# No FM"));
    }

    #[test]
    fn update_add_field_to_existing() {
        let md = "---\ntitle: Keep\n---\n# Body";
        let result = update_frontmatter(md, |fm| {
            fm.set("published", serde_yaml::Value::Bool(true));
        })
        .expect("update");
        assert!(result.contains("title: Keep"));
        assert!(result.contains("published: true"));
        assert!(result.contains("# Body"));
    }

    #[test]
    fn update_rejects_invalid_yaml() {
        // 区切り行はあるが YAML として壊れている場合はエラーを返す
        let md = "---\n: invalid: yaml: [broken\n---\n# Body";
        let result = update_frontmatter(md, |fm| {
            fm.set_str("title", "Should fail");
        });
        assert!(result.is_err());
    }

    // ── エッジケース ──

    #[test]
    fn parse_multiline_values() {
        let md = "---\ndescription: |\n  This is a\n  multiline value\ntitle: ML\n---\n# Body";
        let fm = parse(md).expect("parse");
        assert_eq!(fm.get_str("title"), Some("ML"));
        let desc = fm.get_str("description").expect("description");
        assert!(desc.contains("multiline"));
    }

    #[test]
    fn parse_nested_yaml() {
        let md = "---\nmeta:\n  author: Alice\n  version: 1\n---\n";
        let fm = parse(md).expect("parse");
        assert!(fm.has("meta"));
    }

    #[test]
    fn parse_frontmatter_with_leading_whitespace() {
        // 先頭に空白がある場合 → Obsidian ではフロントマターと見なさない
        let md = "  ---\ntitle: Indented\n---\n# Body";
        assert!(parse(md).is_none());
    }

    #[test]
    fn parse_closing_without_trailing_newline() {
        // 末尾改行なしでも閉じ区切りを認識する
        let md = "---\ntitle: EOF\n---";
        let fm = parse(md).expect("parse");
        assert_eq!(fm.get_str("title"), Some("EOF"));
    }

    #[test]
    fn parse_document_closing_without_trailing_newline() {
        let md = "---\ntitle: EOF\n---";
        let doc = parse_document(md);
        assert!(doc.frontmatter.is_some());
        assert!(doc.body.is_empty());
    }
}
