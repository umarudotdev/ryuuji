use std::collections::HashMap;

use crate::error::KurozumiError;

/// Embedded anime-relations data from erengy/anime-relations.
const EMBEDDED_DATA: &str = include_str!("../data/anime-relations.txt");

/// An episode range (inclusive on both ends).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeRange {
    pub start: u32,
    pub end: u32,
}

impl EpisodeRange {
    pub fn contains(&self, episode: u32) -> bool {
        episode >= self.start && episode <= self.end
    }

    /// Compute the offset of an episode within this range.
    pub fn offset(&self, episode: u32) -> u32 {
        episode - self.start
    }
}

/// A single relation rule mapping source episodes to destination episodes.
#[derive(Debug, Clone)]
pub struct RelationRule {
    pub source_mal: Option<u64>,
    pub source_kitsu: Option<u64>,
    pub source_anilist: Option<u64>,
    pub source_episodes: EpisodeRange,
    pub dest_mal: Option<u64>,
    pub dest_kitsu: Option<u64>,
    pub dest_anilist: Option<u64>,
    pub dest_episodes: EpisodeRange,
}

/// Result of an episode redirect lookup.
#[derive(Debug, Clone)]
pub struct EpisodeRedirect {
    pub dest_mal: Option<u64>,
    pub dest_kitsu: Option<u64>,
    pub dest_anilist: Option<u64>,
    pub dest_episode: u32,
}

/// Database of anime episode relation rules.
#[derive(Debug, Clone)]
pub struct RelationDatabase {
    /// Rules indexed by MAL ID for fast lookup.
    pub by_mal: HashMap<u64, Vec<RelationRule>>,
}

impl RelationDatabase {
    /// Create an empty database.
    pub fn new() -> Self {
        Self {
            by_mal: HashMap::new(),
        }
    }

    /// Load the embedded anime-relations.txt data.
    pub fn embedded() -> Result<Self, KurozumiError> {
        Self::parse(EMBEDDED_DATA)
    }

    /// Parse anime-relations.txt format into a database.
    pub fn parse(data: &str) -> Result<Self, KurozumiError> {
        let mut db = Self::new();
        let mut in_rules = false;

        for line in data.lines() {
            let line = line.trim();

            // Section headers.
            if line == "::rules" {
                in_rules = true;
                continue;
            }
            if line.starts_with("::") {
                in_rules = false;
                continue;
            }

            // Skip comments, blank lines, and metadata.
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("- version:")
                || line.starts_with("- last_modified:")
            {
                continue;
            }

            if !in_rules {
                continue;
            }

            // Rule lines start with "- ".
            let rule_text = match line.strip_prefix("- ") {
                Some(t) => t,
                None => continue,
            };

            // Parse rules. May produce multiple rules if bidirectional (!).
            let rules = parse_rule_line(rule_text)?;
            for rule in rules {
                if let Some(mal_id) = rule.source_mal {
                    db.by_mal.entry(mal_id).or_default().push(rule);
                }
            }
        }

        Ok(db)
    }

    /// Look up an episode redirect by MAL ID and episode number.
    /// Returns the redirect target if a matching rule exists.
    pub fn redirect_mal(&self, mal_id: u64, episode: u32) -> Option<EpisodeRedirect> {
        let rules = self.by_mal.get(&mal_id)?;
        for rule in rules {
            if rule.source_episodes.contains(episode) {
                let offset = rule.source_episodes.offset(episode);
                let dest_episode = rule.dest_episodes.start + offset;
                return Some(EpisodeRedirect {
                    dest_mal: rule.dest_mal,
                    dest_kitsu: rule.dest_kitsu,
                    dest_anilist: rule.dest_anilist,
                    dest_episode,
                });
            }
        }
        None
    }
}

impl Default for RelationDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a single rule line into one or two rules (if bidirectional).
///
/// Format: `MAL|Kitsu|AniList:episodes -> MAL|Kitsu|AniList:episodes[!]`
fn parse_rule_line(line: &str) -> Result<Vec<RelationRule>, KurozumiError> {
    let err = |msg: &str| KurozumiError::Relation(format!("{msg}: {line}"));

    // Check for bidirectional flag.
    let (line, bidirectional) = if let Some(stripped) = line.strip_suffix('!') {
        (stripped, true)
    } else {
        (line, false)
    };

    // Split on " -> ".
    let parts: Vec<&str> = line.splitn(2, " -> ").collect();
    if parts.len() != 2 {
        return Err(err("missing ' -> '"));
    }

    let (source_ids, source_eps) = parse_side(parts[0]).map_err(|e| err(&e))?;
    let (dest_ids_raw, dest_eps) = parse_side(parts[1]).map_err(|e| err(&e))?;

    // Resolve "~" (same as source) in destination IDs.
    let dest_ids = resolve_tilde(&source_ids, &dest_ids_raw);

    let rule = RelationRule {
        source_mal: source_ids[0],
        source_kitsu: source_ids[1],
        source_anilist: source_ids[2],
        source_episodes: source_eps.clone(),
        dest_mal: dest_ids[0],
        dest_kitsu: dest_ids[1],
        dest_anilist: dest_ids[2],
        dest_episodes: dest_eps.clone(),
    };

    let mut rules = vec![rule];

    // Bidirectional: create reverse rule (destination redirects to itself).
    if bidirectional {
        let reverse = RelationRule {
            source_mal: dest_ids[0],
            source_kitsu: dest_ids[1],
            source_anilist: dest_ids[2],
            source_episodes: dest_eps,
            dest_mal: dest_ids[0],
            dest_kitsu: dest_ids[1],
            dest_anilist: dest_ids[2],
            dest_episodes: source_eps,
        };
        rules.push(reverse);
    }

    Ok(rules)
}

/// Parse one side of a rule: `MAL|Kitsu|AniList:episodes`.
fn parse_side(s: &str) -> Result<([Option<u64>; 3], EpisodeRange), String> {
    let s = s.trim();
    let colon_pos = s
        .rfind(':')
        .ok_or_else(|| format!("missing ':' in '{s}'"))?;

    let ids_part = &s[..colon_pos];
    let eps_part = &s[colon_pos + 1..];

    let ids = parse_ids(ids_part)?;
    let eps = parse_episode_range(eps_part)?;

    Ok((ids, eps))
}

/// Parse `MAL|Kitsu|AniList` into three optional IDs.
fn parse_ids(s: &str) -> Result<[Option<u64>; 3], String> {
    let parts: Vec<&str> = s.split('|').collect();
    if parts.len() != 3 {
        return Err(format!(
            "expected 3 pipe-separated IDs, got {}: '{s}'",
            parts.len()
        ));
    }

    let mut ids = [None; 3];
    for (i, part) in parts.iter().enumerate() {
        let part = part.trim();
        if part == "?" || part == "~" {
            // "?" = unknown, "~" = placeholder for tilde resolution.
            if part == "~" {
                // Store a sentinel value that will be resolved later.
                ids[i] = Some(u64::MAX);
            }
            // "?" → None
        } else {
            ids[i] = Some(
                part.parse::<u64>()
                    .map_err(|_| format!("invalid ID '{part}'"))?,
            );
        }
    }

    Ok(ids)
}

/// Resolve tilde ("~") markers in destination IDs with source values.
fn resolve_tilde(source: &[Option<u64>; 3], dest: &[Option<u64>; 3]) -> [Option<u64>; 3] {
    let mut resolved = *dest;
    for i in 0..3 {
        if resolved[i] == Some(u64::MAX) {
            resolved[i] = source[i];
        }
    }
    resolved
}

/// Parse an episode range like "1-12", single episode "13", or open-ended "14-?".
/// "?" means unknown, represented as `u32::MAX` (matches any episode >= start).
fn parse_episode_range(s: &str) -> Result<EpisodeRange, String> {
    let s = s.trim();
    if s == "?" {
        return Ok(EpisodeRange {
            start: 0,
            end: u32::MAX,
        });
    }
    if let Some(dash_pos) = s.find('-') {
        let start_str = s[..dash_pos].trim();
        let end_str = s[dash_pos + 1..].trim();
        let start: u32 = start_str
            .parse()
            .map_err(|_| format!("invalid start episode in '{s}'"))?;
        let end: u32 = if end_str == "?" {
            u32::MAX
        } else {
            end_str
                .parse()
                .map_err(|_| format!("invalid end episode in '{s}'"))?
        };
        Ok(EpisodeRange { start, end })
    } else {
        let ep: u32 = s
            .parse()
            .map_err(|_| format!("invalid episode number '{s}'"))?;
        Ok(EpisodeRange { start: ep, end: ep })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let rules = parse_rule_line("41380|43367|116242:13-24 -> 44881|43883|127366:1-12").unwrap();
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert_eq!(r.source_mal, Some(41380));
        assert_eq!(r.source_kitsu, Some(43367));
        assert_eq!(r.source_anilist, Some(116242));
        assert_eq!(r.source_episodes, EpisodeRange { start: 13, end: 24 });
        assert_eq!(r.dest_mal, Some(44881));
        assert_eq!(r.dest_episodes, EpisodeRange { start: 1, end: 12 });
    }

    #[test]
    fn test_parse_bidirectional() {
        let rules =
            parse_rule_line("41380|43367|116242:13-24 -> 44881|43883|127366:1-12!").unwrap();
        assert_eq!(rules.len(), 2);
        // Forward rule
        assert_eq!(rules[0].source_mal, Some(41380));
        assert_eq!(rules[0].dest_mal, Some(44881));
        // Reverse rule (destination redirects to itself)
        assert_eq!(rules[1].source_mal, Some(44881));
        assert_eq!(rules[1].dest_mal, Some(44881));
    }

    #[test]
    fn test_parse_tilde_resolution() {
        // "~" means same as source
        let rules = parse_rule_line("10001|10002|10003:13 -> ~|~|~:1").unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].dest_mal, Some(10001));
        assert_eq!(rules[0].dest_kitsu, Some(10002));
        assert_eq!(rules[0].dest_anilist, Some(10003));
    }

    #[test]
    fn test_parse_unknown_ids() {
        let rules = parse_rule_line("10001|?|10003:1-12 -> 20001|?|20003:1-12").unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].source_kitsu, None);
        assert_eq!(rules[0].dest_kitsu, None);
    }

    #[test]
    fn test_redirect_lookup() {
        let db = RelationDatabase::parse(
            "::rules\n- 41380|43367|116242:13-24 -> 44881|43883|127366:1-12",
        )
        .unwrap();

        // Episode 13 → 1
        let redirect = db.redirect_mal(41380, 13).unwrap();
        assert_eq!(redirect.dest_mal, Some(44881));
        assert_eq!(redirect.dest_episode, 1);

        // Episode 24 → 12
        let redirect = db.redirect_mal(41380, 24).unwrap();
        assert_eq!(redirect.dest_episode, 12);

        // Episode 12 → no match (before range)
        assert!(db.redirect_mal(41380, 12).is_none());

        // Unknown MAL ID → no match
        assert!(db.redirect_mal(99999, 13).is_none());
    }

    #[test]
    fn test_single_episode_rule() {
        let db =
            RelationDatabase::parse("::rules\n- 6682|4662|6682:13 -> 7739|5102|7739:1").unwrap();

        let redirect = db.redirect_mal(6682, 13).unwrap();
        assert_eq!(redirect.dest_mal, Some(7739));
        assert_eq!(redirect.dest_episode, 1);

        assert!(db.redirect_mal(6682, 12).is_none());
    }

    #[test]
    fn test_embedded_parses_without_error() {
        let db = RelationDatabase::embedded().unwrap();
        // The embedded file should have hundreds of rules.
        let total_rules: usize = db.by_mal.values().map(|v| v.len()).sum();
        assert!(total_rules > 400, "Expected 400+ rules, got {total_rules}");
    }
}
