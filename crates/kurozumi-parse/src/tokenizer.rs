/// Token types produced by the tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Text enclosed in brackets: `[SubGroup]`, `(720p)`.
    Bracketed,
    /// Free text between brackets/delimiters.
    FreeText,
    /// A delimiter character (space, underscore, dot).
    Delimiter,
}

/// A single token from a filename.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    /// Whether this token was inside a bracket pair.
    /// Always true for `Bracketed` tokens.
    pub is_enclosed: bool,
}

/// Opening/closing bracket pairs, including CJK brackets.
const BRACKETS: &[(char, char)] = &[
    ('[', ']'),
    ('(', ')'),
    ('{', '}'),
    ('\u{300C}', '\u{300D}'), // 「」
    ('\u{300E}', '\u{300F}'), // 『』
    ('\u{3010}', '\u{3011}'), // 【】
];

/// Characters that separate tokens (excluding dash, which gets special treatment).
fn is_soft_delimiter(c: char) -> bool {
    matches!(c, ' ' | '_' | '.' | '\u{3000}')
    // space, underscore, dot, ideographic space
}

/// Dash-family characters that act as token separators but are preserved as FreeText.
fn is_dash(c: char) -> bool {
    matches!(c, '-' | '\u{2013}' | '\u{2014}')
    // hyphen-minus, en-dash, em-dash
}

fn opening_bracket(c: char) -> Option<char> {
    BRACKETS
        .iter()
        .find(|(open, _)| *open == c)
        .map(|(_, close)| *close)
}

/// Tokenize an anime filename into structured tokens.
///
/// Returns (tokens, file_extension). The extension is extracted but not discarded.
///
/// Handles:
/// - Bracket-enclosed groups `[...]`, `(...)`, `{...}`, and CJK brackets
/// - Delimiter-separated free text (space, underscore, dot)
/// - Dashes (`-`, `–`, `—`) emitted as `FreeText("-")` tokens
/// - File extension extraction
pub fn tokenize(input: &str) -> (Vec<Token>, Option<String>) {
    let (input, extension) = strip_extension(input);
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Bracket-enclosed content.
        if let Some(close) = opening_bracket(c) {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != close {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            if !text.is_empty() {
                tokens.push(Token {
                    kind: TokenKind::Bracketed,
                    text,
                    is_enclosed: true,
                });
            }
            if i < chars.len() {
                i += 1; // skip closing bracket
            }
            continue;
        }

        // Dashes become FreeText("-") so the parser can detect "- 05" patterns.
        if is_dash(c) {
            tokens.push(Token {
                kind: TokenKind::FreeText,
                text: "-".into(),
                is_enclosed: false,
            });
            i += 1;
            // Skip trailing soft delimiters after dash.
            while i < chars.len() && is_soft_delimiter(chars[i]) {
                i += 1;
            }
            continue;
        }

        // Soft delimiters (space, underscore, dot, ideographic space).
        if is_soft_delimiter(c) {
            while i < chars.len() && is_soft_delimiter(chars[i]) {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Delimiter,
                text: String::from(" "),
                is_enclosed: false,
            });
            continue;
        }

        // Free text: everything else until a delimiter or bracket.
        // A dot between digits (e.g., "07.5", "H.264") is kept as part of the token.
        let start = i;
        while i < chars.len() && !is_dash(chars[i]) && opening_bracket(chars[i]).is_none() {
            if is_soft_delimiter(chars[i]) {
                // Keep dots between digits as part of the token.
                if chars[i] == '.'
                    && i > start
                    && i + 1 < chars.len()
                    && chars[i - 1].is_ascii_digit()
                    && chars[i + 1].is_ascii_digit()
                {
                    i += 1;
                    continue;
                }
                break;
            }
            i += 1;
        }
        let text: String = chars[start..i].iter().collect();
        if !text.is_empty() {
            tokens.push(Token {
                kind: TokenKind::FreeText,
                text,
                is_enclosed: false,
            });
        }
    }

    (tokens, extension.map(|s| s.to_string()))
}

/// Legacy tokenize function that discards the file extension and returns only tokens.
pub fn tokenize_compat(input: &str) -> Vec<Token> {
    tokenize(input).0
}

/// Strip common video file extensions, returning the base name and extracted extension.
fn strip_extension(input: &str) -> (&str, Option<&str>) {
    for ext in &[
        ".mkv", ".mp4", ".avi", ".ogm", ".wmv", ".mpg", ".flv", ".webm", ".m4v", ".ts", ".mov",
        ".3gp", ".rm", ".rmvb", ".m2ts",
    ] {
        if let Some(stripped) = input.strip_suffix(ext) {
            return (stripped, Some(&ext[1..]));
        }
        // Case-insensitive check — guard against multibyte chars near the boundary.
        let split_pos = input.len().wrapping_sub(ext.len());
        if split_pos < input.len() && input.is_char_boundary(split_pos) {
            let suffix = &input[split_pos..];
            if suffix.eq_ignore_ascii_case(ext) {
                return (&input[..split_pos], Some(&ext[1..]));
            }
        }
    }
    (input, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let (tokens, ext) = tokenize("[SubGroup] Anime Title - 05 [1080p].mkv");
        assert_eq!(ext.as_deref(), Some("mkv"));
        assert_eq!(tokens[0].kind, TokenKind::Bracketed);
        assert_eq!(tokens[0].text, "SubGroup");
        assert!(tokens[0].is_enclosed);
        // After bracket there's a space → delimiter
        assert_eq!(tokens[1].kind, TokenKind::Delimiter);
        assert!(!tokens[1].is_enclosed);
        // "Anime" is free text
        assert_eq!(tokens[2].kind, TokenKind::FreeText);
        assert_eq!(tokens[2].text, "Anime");
    }

    #[test]
    fn test_underscore_delimiters() {
        let (tokens, _) = tokenize("[Group]_Anime_Title_-_05_[720p]");
        let free: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::FreeText && t.text != "-")
            .map(|t| t.text.as_str())
            .collect();
        assert!(free.contains(&"Anime"));
        assert!(free.contains(&"Title"));
    }

    #[test]
    fn test_extension_stripping() {
        let (stripped, ext) = strip_extension("test.mkv");
        assert_eq!(stripped, "test");
        assert_eq!(ext, Some("mkv"));
        let (stripped, ext) = strip_extension("test.MKV");
        assert_eq!(stripped, "test");
        assert_eq!(ext, Some("mkv"));
        let (stripped, ext) = strip_extension("test.txt");
        assert_eq!(stripped, "test.txt");
        assert_eq!(ext, None);
    }

    #[test]
    fn test_cjk_brackets() {
        let (tokens, _) = tokenize("【GroupName】 Title - 01");
        assert_eq!(tokens[0].kind, TokenKind::Bracketed);
        assert_eq!(tokens[0].text, "GroupName");
        assert!(tokens[0].is_enclosed);
    }

    #[test]
    fn test_new_extensions() {
        let (_, ext) = tokenize("test.webm");
        assert_eq!(ext.as_deref(), Some("webm"));
        let (_, ext) = tokenize("test.m4v");
        assert_eq!(ext.as_deref(), Some("m4v"));
        let (_, ext) = tokenize("test.ts");
        assert_eq!(ext.as_deref(), Some("ts"));
    }

    #[test]
    fn test_dash_as_free_text() {
        let (tokens, _) = tokenize("Title - 05");
        let texts: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::FreeText)
            .map(|t| t.text.as_str())
            .collect();
        assert!(texts.contains(&"Title"));
        assert!(texts.contains(&"-"));
        assert!(texts.contains(&"05"));
    }

    #[test]
    fn test_en_dash() {
        let (tokens, _) = tokenize("Title \u{2013} 05");
        let has_dash = tokens.iter().any(|t| t.text == "-");
        assert!(has_dash, "En-dash should be normalized to '-'");
    }
}
