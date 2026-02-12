use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::models::Anime;

/// Result of attempting to match a parsed title against the local library.
#[derive(Debug, Clone)]
pub enum MatchResult {
    /// Exact or near-exact match found.
    Matched(Anime),
    /// Fuzzy match found with confidence score (0.0–1.0).
    Fuzzy(Anime, f64),
    /// No match found.
    NoMatch,
}

/// Minimum fuzzy score (0.0–1.0) to consider a match valid.
const FUZZY_THRESHOLD: f64 = 0.6;

/// Attempt to match a parsed title against a list of known anime.
///
/// Strategy: exact → normalized → fuzzy (Skim) → NoMatch.
pub fn match_title(query: &str, candidates: &[Anime]) -> MatchResult {
    if query.is_empty() || candidates.is_empty() {
        return MatchResult::NoMatch;
    }

    let normalized_query = normalize(query);

    // Pass 1: Exact match against any title variant or synonym.
    for anime in candidates {
        if exact_match(query, anime) {
            return MatchResult::Matched(anime.clone());
        }
    }

    // Pass 2: Normalized match (lowercase, no punctuation).
    for anime in candidates {
        if normalized_match(&normalized_query, anime) {
            return MatchResult::Matched(anime.clone());
        }
    }

    // Pass 3: Fuzzy match using Skim algorithm.
    let matcher = SkimMatcherV2::default();
    let mut best_score: i64 = 0;
    let mut best_anime: Option<&Anime> = None;
    let mut max_possible: i64 = 1;

    if let Some(self_score) = matcher.fuzzy_match(&normalized_query, &normalized_query) {
        max_possible = self_score.max(1);
    }

    for anime in candidates {
        let score = best_fuzzy_score(&matcher, &normalized_query, anime);
        if score > best_score {
            best_score = score;
            best_anime = Some(anime);
        }
    }

    if let Some(anime) = best_anime {
        let confidence = best_score as f64 / max_possible as f64;
        if confidence >= FUZZY_THRESHOLD {
            return MatchResult::Fuzzy(anime.clone(), confidence);
        }
    }

    MatchResult::NoMatch
}

/// Check for exact string match against all title variants.
fn exact_match(query: &str, anime: &Anime) -> bool {
    let titles = all_titles(anime);
    titles.contains(&query)
}

/// Check for normalized match (case-insensitive, stripped punctuation).
fn normalized_match(normalized_query: &str, anime: &Anime) -> bool {
    let titles = all_titles(anime);
    titles.iter().any(|t| normalize(t) == *normalized_query)
}

/// Get the best fuzzy score across all title variants.
fn best_fuzzy_score(matcher: &SkimMatcherV2, query: &str, anime: &Anime) -> i64 {
    all_titles(anime)
        .iter()
        .filter_map(|t| matcher.fuzzy_match(&normalize(t), query))
        .max()
        .unwrap_or(0)
}

/// Collect all title strings for an anime (romaji, english, native, synonyms).
pub fn all_titles(anime: &Anime) -> Vec<&str> {
    let mut titles = Vec::new();
    if let Some(r) = &anime.title.romaji {
        titles.push(r.as_str());
    }
    if let Some(e) = &anime.title.english {
        titles.push(e.as_str());
    }
    if let Some(n) = &anime.title.native {
        titles.push(n.as_str());
    }
    for s in &anime.synonyms {
        titles.push(s.as_str());
    }
    titles
}

/// Normalize a title for comparison using the full 8-level pipeline.
///
/// Levels: NFKC + case fold → transliteration → roman numerals → ordinals →
/// season keywords → stop words → punctuation erasure → whitespace collapse.
pub fn normalize(s: &str) -> String {
    crate::normalize::normalize(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AnimeIds, AnimeTitle};

    fn frieren() -> Anime {
        Anime {
            id: 1,
            ids: AnimeIds::default(),
            title: AnimeTitle {
                romaji: Some("Sousou no Frieren".into()),
                english: Some("Frieren: Beyond Journey's End".into()),
                native: Some("葬送のフリーレン".into()),
            },
            synonyms: vec!["Frieren".into()],
            episodes: Some(28),
            cover_url: None,
            season: None,
            year: None,
            synopsis: None,
            genres: vec![],
            media_type: None,
            airing_status: None,
            mean_score: None,
            studios: vec![],
            source: None,
            rating: None,
            start_date: None,
            end_date: None,
        }
    }

    fn aot() -> Anime {
        Anime {
            id: 2,
            ids: AnimeIds::default(),
            title: AnimeTitle {
                romaji: Some("Shingeki no Kyojin".into()),
                english: Some("Attack on Titan".into()),
                native: None,
            },
            synonyms: vec![],
            episodes: Some(25),
            cover_url: None,
            season: None,
            year: None,
            synopsis: None,
            genres: vec![],
            media_type: None,
            airing_status: None,
            mean_score: None,
            studios: vec![],
            source: None,
            rating: None,
            start_date: None,
            end_date: None,
        }
    }

    #[test]
    fn test_exact_match() {
        let candidates = vec![frieren(), aot()];
        match match_title("Sousou no Frieren", &candidates) {
            MatchResult::Matched(a) => assert_eq!(a.id, 1),
            other => panic!("Expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_normalized_match() {
        let candidates = vec![frieren()];
        // Different case and missing colon.
        match match_title("sousou no frieren", &candidates) {
            MatchResult::Matched(a) => assert_eq!(a.id, 1),
            other => panic!("Expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_synonym_match() {
        let candidates = vec![frieren()];
        match match_title("Frieren", &candidates) {
            MatchResult::Matched(a) => assert_eq!(a.id, 1),
            other => panic!("Expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_fuzzy_match() {
        let candidates = vec![frieren(), aot()];
        // English title variant with different wording — should fuzzy match.
        match match_title("Frieren Beyond Journeys End", &candidates) {
            MatchResult::Fuzzy(a, _) | MatchResult::Matched(a) => {
                assert_eq!(a.id, 1);
            }
            other => panic!("Expected Fuzzy or Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_no_match() {
        let candidates = vec![frieren()];
        match match_title("Completely Different Anime", &candidates) {
            MatchResult::NoMatch => {}
            other => panic!("Expected NoMatch, got {other:?}"),
        }
    }

    #[test]
    fn test_empty_inputs() {
        assert!(matches!(
            match_title("", &[frieren()]),
            MatchResult::NoMatch
        ));
        assert!(matches!(match_title("test", &[]), MatchResult::NoMatch));
    }
}
