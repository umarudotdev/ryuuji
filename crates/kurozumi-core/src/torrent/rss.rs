use chrono::{DateTime, Utc};

use crate::error::KurozumiError;
use crate::torrent::models::{FilterState, TorrentFeed, TorrentItem};

/// Fetch and parse a single RSS feed into torrent items.
pub async fn fetch_feed(
    client: &reqwest::Client,
    feed: &TorrentFeed,
) -> Result<Vec<TorrentItem>, KurozumiError> {
    let response = client
        .get(&feed.url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| KurozumiError::Torrent(format!("fetch {}: {e}", feed.name)))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| KurozumiError::Torrent(format!("read {}: {e}", feed.name)))?;

    let channel = rss::Channel::read_from(&bytes[..])
        .map_err(|e| KurozumiError::Torrent(format!("parse {}: {e}", feed.name)))?;

    let items: Vec<TorrentItem> = channel
        .items()
        .iter()
        .map(|rss_item: &rss::Item| {
            let guid = rss_item
                .guid()
                .map(|g| g.value().to_string())
                .or_else(|| rss_item.link().map(|l| l.to_string()))
                .unwrap_or_default();

            let title = rss_item.title().unwrap_or("").to_string();

            // Nyaa uses nyaa:seeders, nyaa:leechers, nyaa:downloads, nyaa:size
            // in the RSS extensions namespace.
            let nyaa = rss_item.extensions().get("nyaa");
            let nyaa_field = |name: &str| -> Option<String> {
                nyaa?.get(name)?.first()?.value().map(|s| s.to_string())
            };

            let seeders: Option<u32> = nyaa_field("seeders").and_then(|s| s.parse().ok());
            let leechers: Option<u32> = nyaa_field("leechers").and_then(|s| s.parse().ok());
            let downloads: Option<u32> = nyaa_field("downloads").and_then(|s| s.parse().ok());
            let size: Option<String> = nyaa_field("size");

            let pub_date: Option<DateTime<Utc>> = rss_item
                .pub_date()
                .and_then(|s| DateTime::parse_from_rfc2822(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            // Nyaa puts magnet links in the link field (the .torrent URL is
            // typically the guid or not present).
            let link_str: Option<String> = rss_item.link().map(|s| s.to_string());
            let (link, magnet_link) = if link_str
                .as_deref()
                .is_some_and(|l| l.starts_with("magnet:"))
            {
                (None, link_str)
            } else {
                (link_str, None)
            };

            TorrentItem {
                guid,
                title,
                link,
                magnet_link,
                description: rss_item.description().map(|s| s.to_string()),
                size,
                seeders,
                leechers,
                downloads,
                pub_date,
                info_link: None,
                anime_id: None,
                anime_title: None,
                episode: None,
                release_group: None,
                resolution: None,
                filter_state: FilterState::None,
            }
        })
        .collect();

    Ok(items)
}

/// Fetch all enabled feeds and return items keyed by feed ID.
pub async fn fetch_all_feeds(
    client: &reqwest::Client,
    feeds: &[TorrentFeed],
) -> Vec<(i64, Result<Vec<TorrentItem>, KurozumiError>)> {
    let mut results = Vec::new();
    for feed in feeds.iter().filter(|f| f.enabled) {
        let result = fetch_feed(client, feed).await;
        results.push((feed.id, result));
    }
    results
}
