use crate::models::ContentType;
use regex::Regex;
use std::sync::LazyLock;

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:https?|ftp)://[^\s/$.?#].[^\s]*$").unwrap()
});

static HEX_COLOR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$").unwrap()
});

static RGB_COLOR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^rgba?\(\s*\d+\s*,\s*\d+\s*,\s*\d+(?:\s*,\s*(?:[01]?(?:\.\d+)?|\d+%))?\s*\)$").unwrap()
});

static HSL_COLOR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^hsla?\(\s*\d+\s*,\s*[\d.]+%?\s*,\s*[\d.]+%?(?:\s*,\s*(?:[01]?(?:\.\d+)?|\d+%))?\s*\)$").unwrap()
});

pub fn classify_content(raw: &str) -> ContentType {
    let text = raw.trim();
    if text.is_empty() {
        return ContentType::Text;
    }

    // 1. Check Color
    if HEX_COLOR_REGEX.is_match(text)
        || RGB_COLOR_REGEX.is_match(text)
        || HSL_COLOR_REGEX.is_match(text)
    {
        return ContentType::Color;
    }

    // 2. Check URL
    if URL_REGEX.is_match(text) {
        return ContentType::Url;
    }

    // 3. Check JSON
    if (text.starts_with('{') && text.ends_with('}'))
        || (text.starts_with('[') && text.ends_with(']'))
    {
        if serde_json::from_str::<serde_json::Value>(text).is_ok() {
            return ContentType::Json;
        }
    }

    // 4. Check Code heuristics
    if is_likely_code(text) {
        return ContentType::Code;
    }

    ContentType::Text
}

fn is_likely_code(text: &str) -> bool {
    let code_indicators = [
        "function ", "const ", "let ", "var ", "def ", "class ", "import ", "export ",
        "return ", "async ", "await ", "fn ", "pub fn ", "struct ", "impl ", "enum ",
        "interface ", "namespace ", "using namespace", "#include ", "public static void",
        "SELECT ", "FROM ", "WHERE ", "INSERT INTO ", "CREATE TABLE ",
        "<html>", "<div", "<span", "/>", "{\n", "}\n", "=>", "==", "!=", "&&", "||",
        "/*", "*/", "///", "#!/bin/",
    ];

    let mut score = 0;

    for ind in code_indicators {
        if text.contains(ind) {
            score += 2;
        }
    }

    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > 1 {
        let indented_lines = lines.iter().filter(|l| l.starts_with("  ") || l.starts_with('\t')).count();
        if indented_lines >= lines.len() / 3 && indented_lines > 0 {
            score += 2;
        }

        let semicolon_lines = lines.iter().filter(|l| l.trim_end().ends_with(';')).count();
        if semicolon_lines >= 2 {
            score += 2;
        }
    }

    score >= 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url() {
        assert_eq!(classify_content("https://tauri.app"), ContentType::Url);
        assert_eq!(classify_content("http://localhost:3000/app"), ContentType::Url);
    }

    #[test]
    fn test_color() {
        assert_eq!(classify_content("#fff"), ContentType::Color);
        assert_eq!(classify_content("#4CC2FF"), ContentType::Color);
        assert_eq!(classify_content("rgb(255, 0, 128)"), ContentType::Color);
        assert_eq!(classify_content("rgba(0, 0, 0, 0.5)"), ContentType::Color);
    }

    #[test]
    fn test_json() {
        assert_eq!(classify_content(r#"{"name": "ClipCtx", "v": 2}"#), ContentType::Json);
        assert_eq!(classify_content(r#"[1, 2, 3, "test"]"#), ContentType::Json);
    }

    #[test]
    fn test_code() {
        assert_eq!(classify_content("fn main() {\n    println!(\"hello\");\n}"), ContentType::Code);
        assert_eq!(classify_content("const add = (a: number, b: number) => a + b;"), ContentType::Code);
    }
}
