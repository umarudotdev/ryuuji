//! 8-level title normalization pipeline for anime title matching.
//!
//! Transforms titles through sequential normalization levels to improve
//! fuzzy and exact matching accuracy across different naming conventions.
//!
//! Based on Taiga's normalization pipeline adapted for Rust.

use unicode_normalization::UnicodeNormalization;

/// Apply the full 8-level normalization pipeline.
///
/// Levels applied in order:
/// 1. Unicode NFKC + case folding
/// 2. Character transliteration
/// 3. Roman numeral conversion
/// 4. Ordinal conversion
/// 5. Season keyword normalization
/// 6. Stop word removal
/// 7. Punctuation erasure
/// 8. Whitespace collapse
pub fn normalize(s: &str) -> String {
    let s = unicode_normalize(s);
    let s = transliterate(&s);
    let s = convert_roman_numerals(&s);
    let s = convert_ordinals(&s);
    let s = normalize_season_keywords(&s);
    let s = remove_stop_words(&s);
    let s = erase_punctuation(&s);
    collapse_whitespace(&s)
}

/// Apply NFKC normalization (fullwidth → ASCII, compose diacritics) and lowercase.
fn unicode_normalize(s: &str) -> String {
    s.nfkc().collect::<String>().to_lowercase()
}

/// Replace common character substitutions used in anime titles.
fn transliterate(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '@' => result.push('a'),
            '0' if looks_like_letter_o(&chars, i) => result.push('o'),
            '\u{00D7}' | '\u{2715}' | '\u{2716}' => result.push('x'), // ×, ✕, ✖
            '\u{2019}' | '\u{2018}' | '\u{02BC}' => result.push('\''), // curly quotes → straight
            '\u{201C}' | '\u{201D}' => result.push('"'),              // curly double quotes
            '\u{2013}' | '\u{2014}' => result.push('-'),              // en/em dash → hyphen
            '\u{2026}' => result.push_str("..."),                     // ellipsis
            '\u{00E6}' => result.push_str("ae"),                      // æ
            '\u{0153}' => result.push_str("oe"),                      // œ
            '\u{00F0}' => result.push('d'),                           // ð
            '\u{00FE}' => result.push_str("th"),                      // þ
            '\u{00DF}' => result.push_str("ss"),                      // ß
            c => result.push(c),
        }
        i += 1;
    }

    result
}

/// Heuristic: '0' looks like letter 'O' if surrounded by letters (not digits).
/// e.g., "Danganr0npa" → "Danganronpa"
fn looks_like_letter_o(chars: &[char], pos: usize) -> bool {
    let before = pos > 0 && chars[pos - 1].is_alphabetic();
    let after = pos + 1 < chars.len() && chars[pos + 1].is_alphabetic();
    before && after
}

/// Roman numeral values for conversion, ordered longest-first for greedy matching.
const ROMAN_NUMERALS: &[(&str, u32)] = &[
    ("xiii", 13),
    ("xii", 12),
    ("xi", 11),
    ("viii", 8),
    ("vii", 7),
    ("vi", 6),
    ("iv", 4),
    ("ix", 9),
    ("x", 10),
    ("v", 5),
    ("iii", 3),
    ("ii", 2),
];

/// Convert roman numerals at word boundaries to arabic numbers.
///
/// Only converts when the numeral is a standalone word (bounded by spaces/punctuation/edges).
/// Careful not to match substrings: "Hawaii" should not become "Hawa2".
fn convert_roman_numerals(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let converted: Vec<String> = words
        .iter()
        .map(|&word| {
            let (base, suffix) = split_trailing_punct(word);
            let lower = base.to_lowercase();

            for &(roman, value) in ROMAN_NUMERALS {
                if lower == roman {
                    return format!("{value}{suffix}");
                }
            }
            word.to_string()
        })
        .collect();

    converted.join(" ")
}

/// Split trailing punctuation from a word (e.g., "iii:" → ("iii", ":")).
fn split_trailing_punct(s: &str) -> (&str, &str) {
    let end = s
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_punctuation())
        .last()
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// Convert ordinal numbers ("1st", "2nd", "3rd", "4th", etc.) to plain numbers.
fn convert_ordinals(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let converted: Vec<String> = words
        .iter()
        .map(|&word| {
            let lower = word.to_lowercase();
            if let Some(num_str) = lower
                .strip_suffix("st")
                .or_else(|| lower.strip_suffix("nd"))
                .or_else(|| lower.strip_suffix("rd"))
                .or_else(|| lower.strip_suffix("th"))
            {
                if num_str.chars().all(|c| c.is_ascii_digit()) && !num_str.is_empty() {
                    return num_str.to_string();
                }
            }
            word.to_string()
        })
        .collect();

    converted.join(" ")
}

/// Normalize season references to just the number.
///
/// Patterns handled:
/// - "Season 2" / "season2" → "2"
/// - "S2" (standalone) → "2"
/// - "2nd Season" → "2 season" (ordinal already handled, "season" removed by stop words)
/// - "Cour 2" → "2"
fn normalize_season_keywords(s: &str) -> String {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut result: Vec<String> = Vec::with_capacity(words.len());
    let mut i = 0;

    while i < words.len() {
        let lower = words[i].to_lowercase();

        if matches!(lower.as_str(), "season" | "cour" | "series") {
            if let Some(next) = words.get(i + 1) {
                if next.chars().all(|c| c.is_ascii_digit()) && !next.is_empty() {
                    result.push(next.to_string());
                    i += 2;
                    continue;
                }
            }
            for keyword in &["season", "cour", "series"] {
                if let Some(digits) = lower.strip_prefix(keyword) {
                    if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                        result.push(digits.to_string());
                        i += 1;
                        continue;
                    }
                }
            }
            result.push(words[i].to_string());
            i += 1;
            continue;
        }

        if lower.starts_with('s')
            && lower.len() > 1
            && lower[1..].chars().all(|c| c.is_ascii_digit())
        {
            result.push(lower[1..].to_string());
            i += 1;
            continue;
        }

        result.push(words[i].to_string());
        i += 1;
    }

    result.join(" ")
}

/// Words to remove entirely from titles during normalization.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "episode", "ep", "ep.", "tv", "ova", "ona", "season", "cour", "part",
];

/// Apply stop word removal and synonym normalization.
///
/// - Removes common articles and filler words
/// - Normalizes "&" → "and"
/// - Normalizes "oad"/"oav" → "ova"
/// - Strips parenthesized tags like "(tv)", "(ova)"
fn remove_stop_words(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_parens = false;
    let mut paren_content = String::new();

    for c in s.chars() {
        match c {
            '(' => {
                in_parens = true;
                paren_content.clear();
            }
            ')' if in_parens => {
                in_parens = false;
                let lower = paren_content.trim().to_lowercase();
                if !matches!(
                    lower.as_str(),
                    "tv" | "ova" | "ona" | "oad" | "oav" | "special" | "specials"
                ) && !lower.chars().all(|c| c.is_ascii_digit())
                {
                    result.push('(');
                    result.push_str(&paren_content);
                    result.push(')');
                }
            }
            _ if in_parens => paren_content.push(c),
            _ => result.push(c),
        }
    }

    let words: Vec<&str> = result.split_whitespace().collect();
    let filtered: Vec<String> = words
        .iter()
        .filter_map(|&word| {
            let lower = word.to_lowercase();

            if STOP_WORDS.contains(&lower.as_str()) {
                return None;
            }

            match lower.as_str() {
                "&" => Some("and".to_string()),
                "oad" | "oav" => Some("ova".to_string()),
                _ => Some(word.to_string()),
            }
        })
        .collect();

    filtered.join(" ")
}

/// Strip all Unicode punctuation and symbol characters, keeping alphanumerics and whitespace.
fn erase_punctuation(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect()
}

/// Trim and collapse multiple whitespace runs to a single space.
fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullwidth_ascii() {
        assert_eq!(unicode_normalize("ＦＵＬＬＷＩＤＴＨ"), "fullwidth");
    }

    #[test]
    fn case_folding() {
        assert_eq!(unicode_normalize("Attack On TITAN"), "attack on titan");
    }

    #[test]
    fn nfkc_diacritics() {
        let result = unicode_normalize("café");
        assert_eq!(result, "café");
    }

    #[test]
    fn at_sign_to_a() {
        assert_eq!(transliterate("Dre@m"), "Dream");
    }

    #[test]
    fn zero_as_letter_o() {
        assert_eq!(transliterate("Danganr0npa"), "Danganronpa");
    }

    #[test]
    fn zero_not_letter_o_in_number() {
        assert_eq!(transliterate("Season 10"), "Season 10");
    }

    #[test]
    fn curly_quotes() {
        assert_eq!(transliterate("it\u{2019}s"), "it's");
    }

    #[test]
    fn em_dash() {
        assert_eq!(transliterate("Title\u{2014}Subtitle"), "Title-Subtitle");
    }

    #[test]
    fn multiplication_sign() {
        assert_eq!(transliterate("Hunter\u{00D7}Hunter"), "HunterxHunter");
    }

    #[test]
    fn ligatures() {
        assert_eq!(transliterate("æther"), "aether");
        assert_eq!(transliterate("œuvre"), "oeuvre");
    }

    #[test]
    fn roman_numeral_basic() {
        assert_eq!(convert_roman_numerals("series ii"), "series 2");
        assert_eq!(convert_roman_numerals("part iv"), "part 4");
        assert_eq!(convert_roman_numerals("title xiii"), "title 13");
    }

    #[test]
    fn roman_numeral_word_boundary() {
        assert_eq!(convert_roman_numerals("hawaii"), "hawaii");
    }

    #[test]
    fn roman_numeral_standalone() {
        assert_eq!(convert_roman_numerals("iii"), "3");
    }

    #[test]
    fn ordinal_conversion() {
        assert_eq!(convert_ordinals("1st season"), "1 season");
        assert_eq!(convert_ordinals("2nd part"), "2 part");
        assert_eq!(convert_ordinals("3rd series"), "3 series");
        assert_eq!(convert_ordinals("4th movie"), "4 movie");
    }

    #[test]
    fn ordinal_not_text() {
        assert_eq!(convert_ordinals("the best"), "the best");
    }

    #[test]
    fn season_n() {
        assert_eq!(
            normalize_season_keywords("attack on titan season 2"),
            "attack on titan 2"
        );
    }

    #[test]
    fn standalone_s_number() {
        assert_eq!(normalize_season_keywords("title s2"), "title 2");
    }

    #[test]
    fn cour_keyword() {
        assert_eq!(normalize_season_keywords("title cour 2"), "title 2");
    }

    #[test]
    fn remove_the_prefix() {
        let result = remove_stop_words("the seven deadly sins");
        assert_eq!(result, "seven deadly sins");
    }

    #[test]
    fn ampersand_to_and() {
        let result = remove_stop_words("romeo & juliet");
        assert_eq!(result, "romeo and juliet");
    }

    #[test]
    fn strip_tv_tag() {
        let result = remove_stop_words("title (TV)");
        assert_eq!(result, "title");
    }

    #[test]
    fn keep_meaningful_parens() {
        let result = remove_stop_words("title (Director's Cut)");
        assert!(result.contains("Director's Cut"));
    }

    #[test]
    fn oad_to_ova() {
        let result = remove_stop_words("title oad");
        assert_eq!(result, "title ova");
    }

    #[test]
    fn erase_all_punctuation() {
        assert_eq!(erase_punctuation("hello: world!"), "hello world");
        assert_eq!(erase_punctuation("test-case_foo"), "testcasefoo");
    }

    #[test]
    fn collapse_spaces() {
        assert_eq!(collapse_whitespace("  hello   world  "), "hello world");
    }

    #[test]
    fn full_pipeline_fullwidth() {
        assert_eq!(normalize("ＦＵＬＬＷＩＤＴＨ"), "fullwidth");
    }

    #[test]
    fn full_pipeline_roman_numeral() {
        assert_eq!(normalize("Series II"), "2");
        assert_eq!(normalize("Jojo Part III"), "jojo 3");
    }

    #[test]
    fn full_pipeline_hawaii_safe() {
        assert_eq!(normalize("Hawaii"), "hawaii");
    }

    #[test]
    fn full_pipeline_season_keyword() {
        assert_eq!(normalize("Attack on Titan Season 2"), "attack on titan 2");
    }

    #[test]
    fn full_pipeline_stop_words() {
        assert_eq!(normalize("The Seven Deadly Sins"), "seven deadly sins");
    }

    #[test]
    fn full_pipeline_complex() {
        assert_eq!(normalize("The Title: 2nd Season (TV)"), "title 2");
    }

    #[test]
    fn full_pipeline_s_prefix() {
        assert_eq!(normalize("My Hero Academia S3"), "my hero academia 3");
    }

    #[test]
    fn full_pipeline_ampersand() {
        assert_eq!(normalize("Romeo & Juliet"), "romeo and juliet");
    }

    #[test]
    fn full_pipeline_empty() {
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn full_pipeline_only_punctuation() {
        assert_eq!(normalize("---"), "");
    }

    #[test]
    fn full_pipeline_preserves_numbers() {
        assert_eq!(normalize("Bleach 1000"), "bleach 1000");
    }
}
