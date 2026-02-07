use crate::elements::Elements;
use crate::keyword::{self, KeywordKind};
use crate::tokenizer::{self, Token, TokenKind};

/// Parse an anime filename into its component elements.
///
/// # Example
/// ```
/// let result = kurozumi_parse::parser::parse("[SubsPlease] Sousou no Frieren - 05 (1080p) [ABCD1234].mkv");
/// assert_eq!(result.title.as_deref(), Some("Sousou no Frieren"));
/// assert_eq!(result.episode_number, Some(5));
/// assert_eq!(result.release_group.as_deref(), Some("SubsPlease"));
/// assert_eq!(result.resolution.as_deref(), Some("1080p"));
/// assert_eq!(result.checksum.as_deref(), Some("ABCD1234"));
/// ```
pub fn parse(filename: &str) -> Elements {
    let tokens = tokenizer::tokenize(filename);
    let mut elements = Elements::default();
    let mut identified = vec![false; tokens.len()];

    // Pass 1: Identify keywords in bracketed tokens.
    identify_bracketed_keywords(&tokens, &mut elements, &mut identified);

    // Pass 2: Extract release group (first bracketed token before any free text).
    extract_release_group(&tokens, &mut elements, &mut identified);

    // Pass 3: Extract checksum (8-char hex in brackets).
    extract_checksum(&tokens, &mut elements, &mut identified);

    // Pass 4: Identify keywords in free text.
    identify_free_keywords(&tokens, &mut elements, &mut identified);

    // Pass 5: Extract episode number from remaining tokens.
    extract_episode(&tokens, &mut elements, &mut identified);

    // Pass 6: Extract title from remaining unidentified free text.
    extract_title(&tokens, &mut elements, &identified);

    elements
}

/// Pass 1: Check bracketed tokens against keyword table.
fn identify_bracketed_keywords(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    for (i, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Bracketed || identified[i] {
            continue;
        }
        if let Some(kind) = keyword::lookup(&token.text) {
            apply_keyword(kind, &token.text, elements);
            identified[i] = true;
        }
        // Also check for resolution pattern inside brackets (e.g., "1920x1080").
        if elements.resolution.is_none() {
            if let Some(res) = parse_resolution(&token.text) {
                elements.resolution = Some(res);
                identified[i] = true;
            }
        }
    }
}

/// Pass 2: First bracketed token (before free text begins) is likely the release group.
fn extract_release_group(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    if elements.release_group.is_some() {
        return;
    }
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if token.kind == TokenKind::Bracketed {
            // Don't claim something that's clearly a keyword or resolution.
            if keyword::lookup(&token.text).is_none() && !is_checksum(&token.text) {
                elements.release_group = Some(token.text.clone());
                identified[i] = true;
                return;
            }
        }
        if token.kind == TokenKind::FreeText {
            // Once we hit free text, the release group window has passed.
            return;
        }
    }
}

/// Pass 3: 8-character hex string in brackets is a CRC32 checksum.
fn extract_checksum(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    for (i, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Bracketed && !identified[i] && is_checksum(&token.text) {
            elements.checksum = Some(token.text.clone());
            identified[i] = true;
            return;
        }
    }
}

fn is_checksum(s: &str) -> bool {
    s.len() == 8 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Pass 4: Check free text tokens against keyword table.
fn identify_free_keywords(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    for (i, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::FreeText || identified[i] {
            continue;
        }
        if let Some(kind) = keyword::lookup(&token.text) {
            apply_keyword(kind, &token.text, elements);
            identified[i] = true;
        }
        if elements.resolution.is_none() {
            if let Some(res) = parse_resolution(&token.text) {
                elements.resolution = Some(res);
                identified[i] = true;
            }
        }
    }
}

/// Pass 5: Find episode number patterns in unidentified tokens.
fn extract_episode(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    // Strategy 1: Look for " - XX" pattern (dash followed by number).
    for i in 0..tokens.len() {
        if identified[i] || tokens[i].kind != TokenKind::FreeText {
            continue;
        }
        let text = &tokens[i].text;

        // Check if previous non-delimiter token is a dash.
        if text == "-" {
            identified[i] = true;
            // Look for number after this dash.
            if let Some(next) = next_free_text(tokens, identified, i) {
                if let Some((ep_str, ep_num)) = parse_episode_number(&tokens[next].text) {
                    elements.episode = Some(ep_str);
                    elements.episode_number = Some(ep_num);
                    identified[next] = true;
                    return;
                }
            }
            continue;
        }
    }

    // Strategy 2: Look for standalone number tokens (e.g., "05", "12").
    // Prefer numbers that appear after a sequence of text tokens.
    let mut saw_text = false;
    for i in 0..tokens.len() {
        if identified[i] {
            continue;
        }
        if tokens[i].kind == TokenKind::FreeText {
            if let Some((ep_str, ep_num)) = parse_episode_number(&tokens[i].text) {
                if saw_text {
                    elements.episode = Some(ep_str);
                    elements.episode_number = Some(ep_num);
                    identified[i] = true;
                    return;
                }
            } else {
                saw_text = true;
            }
        }
    }

    // Strategy 3: Check bracketed tokens for episode patterns like "01", "12v2".
    for i in 0..tokens.len() {
        if identified[i] || tokens[i].kind != TokenKind::Bracketed {
            continue;
        }
        if let Some((ep_str, ep_num)) = parse_episode_number(&tokens[i].text) {
            elements.episode = Some(ep_str);
            elements.episode_number = Some(ep_num);
            identified[i] = true;
            return;
        }
    }
}

/// Pass 6: Remaining unidentified free text tokens form the title.
fn extract_title(tokens: &[Token], elements: &mut Elements, identified: &[bool]) {
    let mut title_parts = Vec::new();
    let mut started = false;

    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            if started {
                // Stop collecting title when we hit an identified token.
                break;
            }
            continue;
        }

        match token.kind {
            TokenKind::FreeText => {
                // Skip standalone dashes.
                if token.text == "-" {
                    if started {
                        break;
                    }
                    continue;
                }
                started = true;
                title_parts.push(token.text.as_str());
            }
            TokenKind::Delimiter if started => {
                // Include delimiter as space in title.
                title_parts.push(" ");
            }
            _ => {
                if started {
                    break;
                }
            }
        }
    }

    if !title_parts.is_empty() {
        let title = title_parts.join("").trim().to_string();
        if !title.is_empty() {
            elements.title = Some(title);
        }
    }
}

/// Find the next unidentified free text token after index `start`.
fn next_free_text(tokens: &[Token], identified: &[bool], start: usize) -> Option<usize> {
    for i in (start + 1)..tokens.len() {
        if identified[i] {
            continue;
        }
        if tokens[i].kind == TokenKind::FreeText {
            return Some(i);
        }
        if tokens[i].kind == TokenKind::Bracketed {
            return None;
        }
    }
    None
}

/// Try to parse a string as an episode number.
/// Handles: "05", "12", "12v2", "12.5", "S2", "01-03" (range).
fn parse_episode_number(s: &str) -> Option<(String, u32)> {
    let s = s.trim();

    // Skip if it looks like a year (4 digits, starts with 19 or 20).
    if s.len() == 4 && (s.starts_with("19") || s.starts_with("20")) {
        if s.parse::<u32>().is_ok() {
            return None;
        }
    }

    // "12v2" pattern — strip version suffix.
    let base = if let Some(pos) = s.to_lowercase().find('v') {
        &s[..pos]
    } else {
        s
    };

    // "12.5" — take integer part.
    let int_part = if let Some(pos) = base.find('.') {
        &base[..pos]
    } else {
        base
    };

    // Range "01-03" — take first number.
    let first = if let Some(pos) = int_part.find('-') {
        &int_part[..pos]
    } else {
        int_part
    };

    let num = first.parse::<u32>().ok()?;

    // Sanity: episode numbers are typically < 2000.
    if num > 1999 {
        return None;
    }

    Some((s.to_string(), num))
}

/// Try to parse a resolution string.
/// Handles: "1920x1080", "1280x720", "1080p", "720p", etc.
fn parse_resolution(s: &str) -> Option<String> {
    let lower = s.to_lowercase();

    // "1920x1080" → "1080p"
    if let Some(pos) = lower.find('x') {
        let height = &lower[pos + 1..];
        if height.parse::<u32>().is_ok() {
            return Some(format!("{height}p"));
        }
    }

    // Already a resolution like "1080p"
    if lower.ends_with('p') || lower.ends_with('i') {
        let num_part = &lower[..lower.len() - 1];
        if num_part.parse::<u32>().is_ok() {
            return Some(lower);
        }
    }

    None
}

/// Apply a keyword match to the appropriate element field.
fn apply_keyword(kind: KeywordKind, text: &str, elements: &mut Elements) {
    match kind {
        KeywordKind::VideoCodec => {
            if elements.video_codec.is_none() {
                elements.video_codec = Some(text.to_string());
            }
        }
        KeywordKind::AudioCodec => {
            if elements.audio_codec.is_none() {
                elements.audio_codec = Some(text.to_string());
            }
        }
        KeywordKind::Resolution => {
            if elements.resolution.is_none() {
                elements.resolution = Some(text.to_string());
            }
        }
        KeywordKind::Source => {
            if elements.source.is_none() {
                elements.source = Some(text.to_string());
            }
        }
        _ => {} // Other categories noted but not stored separately yet.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typical_subgroup_format() {
        let r = parse("[SubsPlease] Sousou no Frieren - 05 (1080p) [ABCD1234].mkv");
        assert_eq!(r.title.as_deref(), Some("Sousou no Frieren"));
        assert_eq!(r.episode_number, Some(5));
        assert_eq!(r.release_group.as_deref(), Some("SubsPlease"));
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.checksum.as_deref(), Some("ABCD1234"));
    }

    #[test]
    fn test_underscore_format() {
        let r = parse("[HorribleSubs]_Naruto_Shippuuden_-_500_[720p].mkv");
        assert_eq!(r.title.as_deref(), Some("Naruto Shippuuden"));
        assert_eq!(r.episode_number, Some(500));
        assert_eq!(r.release_group.as_deref(), Some("HorribleSubs"));
    }

    #[test]
    fn test_no_group() {
        let r = parse("Steins;Gate - 01 [1080p][HEVC].mkv");
        assert_eq!(r.title.as_deref(), Some("Steins;Gate"));
        assert_eq!(r.episode_number, Some(1));
        assert_eq!(r.video_codec.as_deref(), Some("HEVC"));
    }

    #[test]
    fn test_version_suffix() {
        let r = parse("[Group] Title - 05v2 [720p].mkv");
        assert_eq!(r.episode_number, Some(5));
        assert_eq!(r.episode.as_deref(), Some("05v2"));
    }

    #[test]
    fn test_resolution_wxh() {
        let r = parse("[Group] Title - 01 (1920x1080) [x264].mkv");
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
    }

    #[test]
    fn test_episode_number_parsing() {
        assert_eq!(parse_episode_number("05"), Some(("05".into(), 5)));
        assert_eq!(parse_episode_number("12v2"), Some(("12v2".into(), 12)));
        assert_eq!(parse_episode_number("12.5"), Some(("12.5".into(), 12)));
        assert_eq!(parse_episode_number("2024"), None); // Year, not episode
        assert_eq!(parse_episode_number("abc"), None);
    }
}
