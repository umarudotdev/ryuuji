use std::collections::{HashMap, VecDeque};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::debug_log::CacheStats;
use crate::error::RyuujiError;
use crate::matcher::{self, MatchResult};
use crate::models::Anime;
use crate::storage::Storage;

/// Maximum number of recent query results to cache.
const QUERY_CACHE_CAPACITY: usize = 64;

/// Minimum fuzzy confidence (0.0–1.0) to consider a match valid.
const FUZZY_THRESHOLD: f64 = 0.6;

/// Pre-built in-memory index of anime titles for fast recognition.
///
/// Avoids repeated DB scans and string normalization on every detection tick.
/// The cache auto-populates from storage on the first `recognize()` call and
/// rebuilds lazily after `invalidate()`.
pub struct RecognitionCache {
    entries: Vec<Anime>,
    exact_index: HashMap<String, i64>,
    normalized_index: HashMap<String, i64>,
    query_cache: VecDeque<(String, CachedMatch)>,
    populated: bool,
    stats: CacheStats,
}

/// A cached recognition result (avoids cloning full Anime on every cache hit).
#[derive(Debug, Clone)]
enum CachedMatch {
    Matched(i64),
    Fuzzy(i64, f64),
    NoMatch,
}

impl Default for RecognitionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RecognitionCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            exact_index: HashMap::new(),
            normalized_index: HashMap::new(),
            query_cache: VecDeque::with_capacity(QUERY_CACHE_CAPACITY),
            populated: false,
            stats: CacheStats::default(),
        }
    }

    /// Load all anime from storage and build the title indices.
    pub fn populate(&mut self, storage: &Storage) -> Result<(), RyuujiError> {
        self.entries = storage.all_anime()?;
        self.exact_index.clear();
        self.normalized_index.clear();
        self.query_cache.clear();

        for anime in &self.entries {
            let titles = matcher::all_titles(anime);
            for title in titles {
                self.exact_index
                    .entry(title.to_string())
                    .or_insert(anime.id);
                self.normalized_index
                    .entry(matcher::normalize(title))
                    .or_insert(anime.id);
            }
        }

        self.populated = true;
        self.stats.entries_indexed = self.entries.len();
        Ok(())
    }

    /// Mark the cache as stale. It will rebuild on the next `recognize()` call.
    pub fn invalidate(&mut self) {
        self.populated = false;
        self.entries.clear();
        self.exact_index.clear();
        self.normalized_index.clear();
        self.query_cache.clear();
        self.stats = CacheStats::default();
    }

    /// Return current cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.clone()
    }

    /// Recognize an anime title, using cached indices for fast lookup.
    ///
    /// Flow: query cache → exact index → normalized index → fuzzy scan.
    /// Results are stored in the query cache for subsequent calls.
    #[tracing::instrument(name = "recognize", skip(self, storage), fields(query = %query))]
    pub fn recognize(&mut self, query: &str, storage: &Storage) -> MatchResult {
        if query.is_empty() {
            return MatchResult::NoMatch;
        }

        if !self.populated {
            if let Err(e) = self.populate(storage) {
                tracing::error!(error = %e, "Failed to populate recognition cache");
                return MatchResult::NoMatch;
            }
            tracing::debug!(
                anime_count = self.entries.len(),
                exact_keys = self.exact_index.len(),
                "Recognition cache populated"
            );
        }

        // 1. Check query cache.
        if let Some(cached) = self.query_cache_lookup(query) {
            tracing::debug!(method = "query_cache", "Recognition hit");
            self.stats.hits_lru += 1;
            return cached;
        }

        // 2. Check exact index.
        if let Some(&anime_id) = self.exact_index.get(query) {
            if let Some(anime) = self.find_entry(anime_id) {
                tracing::debug!(method = "exact", matched = %anime.title.preferred(), "Recognition hit");
                let result = MatchResult::Matched(anime);
                self.query_cache_insert(query, &result);
                self.stats.hits_exact += 1;
                self.stats.lru_size = self.query_cache.len();
                return result;
            }
        }

        // 3. Check normalized index.
        let normalized = matcher::normalize(query);
        if let Some(&anime_id) = self.normalized_index.get(&normalized) {
            if let Some(anime) = self.find_entry(anime_id) {
                tracing::debug!(method = "normalized", matched = %anime.title.preferred(), "Recognition hit");
                let result = MatchResult::Matched(anime);
                self.query_cache_insert(query, &result);
                self.stats.hits_normalized += 1;
                self.stats.lru_size = self.query_cache.len();
                return result;
            }
        }

        // 4. Fuzzy fallback over all entries.
        let result = self.fuzzy_scan(&normalized);
        match &result {
            MatchResult::Fuzzy(anime, confidence) => {
                tracing::debug!(
                    method = "fuzzy",
                    matched = %anime.title.preferred(),
                    confidence = format!("{:.1}%", confidence * 100.0),
                    "Recognition hit"
                );
                self.stats.hits_fuzzy += 1;
            }
            MatchResult::NoMatch => {
                tracing::debug!("No recognition match");
                self.stats.misses += 1;
            }
            _ => {}
        }
        self.query_cache_insert(query, &result);
        self.stats.lru_size = self.query_cache.len();
        result
    }

    /// Run fuzzy matching over all cached entries.
    fn fuzzy_scan(&self, normalized_query: &str) -> MatchResult {
        if self.entries.is_empty() {
            return MatchResult::NoMatch;
        }

        let matcher = SkimMatcherV2::default();
        let max_possible = matcher
            .fuzzy_match(normalized_query, normalized_query)
            .unwrap_or(1)
            .max(1);

        let mut best_score: i64 = 0;
        let mut best_anime: Option<&Anime> = None;

        for anime in &self.entries {
            let score = matcher::all_titles(anime)
                .iter()
                .filter_map(|t| matcher.fuzzy_match(&matcher::normalize(t), normalized_query))
                .max()
                .unwrap_or(0);
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

    /// Look up a query in the bounded query cache.
    fn query_cache_lookup(&self, query: &str) -> Option<MatchResult> {
        for (cached_query, cached_match) in &self.query_cache {
            if cached_query == query {
                return Some(self.cached_to_result(cached_match));
            }
        }
        None
    }

    /// Insert a result into the query cache, evicting the oldest if full.
    fn query_cache_insert(&mut self, query: &str, result: &MatchResult) {
        if self.query_cache.len() >= QUERY_CACHE_CAPACITY {
            self.query_cache.pop_front();
        }
        self.query_cache
            .push_back((query.to_string(), result_to_cached(result)));
    }

    /// Convert a cached match back to a full MatchResult by looking up the anime.
    fn cached_to_result(&self, cached: &CachedMatch) -> MatchResult {
        match cached {
            CachedMatch::Matched(id) => match self.find_entry(*id) {
                Some(anime) => MatchResult::Matched(anime),
                None => MatchResult::NoMatch,
            },
            CachedMatch::Fuzzy(id, confidence) => match self.find_entry(*id) {
                Some(anime) => MatchResult::Fuzzy(anime, *confidence),
                None => MatchResult::NoMatch,
            },
            CachedMatch::NoMatch => MatchResult::NoMatch,
        }
    }

    /// Find an anime by ID in the cached entries.
    fn find_entry(&self, anime_id: i64) -> Option<Anime> {
        self.entries.iter().find(|a| a.id == anime_id).cloned()
    }
}

/// Convert a MatchResult into a lightweight CachedMatch.
fn result_to_cached(result: &MatchResult) -> CachedMatch {
    match result {
        MatchResult::Matched(anime) => CachedMatch::Matched(anime.id),
        MatchResult::Fuzzy(anime, confidence) => CachedMatch::Fuzzy(anime.id, *confidence),
        MatchResult::NoMatch => CachedMatch::NoMatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AnimeIds, AnimeTitle};

    fn insert_frieren(storage: &Storage) -> i64 {
        storage
            .insert_anime(&Anime {
                id: 0,
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
            })
            .unwrap()
    }

    fn insert_aot(storage: &Storage) -> i64 {
        storage
            .insert_anime(&Anime {
                id: 0,
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
            })
            .unwrap()
    }

    #[test]
    fn test_exact_cache_hit() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        match cache.recognize("Sousou no Frieren", &storage) {
            MatchResult::Matched(a) => {
                assert_eq!(a.title.romaji.as_deref(), Some("Sousou no Frieren"))
            }
            other => panic!("Expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_normalized_cache_hit() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        // Different case — should still match via normalized index.
        match cache.recognize("sousou no frieren", &storage) {
            MatchResult::Matched(a) => {
                assert_eq!(a.title.romaji.as_deref(), Some("Sousou no Frieren"))
            }
            other => panic!("Expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_fuzzy_fallback() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);
        insert_aot(&storage);

        let mut cache = RecognitionCache::new();
        // Partial match — should fuzzy match.
        match cache.recognize("Frieren Beyond Journeys End", &storage) {
            MatchResult::Fuzzy(a, _) | MatchResult::Matched(a) => {
                assert_eq!(a.title.romaji.as_deref(), Some("Sousou no Frieren"));
            }
            other => panic!("Expected Fuzzy or Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_query_cache() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();

        // First call populates and matches.
        let result1 = cache.recognize("Sousou no Frieren", &storage);
        assert!(matches!(result1, MatchResult::Matched(_)));

        // Second call should hit query cache (still returns correct result).
        let result2 = cache.recognize("Sousou no Frieren", &storage);
        assert!(matches!(result2, MatchResult::Matched(_)));

        // Verify query cache has an entry.
        assert_eq!(cache.query_cache.len(), 1);
    }

    #[test]
    fn test_invalidate() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();

        // Recognize existing anime.
        assert!(matches!(
            cache.recognize("Sousou no Frieren", &storage),
            MatchResult::Matched(_)
        ));

        // Insert new anime and invalidate.
        insert_aot(&storage);
        cache.invalidate();

        // New anime should now be found after re-population.
        match cache.recognize("Attack on Titan", &storage) {
            MatchResult::Matched(a) => {
                assert_eq!(a.title.english.as_deref(), Some("Attack on Titan"));
            }
            other => panic!("Expected Matched, got {other:?}"),
        }
    }

    #[test]
    fn test_query_cache_eviction() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        cache.populate(&storage).unwrap();

        // Fill the cache beyond capacity.
        for i in 0..QUERY_CACHE_CAPACITY + 10 {
            let query = format!("nonexistent anime {i}");
            cache.recognize(&query, &storage);
        }

        // Cache should be bounded.
        assert_eq!(cache.query_cache.len(), QUERY_CACHE_CAPACITY);
    }

    #[test]
    fn test_empty_query() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        assert!(matches!(
            cache.recognize("", &storage),
            MatchResult::NoMatch
        ));
    }

    #[test]
    fn test_cache_stats_exact() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        cache.recognize("Sousou no Frieren", &storage);
        let stats = cache.stats();
        assert_eq!(stats.hits_exact, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_stats_lru() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        // First call: exact hit.
        cache.recognize("Sousou no Frieren", &storage);
        // Second call: LRU cache hit.
        cache.recognize("Sousou no Frieren", &storage);
        let stats = cache.stats();
        assert_eq!(stats.hits_exact, 1);
        assert_eq!(stats.hits_lru, 1);
    }

    #[test]
    fn test_cache_stats_reset_on_invalidate() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        cache.recognize("Sousou no Frieren", &storage);
        assert_eq!(cache.stats().hits_exact, 1);

        cache.invalidate();
        let stats = cache.stats();
        assert_eq!(stats.hits_exact, 0);
        assert_eq!(stats.entries_indexed, 0);
    }

    #[test]
    fn test_cache_stats_miss() {
        let storage = Storage::open_memory().unwrap();
        insert_frieren(&storage);

        let mut cache = RecognitionCache::new();
        cache.recognize("Totally Unknown Anime Title", &storage);
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
    }
}
