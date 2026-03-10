use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Markdownファイルの YAML フロントマターを表す
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(flatten)]
    pub fields: HashMap<String, serde_yaml::Value>,
}

/// Markdownテキストからフロントマターを抽出しパースする。
/// フロントマターが存在しない場合は None を返す。
pub fn parse(content: &str) -> Option<Frontmatter> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }

    let after_opening = &content[3..];
    let end = after_opening.find("\n---")?;
    let yaml_str = &after_opening[..end];

    serde_yaml::from_str(yaml_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_frontmatter() {
        let md = "---\ntitle: Test\ntags: [a, b]\n---\n# Body";
        let fm = parse(md).unwrap();
        assert_eq!(
            fm.fields.get("title").unwrap(),
            &serde_yaml::Value::String("Test".into())
        );
    }

    #[test]
    fn parse_no_frontmatter() {
        let md = "# Just a heading";
        assert!(parse(md).is_none());
    }
}
