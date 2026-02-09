use regex::Regex;
use std::sync::LazyLock;

/// Result of a successful season extraction.
pub struct SeasonMatch {
    /// Raw season string.
    pub raw: String,
    /// Parsed season number.
    pub number: u32,
}

// ── Regex patterns ──────────────────────────────────────────────

/// "S2", "S01" — standalone season prefix.
static RE_S_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^S(\d{1,2})$").unwrap());

/// "Season 2", "Season II", "Saison 2".
static RE_SEASON_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:Season|Saison)\s+([\dIVXivx]+)$").unwrap());

/// "2nd Season", "3rd Season", etc.
static RE_NTH_SEASON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(\d{1,2})(?:st|nd|rd|th)\s+Season$").unwrap());

/// Japanese: "第2期", "2期".
static RE_JAPANESE_SEASON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:第)?(\d{1,2})期$").unwrap());

/// Try all season extraction strategies.
pub fn try_extract(text: &str) -> Option<SeasonMatch> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    if let Some(m) = try_s_prefix(text) {
        return Some(m);
    }
    if let Some(m) = try_season_word(text) {
        return Some(m);
    }
    if let Some(m) = try_nth_season(text) {
        return Some(m);
    }
    if let Some(m) = try_japanese_season(text) {
        return Some(m);
    }

    None
}

/// "S2", "S01".
fn try_s_prefix(text: &str) -> Option<SeasonMatch> {
    let caps = RE_S_PREFIX.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    Some(SeasonMatch {
        raw: text.to_string(),
        number,
    })
}

/// "Season 2", "Season II", "Saison 3".
fn try_season_word(text: &str) -> Option<SeasonMatch> {
    let caps = RE_SEASON_WORD.captures(text)?;
    let value = &caps[1];
    let number = parse_number_or_roman(value)?;
    Some(SeasonMatch {
        raw: text.to_string(),
        number,
    })
}

/// "2nd Season", "3rd Season".
fn try_nth_season(text: &str) -> Option<SeasonMatch> {
    let caps = RE_NTH_SEASON.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    Some(SeasonMatch {
        raw: text.to_string(),
        number,
    })
}

/// "第2期", "2期".
fn try_japanese_season(text: &str) -> Option<SeasonMatch> {
    let caps = RE_JAPANESE_SEASON.captures(text)?;
    let number: u32 = caps[1].parse().ok()?;
    Some(SeasonMatch {
        raw: text.to_string(),
        number,
    })
}

/// Parse a number that might be Arabic or Roman numerals.
fn parse_number_or_roman(s: &str) -> Option<u32> {
    if let Ok(n) = s.parse::<u32>() {
        return Some(n);
    }
    roman_to_u32(s)
}

/// Simple Roman numeral to u32 conversion (I-XX range).
fn roman_to_u32(s: &str) -> Option<u32> {
    let s = s.to_uppercase();
    let mut total: i32 = 0;
    let mut prev = 0i32;

    for c in s.chars().rev() {
        let value = match c {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            _ => return None,
        };
        if value < prev {
            total -= value;
        } else {
            total += value;
        }
        prev = value;
    }

    if total > 0 {
        Some(total as u32)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s_prefix() {
        let m = try_extract("S2").unwrap();
        assert_eq!(m.number, 2);
        let m = try_extract("S01").unwrap();
        assert_eq!(m.number, 1);
    }

    #[test]
    fn test_season_word() {
        let m = try_extract("Season 2").unwrap();
        assert_eq!(m.number, 2);
        let m = try_extract("Saison 3").unwrap();
        assert_eq!(m.number, 3);
    }

    #[test]
    fn test_season_roman() {
        let m = try_extract("Season II").unwrap();
        assert_eq!(m.number, 2);
        let m = try_extract("Season IV").unwrap();
        assert_eq!(m.number, 4);
    }

    #[test]
    fn test_nth_season() {
        let m = try_extract("2nd Season").unwrap();
        assert_eq!(m.number, 2);
        let m = try_extract("3rd Season").unwrap();
        assert_eq!(m.number, 3);
    }

    #[test]
    fn test_japanese_season() {
        let m = try_extract("第2期").unwrap();
        assert_eq!(m.number, 2);
        let m = try_extract("2期").unwrap();
        assert_eq!(m.number, 2);
    }

    #[test]
    fn test_roman_numerals() {
        assert_eq!(roman_to_u32("I"), Some(1));
        assert_eq!(roman_to_u32("IV"), Some(4));
        assert_eq!(roman_to_u32("IX"), Some(9));
        assert_eq!(roman_to_u32("XII"), Some(12));
    }
}
