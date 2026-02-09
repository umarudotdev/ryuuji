use crate::matcher::MatchResult;
use crate::recognition::RecognitionCache;
use crate::storage::Storage;
use crate::torrent::models::TorrentItem;

/// Parse torrent titles and match them against the library.
///
/// For each item, uses `ryuuji_parse` to extract episode/group/resolution
/// from the raw title, then runs the recognition cache to match against
/// known anime in the database.
pub fn match_torrent_items(
    items: &mut [TorrentItem],
    storage: &Storage,
    cache: &mut RecognitionCache,
) {
    for item in items.iter_mut() {
        let parsed = ryuuji_parse::parse(&item.title);

        item.episode = parsed.episode_number;
        item.release_group = parsed.release_group;
        item.resolution = parsed.resolution;

        if let Some(ref anime_title) = parsed.title {
            match cache.recognize(anime_title, storage) {
                MatchResult::Matched(anime) | MatchResult::Fuzzy(anime, _) => {
                    item.anime_id = Some(anime.id);
                    item.anime_title = Some(anime.title.preferred().to_string());
                }
                MatchResult::NoMatch => {}
            }
        }
    }
}
