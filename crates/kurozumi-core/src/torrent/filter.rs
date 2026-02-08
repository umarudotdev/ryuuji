use serde::{Deserialize, Serialize};

/// A smart filter rule for torrent items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentFilter {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub match_mode: MatchMode,
    pub action: FilterAction,
    pub conditions: Vec<FilterCondition>,
}

/// How multiple conditions in a filter are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchMode {
    All,
    Any,
}

impl std::fmt::Display for MatchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "All"),
            Self::Any => write!(f, "Any"),
        }
    }
}

/// What action a filter takes when it matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterAction {
    Discard,
    Select,
    Prefer,
}

impl std::fmt::Display for FilterAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discard => write!(f, "Discard"),
            Self::Select => write!(f, "Select"),
            Self::Prefer => write!(f, "Prefer"),
        }
    }
}

/// A single condition within a filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub element: FilterElement,
    pub operator: FilterOperator,
    pub value: String,
}

/// Which field of a torrent item to match against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterElement {
    Title,
    Episode,
    ReleaseGroup,
    Resolution,
    Size,
}

impl std::fmt::Display for FilterElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Title => write!(f, "Title"),
            Self::Episode => write!(f, "Episode"),
            Self::ReleaseGroup => write!(f, "Group"),
            Self::Resolution => write!(f, "Resolution"),
            Self::Size => write!(f, "Size"),
        }
    }
}

/// Comparison operator for a filter condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    BeginsWith,
    EndsWith,
    GreaterThan,
    LessThan,
}

impl std::fmt::Display for FilterOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equals => write!(f, "equals"),
            Self::NotEquals => write!(f, "not equals"),
            Self::Contains => write!(f, "contains"),
            Self::BeginsWith => write!(f, "begins with"),
            Self::EndsWith => write!(f, "ends with"),
            Self::GreaterThan => write!(f, ">"),
            Self::LessThan => write!(f, "<"),
        }
    }
}
