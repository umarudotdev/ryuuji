use crate::storage::Storage;
use crate::torrent::filter::TorrentFilter;
use crate::torrent::models::{FilterState, TorrentItem};

/// Apply filters to torrent items by priority order.
///
/// Filters are evaluated in ascending priority order. For each filter,
/// conditions are checked against each item. If matched, the item's
/// `filter_state` is updated. Higher-priority filters override lower ones.
pub fn apply_filters(items: &mut [TorrentItem], filters: &[TorrentFilter]) {
    let mut sorted_filters: Vec<&TorrentFilter> = filters.iter().filter(|f| f.enabled).collect();
    sorted_filters.sort_by_key(|f| f.priority);

    for filter in sorted_filters {
        for item in items.iter_mut() {
            if evaluate_filter(item, filter) {
                item.filter_state = match filter.action {
                    crate::torrent::filter::FilterAction::Discard => FilterState::Discarded,
                    crate::torrent::filter::FilterAction::Select => FilterState::Selected,
                    crate::torrent::filter::FilterAction::Prefer => FilterState::Preferred,
                };
            }
        }
    }
}

/// Evaluate whether a filter matches a torrent item.
fn evaluate_filter(item: &TorrentItem, filter: &TorrentFilter) -> bool {
    let results: Vec<bool> = filter
        .conditions
        .iter()
        .map(|c| evaluate_condition(item, c))
        .collect();

    match filter.match_mode {
        crate::torrent::filter::MatchMode::All => results.iter().all(|&r| r),
        crate::torrent::filter::MatchMode::Any => results.iter().any(|&r| r),
    }
}

/// Evaluate a single condition against a torrent item.
fn evaluate_condition(
    item: &TorrentItem,
    cond: &crate::torrent::filter::FilterCondition,
) -> bool {
    use crate::torrent::filter::{FilterElement, FilterOperator};

    let field_value = match cond.element {
        FilterElement::Title => &item.title,
        FilterElement::Episode => {
            return match cond.operator {
                FilterOperator::GreaterThan => {
                    item.episode.unwrap_or(0) > cond.value.parse().unwrap_or(0)
                }
                FilterOperator::LessThan => {
                    item.episode.unwrap_or(0) < cond.value.parse().unwrap_or(u32::MAX)
                }
                FilterOperator::Equals => {
                    item.episode.map(|e| e.to_string()).as_deref() == Some(&cond.value)
                }
                FilterOperator::NotEquals => {
                    item.episode.map(|e| e.to_string()).as_deref() != Some(&cond.value)
                }
                _ => false,
            };
        }
        FilterElement::ReleaseGroup => {
            if let Some(ref g) = item.release_group {
                g
            } else {
                return false;
            }
        }
        FilterElement::Resolution => {
            if let Some(ref r) = item.resolution {
                r
            } else {
                return false;
            }
        }
        FilterElement::Size => {
            if let Some(ref s) = item.size {
                s
            } else {
                return false;
            }
        }
    };

    let value_lower = cond.value.to_lowercase();
    let field_lower = field_value.to_lowercase();

    match cond.operator {
        FilterOperator::Equals => field_lower == value_lower,
        FilterOperator::NotEquals => field_lower != value_lower,
        FilterOperator::Contains => field_lower.contains(&value_lower),
        FilterOperator::BeginsWith => field_lower.starts_with(&value_lower),
        FilterOperator::EndsWith => field_lower.ends_with(&value_lower),
        FilterOperator::GreaterThan => field_lower > value_lower,
        FilterOperator::LessThan => field_lower < value_lower,
    }
}

/// Remove items whose GUID is already in the archive.
pub fn filter_archived(items: &mut Vec<TorrentItem>, storage: &Storage) {
    items.retain(|item| !storage.is_torrent_archived(&item.guid).unwrap_or(false));
}
