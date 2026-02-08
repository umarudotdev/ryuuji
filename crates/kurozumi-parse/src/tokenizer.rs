/// Token types produced by the tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Text enclosed in brackets: `[SubGroup]`, `(720p)`.
    Bracketed,
    /// Free text between brackets/delimiters.
    FreeText,
    /// A delimiter character (space, underscore, dot, dash).
    Delimiter,
}

/// A single token from a filename.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

/// Opening/closing bracket pairs.
const BRACKETS: &[(char, char)] = &[('[', ']'), ('(', ')')];

fn is_delimiter(c: char) -> bool {
    matches!(c, ' ' | '_' | '.')
}

fn opening_bracket(c: char) -> Option<char> {
    BRACKETS
        .iter()
        .find(|(open, _)| *open == c)
        .map(|(_, close)| *close)
}

/// Tokenize an anime filename into structured tokens.
///
/// The tokenizer handles:
/// - Bracket-enclosed groups `[...]` and `(...)`
/// - Delimiter-separated free text
/// - File extension stripping
pub fn tokenize(input: &str) -> Vec<Token> {
    // Strip file extension if present.
    let input = strip_extension(input);
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Check for opening bracket.
        if let Some(close) = opening_bracket(c) {
            i += 1;
            let start = i;
            // Find matching closing bracket.
            while i < chars.len() && chars[i] != close {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            if !text.is_empty() {
                tokens.push(Token {
                    kind: TokenKind::Bracketed,
                    text,
                });
            }
            if i < chars.len() {
                i += 1; // skip closing bracket
            }
            continue;
        }

        // Check for delimiter.
        if is_delimiter(c) {
            // Consume consecutive delimiters but just emit one.
            while i < chars.len() && is_delimiter(chars[i]) {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Delimiter,
                text: String::from(" "),
            });
            continue;
        }

        // Free text: consume until bracket or delimiter.
        let start = i;
        while i < chars.len() && !is_delimiter(chars[i]) && opening_bracket(chars[i]).is_none() {
            i += 1;
        }
        let text: String = chars[start..i].iter().collect();
        if !text.is_empty() {
            tokens.push(Token {
                kind: TokenKind::FreeText,
                text,
            });
        }
    }

    tokens
}

/// Strip common video file extensions.
fn strip_extension(input: &str) -> &str {
    for ext in &[".mkv", ".mp4", ".avi", ".ogm", ".wmv", ".mpg", ".flv"] {
        if let Some(stripped) = input.strip_suffix(ext) {
            return stripped;
        }
        // Case-insensitive check.
        if input.len() > ext.len() {
            let suffix = &input[input.len() - ext.len()..];
            if suffix.eq_ignore_ascii_case(ext) {
                return &input[..input.len() - ext.len()];
            }
        }
    }
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let tokens = tokenize("[SubGroup] Anime Title - 05 [1080p].mkv");
        assert_eq!(tokens[0].kind, TokenKind::Bracketed);
        assert_eq!(tokens[0].text, "SubGroup");
        // After bracket there's a space → delimiter
        assert_eq!(tokens[1].kind, TokenKind::Delimiter);
        // "Anime" is free text
        assert_eq!(tokens[2].kind, TokenKind::FreeText);
        assert_eq!(tokens[2].text, "Anime");
    }

    #[test]
    fn test_underscore_delimiters() {
        let tokens = tokenize("[Group]_Anime_Title_-_05_[720p]");
        let free: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::FreeText)
            .map(|t| t.text.as_str())
            .collect();
        assert!(free.contains(&"Anime"));
        assert!(free.contains(&"Title"));
    }

    #[test]
    fn test_extension_stripping() {
        assert_eq!(strip_extension("test.mkv"), "test");
        assert_eq!(strip_extension("test.MKV"), "test");
        assert_eq!(strip_extension("test.txt"), "test.txt");
    }
}
