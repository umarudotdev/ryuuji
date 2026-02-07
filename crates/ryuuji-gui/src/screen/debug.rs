//! Debug dashboard screen.
//!
//! Displays the detection pipeline state: current detection status,
//! last parse/match results, cache statistics, and a scrollable
//! event history log.

use iced::widget::{column, container, row, scrollable, text, toggler, Space};
use iced::{Element, Length};

use ryuuji_core::debug_log::{CacheStats, DebugEvent, EventEntry, SharedEventLog};

use crate::db::DbHandle;
use crate::screen::Action;
use crate::style;
use crate::theme::{self, ColorScheme};

/// Debug screen state.
pub struct Debug {
    /// Snapshot of the event log (newest last).
    events: Vec<EventEntry>,
    /// Cache statistics from the recognition cache.
    cache_stats: Option<CacheStats>,
    /// Whether verbose mode is on (expands all event fields).
    verbose: bool,
}

/// Messages handled by the Debug screen.
#[derive(Debug, Clone)]
pub enum Message {
    /// New event log snapshot received.
    #[allow(dead_code)]
    EventsRefreshed(Vec<EventEntry>),
    /// Cache stats loaded from DB actor.
    CacheStatsLoaded(CacheStats),
    /// Toggle verbose display.
    ToggleVerbose(bool),
}

impl Debug {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            cache_stats: None,
            verbose: false,
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::EventsRefreshed(events) => {
                self.events = events;
                Action::None
            }
            Message::CacheStatsLoaded(stats) => {
                self.cache_stats = Some(stats);
                Action::None
            }
            Message::ToggleVerbose(on) => {
                self.verbose = on;
                Action::None
            }
        }
    }

    /// Refresh the debug screen from the shared event log and request cache stats.
    ///
    /// `wrap` maps a `debug::Message` into the top-level app message so
    /// callers can embed this in whichever screen owns the debug panel.
    pub fn refresh<F>(
        &mut self,
        event_log: &SharedEventLog,
        db: Option<&DbHandle>,
        wrap: F,
    ) -> Action
    where
        F: Fn(Message) -> crate::app::Message + Send + 'static,
    {
        // Snapshot the event log (brief lock).
        let snapshot = event_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot();
        self.events = snapshot;

        // Request cache stats from the DB actor.
        if let Some(db) = db {
            let db = db.clone();
            return Action::RunTask(iced::Task::perform(
                async move { db.get_cache_stats().await },
                move |stats| wrap(Message::CacheStatsLoaded(stats)),
            ));
        }
        Action::None
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let header_row = row![
            Space::new().width(Length::Fill),
            text("Verbose")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
            toggler(self.verbose)
                .on_toggle(Message::ToggleVerbose)
                .size(16.0),
        ]
        .spacing(style::SPACE_SM)
        .align_y(iced::Alignment::Center);

        // Current State section
        let state_section = self.current_state_view(cs);

        // Event History section
        let history_section = self.event_history_view(cs);

        let page = column![header_row, state_section, history_section]
            .spacing(style::SPACE_LG)
            .padding(style::SPACE_XL)
            .width(Length::Fill)
            .height(Length::Fill);

        page.into()
    }

    /// Render the "current state" snapshot panel.
    fn current_state_view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let last_detection = self
            .events
            .iter()
            .rev()
            .find_map(|(_, e)| match e {
                DebugEvent::PlayerDetected {
                    player_name,
                    media_title,
                    ..
                } => Some(format!(
                    "{} \u{2014} {}",
                    player_name,
                    media_title.as_deref().unwrap_or("(no title)")
                )),
                _ => None,
            })
            .unwrap_or_else(|| "Nothing detected".into());

        let last_parse = self
            .events
            .iter()
            .rev()
            .find_map(|(_, e)| match e {
                DebugEvent::Parsed {
                    raw_title,
                    title,
                    episode,
                    group,
                    ..
                } => {
                    let title_str = title.as_deref().unwrap_or("?");
                    let ep_str = episode.map(|e| format!(" ep {e}")).unwrap_or_default();
                    let group_str = group
                        .as_ref()
                        .map(|g| format!(" [{g}]"))
                        .unwrap_or_default();
                    Some(format!("{raw_title} -> {title_str}{ep_str}{group_str}"))
                }
                _ => None,
            })
            .unwrap_or_else(|| "\u{2014}".into());

        let last_match = self
            .events
            .iter()
            .rev()
            .find_map(|(_, e)| match e {
                DebugEvent::RecognitionResult {
                    query,
                    match_level,
                    anime_title,
                } => {
                    let level_str = format!("{match_level:?}");
                    let title_str = anime_title.as_deref().unwrap_or("\u{2014}");
                    Some(format!("\"{query}\" -> {title_str} ({level_str})"))
                }
                _ => None,
            })
            .unwrap_or_else(|| "\u{2014}".into());

        let cache_line = if let Some(ref stats) = self.cache_stats {
            format!(
                "Indexed: {} | LRU: {} | Hits: exact {} / norm {} / fuzzy {} / lru {} | Miss: {}",
                stats.entries_indexed,
                stats.lru_size,
                stats.hits_exact,
                stats.hits_normalized,
                stats.hits_fuzzy,
                stats.hits_lru,
                stats.misses,
            )
        } else {
            "Cache stats loading...".into()
        };

        let card = column![
            self.label_value(cs, "Detection", &last_detection),
            self.label_value(cs, "Last parse", &last_parse),
            self.label_value(cs, "Last match", &last_match),
            self.label_value(cs, "Cache", &cache_line),
        ]
        .spacing(style::SPACE_SM)
        .padding(style::SPACE_LG)
        .width(Length::Fill);

        container(card)
            .style(theme::card(cs))
            .width(Length::Fill)
            .into()
    }

    /// Render a label: value pair.
    fn label_value<'a>(&self, cs: &ColorScheme, label: &str, value: &str) -> Element<'a, Message> {
        row![
            text(format!("{label}:"))
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .width(Length::Fixed(80.0)),
            text(value.to_string())
                .size(style::TEXT_SM)
                .color(cs.on_surface),
        ]
        .spacing(style::SPACE_SM)
        .into()
    }

    /// Render the scrollable event history.
    fn event_history_view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let mut event_column = column![].spacing(style::SPACE_XXS);

        // Reverse chronological — newest first.
        for (timestamp, event) in self.events.iter().rev() {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            let (summary, color) = self.event_summary(cs, event);

            let row_content = if self.verbose {
                let detail = format!("{event:?}");
                column![
                    row![
                        text(time_str)
                            .size(style::TEXT_XS)
                            .color(cs.on_surface_variant)
                            .width(Length::Fixed(64.0)),
                        text(summary).size(style::TEXT_SM).color(color),
                    ]
                    .spacing(style::SPACE_SM),
                    text(detail)
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant),
                ]
                .spacing(style::SPACE_XXS)
            } else {
                column![row![
                    text(time_str)
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(64.0)),
                    text(summary).size(style::TEXT_SM).color(color),
                ]
                .spacing(style::SPACE_SM)]
            };

            event_column = event_column.push(row_content);
        }

        let history = if self.events.is_empty() {
            column![
                text("No events yet. Detection events will appear here as the pipeline runs.")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
            ]
        } else {
            event_column
        };

        scrollable(
            container(history)
                .padding(style::SPACE_LG)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    }

    /// Return a one-line summary and color for an event.
    fn event_summary(&self, cs: &ColorScheme, event: &DebugEvent) -> (String, iced::Color) {
        match event {
            DebugEvent::DetectionTick { players_found } => (
                format!("Tick \u{2014} {players_found} player(s)"),
                cs.on_surface_variant,
            ),
            DebugEvent::PlayerDetected {
                player_name,
                is_browser,
                ..
            } => {
                let kind = if *is_browser { "browser" } else { "player" };
                (format!("Detected {kind}: {player_name}"), cs.on_surface)
            }
            DebugEvent::StreamMatched {
                service_name,
                extracted_title,
            } => (
                format!("Stream: {service_name} \u{2014} \"{extracted_title}\""),
                cs.primary,
            ),
            DebugEvent::StreamNotMatched { player_name } => (
                format!("No stream match for {player_name}"),
                cs.on_surface_variant,
            ),
            DebugEvent::Parsed { title, episode, .. } => {
                let t = title.as_deref().unwrap_or("?");
                let ep = episode.map(|e| format!(" ep {e}")).unwrap_or_default();
                (format!("Parsed: {t}{ep}"), cs.on_surface)
            }
            DebugEvent::RecognitionResult {
                match_level,
                anime_title,
                ..
            } => {
                let title = anime_title.as_deref().unwrap_or("\u{2014}");
                match match_level {
                    ryuuji_core::debug_log::MatchLevel::NoMatch => ("No match".into(), cs.error),
                    ryuuji_core::debug_log::MatchLevel::Fuzzy(score) => (
                        format!("Fuzzy: {title} ({:.0}%)", score * 100.0),
                        cs.tertiary,
                    ),
                    level => (format!("{level:?}: {title}"), cs.primary),
                }
            }
            DebugEvent::EpisodeRedirect {
                from_title,
                from_ep,
                to_title,
                to_ep,
            } => (
                format!("Redirect: {from_title} ep {from_ep} -> {to_title} ep {to_ep}"),
                cs.on_surface,
            ),
            DebugEvent::LibraryUpdate {
                anime_title,
                episode,
                outcome,
            } => {
                let verb = match outcome {
                    ryuuji_core::debug_log::UpdateKind::Updated => "Updated",
                    ryuuji_core::debug_log::UpdateKind::AlreadyCurrent => "Current",
                    ryuuji_core::debug_log::UpdateKind::Added => "Added",
                };
                (format!("{verb}: {anime_title} ep {episode}"), cs.primary)
            }
            DebugEvent::Unrecognized { raw_title } => {
                (format!("Unrecognized: \"{raw_title}\""), cs.tertiary)
            }
            DebugEvent::Error { source, message } => {
                (format!("Error ({source}): {message}"), cs.error)
            }
        }
    }
}
