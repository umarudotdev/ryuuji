use regex::Regex;
use std::sync::LazyLock;

/// Result of a successful episode extraction.
pub struct EpisodeMatch {
    /// Raw episode string (e.g., "05", "12.5", "01-13").
    pub raw: String,
    /// Parsed episode number.
    pub number: u32,
    /// Season number if extracted from combined pattern (e.g., S01E05 → season 1).
    pub season: Option<u32>,
    /// Release version if extracted (e.g., "05v2" → version "v2").
    pub version: Option<String>,
}

// ── Regex patterns (compiled once) ──────────────────────────────

static RE_COMBINED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^S(\d{1,2})E(\d{1,4})$").unwrap());

static RE_COMBINED_X: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,2})[xX](\d{1,4})$").unwrap());

static RE_KEYWORD_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:EP\.?|E|EPS|EPISODE|#)\s*(\d{1,4})(?:v(\d))?$").unwrap());

static RE_VERSION_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,4})[vV](\d)$").unwrap());

static RE_FRACTIONAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,4})\.5$").unwrap());

static RE_RANGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,4})\s*[-~]\s*(\d{1,4})$").unwrap());

static RE_JAPANESE_COUNTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^第(\d{1,4})[話集]$").unwrap());

static RE_PARTIAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,4})[a-cA-C]$").unwrap());

static RE_VOL_EPISODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:Vol\.?\s*\d+\s+)?(?:EP\.?\s*)?(\d{1,4})(?:v(\d))?$").unwrap()
});

/// Try all 13 episode strategies in order of specificity.
/// Returns the first successful match.
pub fn try_extract(text: &str) -> Option<EpisodeMatch> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // Reject year-like 4-digit numbers early.
    if is_year_like(text) {
        return None;
    }

    // Strategy 1: Combined S01E05 or 01x05.
    if let Some(m) = try_combined(text) {
        return Some(m);
    }
    // Strategy 2: Keyword-prefixed EP05, E05, #05, Episode 05.
    if let Some(m) = try_keyword_prefix(text) {
        return Some(m);
    }
    // Strategy 4: Version suffix 05v2.
    if let Some(m) = try_version_suffix(text) {
        return Some(m);
    }
    // Strategy 5: Fractional 07.5.
    if let Some(m) = try_fractional(text) {
        return Some(m);
    }
    // Strategy 6: Range 01-13.
    if let Some(m) = try_range(text) {
        return Some(m);
    }
    // Strategy 7: Japanese counter 第05話, 第05集.
    if let Some(m) = try_japanese_counter(text) {
        return Some(m);
    }
    // Strategy 9: Partial 4a, 111C.
    if let Some(m) = try_partial(text) {
        return Some(m);
    }
    // Strategy 13: Volume + episode Vol.3 EP05.
    if let Some(m) = try_vol_episode(text) {
        return Some(m);
    }
    // Strategy 11/12: Plain number (handled by caller using parse_plain_number).
    if let Some(m) = try_plain_number(text) {
        return Some(m);
    }

    None
}

/// Strategy 1: Combined format S01E05 or 01x05.
fn try_combined(text: &str) -> Option<EpisodeMatch> {
    if let Some(caps) = RE_COMBINED.captures(text) {
        let season: u32 = caps[1].parse().ok()?;
        let episode: u32 = caps[2].parse().ok()?;
        return Some(EpisodeMatch {
            raw: text.to_string(),
            number: episode,
            season: Some(season),
            version: None,
        });
    }
    if let Some(caps) = RE_COMBINED_X.captures(text) {
        let season: u32 = caps[1].parse().ok()?;
        let episode: u32 = caps[2].parse().ok()?;
        return Some(EpisodeMatch {
            raw: text.to_string(),
            number: episode,
            season: Some(season),
            version: None,
        });
    }
    None
}

/// Strategy 2: Keyword-prefixed episode numbers.
fn try_keyword_prefix(text: &str) -> Option<EpisodeMatch> {
    let caps = RE_KEYWORD_PREFIX.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    if number > 1999 {
        return None;
    }
    let version = caps.get(2).map(|m| format!("v{}", m.as_str()));
    Some(EpisodeMatch {
        raw: text.to_string(),
        number,
        season: None,
        version,
    })
}

/// Strategy 4: Version suffix (05v2 → episode 5, version "v2").
fn try_version_suffix(text: &str) -> Option<EpisodeMatch> {
    let caps = RE_VERSION_SUFFIX.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    if number > 1999 {
        return None;
    }
    Some(EpisodeMatch {
        raw: text.to_string(),
        number,
        season: None,
        version: Some(format!("v{}", &caps[2])),
    })
}

/// Strategy 5: Fractional episode (07.5).
fn try_fractional(text: &str) -> Option<EpisodeMatch> {
    let caps = RE_FRACTIONAL.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    if number > 1999 {
        return None;
    }
    Some(EpisodeMatch {
        raw: text.to_string(),
        number,
        season: None,
        version: None,
    })
}

/// Strategy 6: Episode range (01-13).
fn try_range(text: &str) -> Option<EpisodeMatch> {
    let caps = RE_RANGE.captures(text)?;
    let start: u32 = caps[1].parse().ok()?;
    let end: u32 = caps[2].parse().ok()?;
    if start > 1999 || end > 1999 || start >= end {
        return None;
    }
    Some(EpisodeMatch {
        raw: text.to_string(),
        number: start,
        season: None,
        version: None,
    })
}

/// Strategy 7: Japanese counter (第05話, 第05集).
fn try_japanese_counter(text: &str) -> Option<EpisodeMatch> {
    let caps = RE_JAPANESE_COUNTER.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    if number > 1999 {
        return None;
    }
    Some(EpisodeMatch {
        raw: text.to_string(),
        number,
        season: None,
        version: None,
    })
}

/// Strategy 9: Partial episode (4a, 111C).
fn try_partial(text: &str) -> Option<EpisodeMatch> {
    let caps = RE_PARTIAL.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    if number > 1999 {
        return None;
    }
    Some(EpisodeMatch {
        raw: text.to_string(),
        number,
        season: None,
        version: None,
    })
}

/// Strategy 13: Volume + episode (Vol.3 EP05).
fn try_vol_episode(text: &str) -> Option<EpisodeMatch> {
    // Only trigger if text contains "vol" (case-insensitive).
    if !text.to_lowercase().contains("vol") {
        return None;
    }
    let caps = RE_VOL_EPISODE.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    if number > 1999 {
        return None;
    }
    let version = caps.get(2).map(|m| format!("v{}", m.as_str()));
    Some(EpisodeMatch {
        raw: text.to_string(),
        number,
        season: None,
        version,
    })
}

/// Parse a plain number as an episode number (strategies 11/12).
pub fn try_plain_number(text: &str) -> Option<EpisodeMatch> {
    let text = text.trim();
    let number: u32 = text.parse().ok()?;
    if number > 1999 || is_year_like(text) {
        return None;
    }
    Some(EpisodeMatch {
        raw: text.to_string(),
        number,
        season: None,
        version: None,
    })
}

/// Check if a 4-digit number looks like a year (1950-2050).
fn is_year_like(s: &str) -> bool {
    if s.len() == 4 {
        if let Ok(n) = s.parse::<u32>() {
            return (1950..=2050).contains(&n);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combined_s01e05() {
        let m = try_extract("S01E05").unwrap();
        assert_eq!(m.number, 5);
        assert_eq!(m.season, Some(1));
    }

    #[test]
    fn test_combined_01x05() {
        let m = try_extract("01x05").unwrap();
        assert_eq!(m.number, 5);
        assert_eq!(m.season, Some(1));
    }

    #[test]
    fn test_keyword_prefix_ep() {
        let m = try_extract("EP05").unwrap();
        assert_eq!(m.number, 5);
        assert_eq!(m.season, None);

        let m = try_extract("Episode 12").unwrap();
        assert_eq!(m.number, 12);
    }

    #[test]
    fn test_keyword_prefix_hash() {
        let m = try_extract("#03").unwrap();
        assert_eq!(m.number, 3);
    }

    #[test]
    fn test_version_suffix() {
        let m = try_extract("05v2").unwrap();
        assert_eq!(m.number, 5);
        assert_eq!(m.version.as_deref(), Some("v2"));
    }

    #[test]
    fn test_fractional() {
        let m = try_extract("07.5").unwrap();
        assert_eq!(m.number, 7);
    }

    #[test]
    fn test_range() {
        let m = try_extract("01-13").unwrap();
        assert_eq!(m.number, 1);
    }

    #[test]
    fn test_japanese_counter() {
        let m = try_extract("第05話").unwrap();
        assert_eq!(m.number, 5);

        let m = try_extract("第12集").unwrap();
        assert_eq!(m.number, 12);
    }

    #[test]
    fn test_partial() {
        let m = try_extract("4a").unwrap();
        assert_eq!(m.number, 4);
    }

    #[test]
    fn test_year_rejected() {
        assert!(try_extract("2024").is_none());
        assert!(try_extract("1999").is_none());
    }

    #[test]
    fn test_plain_number() {
        let m = try_extract("05").unwrap();
        assert_eq!(m.number, 5);

        let m = try_extract("500").unwrap();
        assert_eq!(m.number, 500);
    }
}
