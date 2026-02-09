pub mod episode;
pub mod season;
pub mod title;

use crate::elements::Elements;
use crate::keyword::{self, KeywordKind};
use crate::tokenizer::{self, Token, TokenKind};

/// Parse an anime filename into its component elements.
///
/// Uses a 10-pass strategy inspired by anitomy:
/// 1. Keywords in bracketed tokens (contextual matching)
/// 2. Release group (first unidentified bracket before free text)
/// 3. Checksum (8-char hex in brackets)
/// 4. Keywords in free text (contextual matching)
/// 5. Resolution (NNNNxNNNN or NNNNp)
/// 6. Year (4-digit 1950–2050)
/// 7. Season (S2, "2nd Season", 第2期, etc.)
/// 8. Episode number (13 strategies)
/// 9. Title (remaining unidentified free text before episode)
/// 10. Episode title (unidentified text after episode number)
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
    let (tokens, extension) = tokenizer::tokenize(filename);
    let mut elements = Elements::default();
    let mut identified = vec![false; tokens.len()];

    elements.file_extension = extension;

    // Pass 1: Identify keywords in bracketed tokens (contextual).
    identify_keywords_contextual(&tokens, &mut elements, &mut identified, true);

    // Pass 2: Extract release group (first unidentified bracket before free text).
    extract_release_group(&tokens, &mut elements, &mut identified);

    // Pass 3: Extract checksum (8-char hex in brackets).
    extract_checksum(&tokens, &mut elements, &mut identified);

    // Pass 4: Identify keywords in free text (contextual — ambiguous keywords skipped).
    identify_keywords_contextual(&tokens, &mut elements, &mut identified, false);

    // Pass 5: Extract resolution from remaining tokens.
    extract_resolution(&tokens, &mut elements, &mut identified);

    // Pass 6: Extract year.
    extract_year(&tokens, &mut elements, &mut identified);

    // Pass 7: Extract season.
    extract_season(&tokens, &mut elements, &mut identified);

    // Pass 8: Extract episode number.
    let episode_idx = extract_episode(&tokens, &mut elements, &mut identified);

    // Pass 9: Extract title.
    elements.title = title::extract_title(&tokens, &identified);

    // Pass 10: Extract episode title.
    elements.episode_title = title::extract_episode_title(&tokens, &identified, episode_idx);

    elements
}

/// Pass 1 & 4: Identify keywords using contextual matching.
/// When `enclosed_only` is true, only processes Bracketed tokens.
/// When false, processes FreeText tokens (ambiguous keywords are skipped).
fn identify_keywords_contextual(
    tokens: &[Token],
    elements: &mut Elements,
    identified: &mut [bool],
    enclosed_only: bool,
) {
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }

        let target_kind = if enclosed_only {
            TokenKind::Bracketed
        } else {
            TokenKind::FreeText
        };
        if token.kind != target_kind {
            continue;
        }

        let is_enclosed = token.kind == TokenKind::Bracketed || token.is_enclosed;

        if let Some(entry) = keyword::lookup_contextual(&token.text, is_enclosed) {
            // Skip PREFIX_NUMBER keywords — they're handled by dedicated passes
            // (season, episode, volume) which need to see them in context.
            if entry.flags.contains(keyword::KeywordFlags::PREFIX_NUMBER) {
                continue;
            }
            apply_keyword(entry.kind, &token.text, elements);
            identified[i] = true;
        }
    }
}

/// Pass 2: First bracketed token (before free text) is likely the release group.
fn extract_release_group(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    if elements.release_group.is_some() {
        return;
    }
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if token.kind == TokenKind::Bracketed
            && keyword::lookup_contextual(&token.text, true).is_none()
            && !is_checksum(&token.text)
        {
            elements.release_group = Some(token.text.clone());
            identified[i] = true;
            return;
        }
        if token.kind == TokenKind::FreeText && token.text != "-" {
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

/// Pass 5: Extract resolution patterns from remaining tokens.
fn extract_resolution(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    if elements.resolution.is_some() {
        return;
    }
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if matches!(token.kind, TokenKind::FreeText | TokenKind::Bracketed) {
            if let Some(res) = parse_resolution(&token.text) {
                elements.resolution = Some(res);
                identified[i] = true;
                return;
            }
        }
    }
}

/// Pass 6: Extract year (4-digit 1950-2050).
fn extract_year(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    if elements.year.is_some() {
        return;
    }
    // Prefer years in brackets.
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if token.kind == TokenKind::Bracketed {
            if let Some(year) = parse_year(&token.text) {
                elements.year = Some(year);
                identified[i] = true;
                return;
            }
        }
    }
    // Fallback: year in free text (only after some title text has been seen).
    let mut saw_text = false;
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if token.kind == TokenKind::FreeText && token.text != "-" {
            if saw_text {
                if let Some(year) = parse_year(&token.text) {
                    elements.year = Some(year);
                    identified[i] = true;
                    return;
                }
            }
            saw_text = true;
        }
    }
}

/// Pass 7: Extract season number.
fn extract_season(tokens: &[Token], elements: &mut Elements, identified: &mut [bool]) {
    if elements.season_number.is_some() {
        return;
    }

    // Try multi-token season patterns first: reconstruct phrases from consecutive tokens.
    // Check individual tokens against season strategies.
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if matches!(token.kind, TokenKind::FreeText | TokenKind::Bracketed) {
            if let Some(m) = season::try_extract(&token.text) {
                elements.season = Some(m.raw);
                elements.season_number = Some(m.number);
                identified[i] = true;
                return;
            }
        }
    }

    // Try multi-word patterns: "Season 2", "2nd Season", "Saison 3".
    // Look at pairs of consecutive free text tokens.
    let free_indices: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(i, t)| !identified[*i] && t.kind == TokenKind::FreeText && t.text != "-")
        .map(|(i, _)| i)
        .collect();

    for pair in free_indices.windows(2) {
        let combined = format!("{} {}", tokens[pair[0]].text, tokens[pair[1]].text);
        if let Some(m) = season::try_extract(&combined) {
            elements.season = Some(m.raw);
            elements.season_number = Some(m.number);
            identified[pair[0]] = true;
            identified[pair[1]] = true;
            return;
        }
    }
}

/// Pass 8: Extract episode number using 13 strategies.
/// Returns the token index where the episode was found (for episode title extraction).
fn extract_episode(
    tokens: &[Token],
    elements: &mut Elements,
    identified: &mut [bool],
) -> Option<usize> {
    // Strategy 3: Dash-separated "- 08", "- 08v2".
    // This is the most common anime filename pattern.
    for i in 0..tokens.len() {
        if identified[i] || tokens[i].kind != TokenKind::FreeText {
            continue;
        }
        if tokens[i].text == "-" {
            identified[i] = true;
            if let Some(next) = next_free_text(tokens, identified, i) {
                if let Some(m) = episode::try_extract(&tokens[next].text) {
                    apply_episode(&m, elements);
                    identified[next] = true;
                    return Some(next);
                }
            }
        }
    }

    // Try episode extraction on all remaining unidentified tokens.
    // Look for combined patterns (S01E05) and keyword-prefixed patterns first.
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if matches!(token.kind, TokenKind::FreeText | TokenKind::Bracketed) {
            if let Some(m) = episode::try_extract(&token.text) {
                // For combined S01E05, also set the season.
                apply_episode(&m, elements);
                identified[i] = true;
                return Some(i);
            }
        }
    }

    // Strategy 10: Isolated bracket number [12] after title text.
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] || token.kind != TokenKind::Bracketed {
            continue;
        }
        if let Some(m) = episode::try_plain_number(&token.text) {
            apply_episode(&m, elements);
            identified[i] = true;
            return Some(i);
        }
    }

    // Strategy 11: First number after title text.
    let mut saw_text = false;
    for (i, token) in tokens.iter().enumerate() {
        if identified[i] {
            continue;
        }
        if token.kind == TokenKind::FreeText && token.text != "-" {
            if saw_text {
                if let Some(m) = episode::try_plain_number(&token.text) {
                    apply_episode(&m, elements);
                    identified[i] = true;
                    return Some(i);
                }
            } else {
                // Check if this is text (not a number) to mark as "seen title text".
                if token.text.parse::<u32>().is_err() {
                    saw_text = true;
                }
            }
        }
    }

    None
}

/// Apply an episode match to the elements.
fn apply_episode(m: &episode::EpisodeMatch, elements: &mut Elements) {
    elements.episode = Some(m.raw.clone());
    elements.episode_number = Some(m.number);
    if let Some(season) = m.season {
        if elements.season_number.is_none() {
            elements.season_number = Some(season);
            elements.season = Some(season.to_string());
        }
    }
    if let Some(ref version) = m.version {
        elements.release_version = Some(version.clone());
    }
}

/// Find the next unidentified free text token after index `start`.
fn next_free_text(tokens: &[Token], identified: &[bool], start: usize) -> Option<usize> {
    for i in (start + 1)..tokens.len() {
        if identified[i] {
            continue;
        }
        if tokens[i].kind == TokenKind::FreeText && tokens[i].text != "-" {
            return Some(i);
        }
        if tokens[i].kind == TokenKind::Bracketed {
            return None;
        }
    }
    None
}

/// Try to parse a resolution string.
fn parse_resolution(s: &str) -> Option<String> {
    let lower = s.to_lowercase();

    // "1920x1080" → "1080p"
    if let Some(pos) = lower.find('x') {
        let height = &lower[pos + 1..];
        if height.parse::<u32>().is_ok() {
            return Some(format!("{height}p"));
        }
    }

    // Already a resolution like "1080p" or "1080i"
    if lower.ends_with('p') || lower.ends_with('i') {
        let num_part = &lower[..lower.len() - 1];
        if num_part.parse::<u32>().is_ok() {
            return Some(lower);
        }
    }

    None
}

/// Try to parse a 4-digit year (1950-2050).
fn parse_year(s: &str) -> Option<u32> {
    if s.len() == 4 {
        if let Ok(n) = s.parse::<u32>() {
            if (1950..=2050).contains(&n) {
                return Some(n);
            }
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
        KeywordKind::VideoColorDepth
        | KeywordKind::VideoDynamicRange
        | KeywordKind::VideoTerm
        | KeywordKind::VideoFrameRate => {
            elements.video_term.push(text.to_string());
        }
        KeywordKind::AudioChannels | KeywordKind::AudioTerm => {
            elements.audio_term.push(text.to_string());
        }
        KeywordKind::Language => {
            elements.language.push(text.to_string());
        }
        KeywordKind::Subtitles => {
            elements.subtitles.push(text.to_string());
        }
        KeywordKind::ReleaseInfo => {
            elements.release_info.push(text.to_string());
        }
        KeywordKind::ReleaseVersion => {
            if elements.release_version.is_none() {
                elements.release_version = Some(text.to_string());
            }
        }
        KeywordKind::EpisodeType => {
            if elements.anime_type.is_none() {
                elements.anime_type = Some(text.to_string());
            }
        }
        KeywordKind::StreamingSource => {
            if elements.streaming_source.is_none() {
                elements.streaming_source = Some(text.to_string());
            }
        }
        KeywordKind::Season
        | KeywordKind::Episode
        | KeywordKind::Part
        | KeywordKind::Volume
        | KeywordKind::DeviceCompat
        | KeywordKind::FileExtension => {
            // Handled by dedicated passes or not stored separately.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Original tests (must pass unchanged) ────────────────────

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
        assert_eq!(
            episode::try_extract("05").map(|m| (m.raw.clone(), m.number)),
            Some(("05".into(), 5))
        );
        assert_eq!(
            episode::try_extract("12v2").map(|m| (m.raw.clone(), m.number)),
            Some(("12v2".into(), 12))
        );
        assert_eq!(
            episode::try_extract("12.5").map(|m| (m.raw.clone(), m.number)),
            Some(("12.5".into(), 12))
        );
        assert!(episode::try_extract("2024").is_none());
        assert!(episode::try_extract("abc").is_none());
    }

    // ── New tests ───────────────────────────────────────────────

    #[test]
    fn test_combined_s01e05() {
        let r = parse("[SubsPlease] Title S01E05 [1080p].mkv");
        assert_eq!(r.episode_number, Some(5));
        assert_eq!(r.season_number, Some(1));
        assert_eq!(r.title.as_deref(), Some("Title"));
    }

    #[test]
    fn test_combined_01x05() {
        let r = parse("[Group] Title - 01x05 [720p].mkv");
        assert_eq!(r.episode_number, Some(5));
        assert_eq!(r.season_number, Some(1));
    }

    #[test]
    fn test_fractional_episode() {
        let r = parse("[Group] Title - 07.5 [1080p].mkv");
        assert_eq!(r.episode_number, Some(7));
        assert_eq!(r.episode.as_deref(), Some("07.5"));
    }

    #[test]
    fn test_japanese_counter() {
        let r = parse("[Group] Title 第05話 [1080p].mkv");
        assert_eq!(r.episode_number, Some(5));
    }

    #[test]
    fn test_year_extraction() {
        let r = parse("[Group] Title (2023) - 05 [1080p].mkv");
        assert_eq!(r.year, Some(2023));
        assert_eq!(r.episode_number, Some(5));
    }

    #[test]
    fn test_season_extraction() {
        let r = parse("[Group] Title S2 - 05 [1080p].mkv");
        assert_eq!(r.season_number, Some(2));
        assert_eq!(r.episode_number, Some(5));
    }

    #[test]
    fn test_season_word() {
        let r = parse("[Group] Title Season 2 - 05 [1080p].mkv");
        assert_eq!(r.season_number, Some(2));
    }

    #[test]
    fn test_nth_season() {
        let r = parse("[Group] Title 2nd Season - 05 [1080p].mkv");
        assert_eq!(r.season_number, Some(2));
    }

    #[test]
    fn test_japanese_season() {
        let r = parse("[Group] Title 第2期 - 05 [1080p].mkv");
        assert_eq!(r.season_number, Some(2));
    }

    #[test]
    fn test_keyword_ep_prefix() {
        let r = parse("[Group] Title EP05 [1080p].mkv");
        assert_eq!(r.episode_number, Some(5));
    }

    #[test]
    fn test_version_extraction() {
        let r = parse("[Group] Title - 05v2 [1080p].mkv");
        assert_eq!(r.episode_number, Some(5));
        assert_eq!(r.release_version.as_deref(), Some("v2"));
    }

    #[test]
    fn test_file_extension_preserved() {
        let r = parse("[Group] Title - 05 [1080p].mkv");
        assert_eq!(r.file_extension.as_deref(), Some("mkv"));
    }

    #[test]
    fn test_ambiguous_keyword_in_brackets() {
        let r = parse("[Group] Title - 05 [BD][1080p].mkv");
        assert_eq!(r.source.as_deref(), Some("BD"));
    }

    #[test]
    fn test_streaming_source() {
        let r = parse("[SubsPlease] Title - 05 (1080p) [AMZN].mkv");
        // AMZN is not 8 hex chars so it won't be treated as checksum.
        // It's a StreamingSource keyword.
        assert_eq!(r.streaming_source.as_deref(), Some("AMZN"));
    }

    #[test]
    fn test_anime_type_ova() {
        let r = parse("[Group] Title OVA - 01 [1080p].mkv");
        assert_eq!(r.anime_type.as_deref(), Some("OVA"));
        assert_eq!(r.episode_number, Some(1));
    }

    #[test]
    fn test_cjk_brackets() {
        let r = parse("【SubGroup】 Title - 05.mkv");
        assert_eq!(r.release_group.as_deref(), Some("SubGroup"));
        assert_eq!(r.episode_number, Some(5));
    }

    #[test]
    fn test_webm_extension() {
        let r = parse("[Group] Title - 05.webm");
        assert_eq!(r.file_extension.as_deref(), Some("webm"));
        assert_eq!(r.episode_number, Some(5));
    }

    #[test]
    fn test_no_episode() {
        let r = parse("[Group] Title [1080p].mkv");
        assert_eq!(r.title.as_deref(), Some("Title"));
        assert_eq!(r.episode_number, None);
    }

    #[test]
    fn test_multiple_keywords() {
        let r = parse("[Group] Title - 05 [1080p][HEVC][FLAC].mkv");
        assert_eq!(r.video_codec.as_deref(), Some("HEVC"));
        assert_eq!(r.audio_codec.as_deref(), Some("FLAC"));
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
    }
}
