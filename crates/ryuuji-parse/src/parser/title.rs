use crate::tokenizer::{Token, TokenKind};

/// Extract the anime title from remaining unidentified free text tokens.
/// The title is the longest consecutive run of unidentified free text
/// that appears before the episode number.
pub fn extract_title(tokens: &[Token], identified: &[bool]) -> Option<String> {
    let mut title_parts: Vec<&str> = Vec::new();
    let mut started = false;

    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            if started {
                break;
            }
            continue;
        }

        match token.kind {
            TokenKind::FreeText => {
                if token.text == "-" {
                    if started {
                        break;
                    }
                    continue;
                }
                started = true;
                title_parts.push(&token.text);
            }
            TokenKind::Delimiter if started => {
                title_parts.push(" ");
            }
            _ => {
                if started {
                    break;
                }
            }
        }
    }

    if title_parts.is_empty() {
        return None;
    }

    let title = title_parts.join("").trim().to_string();
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

/// Extract the episode title from unidentified free text that appears
/// *after* the episode number position.
pub fn extract_episode_title(
    tokens: &[Token],
    identified: &[bool],
    episode_index: Option<usize>,
) -> Option<String> {
    let start_after = episode_index? + 1;
    let mut parts: Vec<&str> = Vec::new();
    let mut started = false;

    for i in start_after..tokens.len() {
        if identified[i] {
            if started {
                break;
            }
            continue;
        }

        match tokens[i].kind {
            TokenKind::FreeText => {
                if tokens[i].text == "-" {
                    if started {
                        break;
                    }
                    // Skip leading dash after episode.
                    continue;
                }
                started = true;
                parts.push(&tokens[i].text);
            }
            TokenKind::Delimiter if started => {
                parts.push(" ");
            }
            _ => {
                if started {
                    break;
                }
            }
        }
    }

    if parts.is_empty() {
        return None;
    }

    let title = parts.join("").trim().to_string();
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}
