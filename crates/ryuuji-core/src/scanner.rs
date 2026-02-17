//! Watch folder scanner.
//!
//! Walks user-configured directories, parses video filenames, matches them
//! against the library via the recognition cache, and stores available
//! episode records in the database.

use std::path::Path;

use tracing::warn;
use walkdir::WalkDir;

use crate::config::LibraryConfig;
use crate::error::RyuujiError;
use crate::matcher::MatchResult;
use crate::models::AvailableEpisode;
use crate::recognition::RecognitionCache;
use crate::storage::Storage;

/// Video file extensions to consider.
const VIDEO_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "ogm", "wmv", "webm", "flv", "m4v"];

/// Result of a folder scan operation.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub files_scanned: u32,
    pub files_matched: u32,
    pub files_skipped: u32,
}

/// Scan all configured watch folders and index available episodes.
///
/// For each video file found:
/// 1. Check size threshold (skip tiny files / samples)
/// 2. Check if already indexed with same size + mtime (incremental skip)
/// 3. Parse filename via `ryuuji_parse::parse()`
/// 4. Match title via `RecognitionCache::recognize()`
/// 5. Upsert `available_episode` record
pub fn scan_watch_folders(
    storage: &Storage,
    cache: &mut RecognitionCache,
    config: &LibraryConfig,
) -> Result<ScanResult, RyuujiError> {
    let mut result = ScanResult::default();
    let min_bytes = config.min_file_size_mb * 1024 * 1024;

    for folder in &config.watch_folders {
        let folder_path = Path::new(folder);
        if !folder_path.is_dir() {
            tracing::warn!(path = %folder, "Watch folder does not exist, skipping");
            continue;
        }

        tracing::info!(path = %folder, "Scanning watch folder");

        for entry in WalkDir::new(folder_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // Check video extension
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());
            let is_video = ext
                .as_deref()
                .map(|e| VIDEO_EXTENSIONS.contains(&e))
                .unwrap_or(false);
            if !is_video {
                continue;
            }

            result.files_scanned += 1;

            // Check file size
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    warn!(path = %entry.path().display(), error = %e, "Failed to read file metadata");
                    result.files_skipped += 1;
                    continue;
                }
            };
            let file_size = metadata.len();
            if file_size < min_bytes {
                result.files_skipped += 1;
                continue;
            }

            let file_modified = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    Some(dt.to_rfc3339())
                })
                .unwrap_or_default();

            let file_path_str = path.to_string_lossy().to_string();

            // Incremental: skip if already indexed with same size + mtime
            if storage.is_file_indexed(&file_path_str, file_size, &file_modified)? {
                result.files_skipped += 1;
                continue;
            }

            // Parse filename
            let file_stem = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            let parsed = ryuuji_parse::parse(file_stem);
            let title = parsed.title.as_deref().unwrap_or_default();
            if title.is_empty() {
                result.files_skipped += 1;
                continue;
            }

            // Match against library
            let match_result = cache.recognize(title, storage);
            let anime_id = match &match_result {
                MatchResult::Matched(anime) | MatchResult::Fuzzy(anime, _) => anime.id,
                MatchResult::NoMatch => {
                    result.files_skipped += 1;
                    continue;
                }
            };

            let episode = parsed.episode_number.unwrap_or(1);

            let ep = AvailableEpisode {
                id: 0,
                anime_id,
                episode,
                file_path: file_path_str,
                file_size,
                file_modified,
                release_group: parsed.release_group.clone(),
                resolution: parsed.resolution.clone(),
            };

            if let Err(e) = storage.upsert_available_episode(&ep) {
                tracing::warn!(error = %e, "Failed to upsert available episode");
                continue;
            }

            result.files_matched += 1;
        }
    }

    tracing::info!(
        scanned = result.files_scanned,
        matched = result.files_matched,
        skipped = result.files_skipped,
        "Watch folder scan complete"
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Anime, AnimeIds, AnimeTitle};
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_db() -> (Storage, TempDir) {
        let storage = Storage::open_memory().unwrap();
        let dir = TempDir::new().unwrap();

        // Insert a known anime
        storage
            .insert_anime(&Anime {
                id: 0,
                ids: AnimeIds::default(),
                title: AnimeTitle {
                    romaji: Some("Sousou no Frieren".into()),
                    english: Some("Frieren: Beyond Journey's End".into()),
                    native: None,
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
            .unwrap();

        (storage, dir)
    }

    fn create_video_file(dir: &Path, name: &str, size_mb: u64) {
        let path = dir.join(name);
        let mut file = std::fs::File::create(path).unwrap();
        // Write enough bytes to exceed minimum size
        let bytes = vec![0u8; (size_mb * 1024 * 1024) as usize];
        file.write_all(&bytes).unwrap();
    }

    #[test]
    fn test_scan_matches_known_anime() {
        let (storage, dir) = setup_test_db();
        create_video_file(
            dir.path(),
            "[SubGroup] Sousou no Frieren - 05 (1080p).mkv",
            11,
        );

        let config = LibraryConfig {
            auto_update: true,
            confirm_update: false,
            watch_folders: vec![dir.path().to_string_lossy().to_string()],
            min_file_size_mb: 10,
            scan_on_startup: false,
        };

        let mut cache = RecognitionCache::new();
        let result = scan_watch_folders(&storage, &mut cache, &config).unwrap();

        assert_eq!(result.files_scanned, 1);
        assert_eq!(result.files_matched, 1);

        let summaries = storage.get_available_episode_summaries().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].count, 1);
    }

    #[test]
    fn test_scan_skips_small_files() {
        let (storage, dir) = setup_test_db();
        create_video_file(
            dir.path(),
            "[SubGroup] Sousou no Frieren - 05 (1080p).mkv",
            5, // Below 10MB threshold
        );

        let config = LibraryConfig {
            auto_update: true,
            confirm_update: false,
            watch_folders: vec![dir.path().to_string_lossy().to_string()],
            min_file_size_mb: 10,
            scan_on_startup: false,
        };

        let mut cache = RecognitionCache::new();
        let result = scan_watch_folders(&storage, &mut cache, &config).unwrap();

        assert_eq!(result.files_scanned, 1);
        assert_eq!(result.files_skipped, 1);
        assert_eq!(result.files_matched, 0);
    }

    #[test]
    fn test_scan_skips_non_video() {
        let (storage, dir) = setup_test_db();
        // Create a non-video file
        std::fs::write(dir.path().join("readme.txt"), "hello").unwrap();

        let config = LibraryConfig {
            auto_update: true,
            confirm_update: false,
            watch_folders: vec![dir.path().to_string_lossy().to_string()],
            min_file_size_mb: 10,
            scan_on_startup: false,
        };

        let mut cache = RecognitionCache::new();
        let result = scan_watch_folders(&storage, &mut cache, &config).unwrap();

        // Non-video files are filtered before counting
        assert_eq!(result.files_scanned, 0);
    }

    #[test]
    fn test_incremental_scan_skips_indexed() {
        let (storage, dir) = setup_test_db();
        create_video_file(
            dir.path(),
            "[SubGroup] Sousou no Frieren - 05 (1080p).mkv",
            11,
        );

        let config = LibraryConfig {
            auto_update: true,
            confirm_update: false,
            watch_folders: vec![dir.path().to_string_lossy().to_string()],
            min_file_size_mb: 10,
            scan_on_startup: false,
        };

        let mut cache = RecognitionCache::new();

        // First scan
        let result1 = scan_watch_folders(&storage, &mut cache, &config).unwrap();
        assert_eq!(result1.files_matched, 1);

        // Second scan — file unchanged, should skip
        let result2 = scan_watch_folders(&storage, &mut cache, &config).unwrap();
        assert_eq!(result2.files_skipped, 1);
        assert_eq!(result2.files_matched, 0);
    }
}
