//! Statistics dashboard screen.
//!
//! Displays comprehensive library statistics: entry counts by status,
//! episodes watched, watch time, score distribution, and top genres.

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Element, Length};

use ryuuji_core::models::WatchStatus;
use ryuuji_core::storage::LibraryStatistics;

use crate::app;
use crate::db::DbHandle;
use crate::screen::Action;
use crate::style;
use crate::theme::{self, ColorScheme};

/// Statistics screen state.
pub struct Stats {
    pub statistics: Option<LibraryStatistics>,
    pub loading: bool,
}

/// Messages handled by the Stats screen.
#[derive(Debug, Clone)]
pub enum Message {
    StatsLoaded(Result<LibraryStatistics, String>),
}

impl Stats {
    pub fn new() -> Self {
        Self {
            statistics: None,
            loading: false,
        }
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::StatsLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(stats) => self.statistics = Some(stats),
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to load statistics");
                        self.statistics = None;
                    }
                }
                Action::None
            }
        }
    }

    /// Kick off an async stats load.
    pub fn load_stats(&mut self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        self.loading = true;
        let db = db.clone();
        Action::RunTask(iced::Task::perform(
            async move { db.get_library_statistics().await.map_err(|e| e.to_string()) },
            |result| app::Message::Stats(Message::StatsLoaded(result)),
        ))
    }

    pub fn view<'a>(&'a self, cs: &ColorScheme) -> Element<'a, Message> {
        let header = text("Statistics")
            .size(style::TEXT_XL)
            .font(style::FONT_HEADING)
            .line_height(style::LINE_HEIGHT_TIGHT);

        let content: Element<'_, Message> = if self.loading && self.statistics.is_none() {
            text("Loading statistics...")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .into()
        } else if let Some(stats) = &self.statistics {
            let overview = self.overview_card(cs, stats);
            let status_breakdown = self.status_card(cs, stats);
            let score_dist = self.score_card(cs, stats);
            let genres = self.genres_card(cs, stats);

            column![
                row![overview, status_breakdown]
                    .spacing(style::SPACE_LG)
                    .width(Length::Fill),
                row![score_dist, genres]
                    .spacing(style::SPACE_LG)
                    .width(Length::Fill),
            ]
            .spacing(style::SPACE_LG)
            .width(Length::Fill)
            .into()
        } else {
            text("No statistics available. Import or add anime to your library to see stats.")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .into()
        };

        let page = column![header, content]
            .spacing(style::SPACE_XL)
            .padding(style::SPACE_XL)
            .width(Length::Fill);

        scrollable(page).height(Length::Fill).into()
    }

    /// Overview card: total entries, episodes watched, watch time.
    fn overview_card<'a>(
        &self,
        cs: &ColorScheme,
        stats: &LibraryStatistics,
    ) -> Element<'a, Message> {
        let hours = stats.total_watch_time_minutes / 60;
        let days = hours / 24;
        let remaining_hours = hours % 24;

        let time_str = if days > 0 {
            format!("{days}d {remaining_hours}h")
        } else {
            format!("{hours}h")
        };

        let mean_str = stats
            .mean_score
            .map(|s| format!("{s:.1}"))
            .unwrap_or_else(|| "—".into());

        let content = column![
            text("Overview")
                .size(style::TEXT_LG)
                .font(style::FONT_HEADING)
                .line_height(style::LINE_HEIGHT_TIGHT),
            Space::new().height(style::SPACE_SM),
            stat_row("Total Entries", &stats.total_entries.to_string(), cs),
            stat_row(
                "Episodes Watched",
                &stats.total_episodes_watched.to_string(),
                cs,
            ),
            stat_row(
                "Rewatch Episodes",
                &stats.total_rewatch_episodes.to_string(),
                cs,
            ),
            stat_row("Watch Time", &time_str, cs),
            stat_row("Mean Score", &mean_str, cs),
        ]
        .spacing(style::SPACE_XS)
        .width(Length::Fill);

        container(content)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }

    /// Status breakdown card.
    fn status_card<'a>(&self, cs: &ColorScheme, stats: &LibraryStatistics) -> Element<'a, Message> {
        let statuses = [
            (WatchStatus::Watching, "Watching", cs.status_watching),
            (WatchStatus::Completed, "Completed", cs.status_completed),
            (WatchStatus::OnHold, "On Hold", cs.status_on_hold),
            (WatchStatus::Dropped, "Dropped", cs.status_dropped),
            (WatchStatus::PlanToWatch, "Plan to Watch", cs.status_plan),
        ];

        let mut items = column![
            text("By Status")
                .size(style::TEXT_LG)
                .font(style::FONT_HEADING)
                .line_height(style::LINE_HEIGHT_TIGHT),
            Space::new().height(style::SPACE_SM),
        ]
        .spacing(style::SPACE_XS)
        .width(Length::Fill);

        for (status, label, color) in statuses {
            let count = stats.by_status.get(&status).copied().unwrap_or(0);
            let pct = if stats.total_entries > 0 {
                (count as f32 / stats.total_entries as f32) * 100.0
            } else {
                0.0
            };
            items = items.push(status_row(label, count, pct, color, cs));
        }

        container(items)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }

    /// Score distribution card with horizontal bar chart.
    fn score_card<'a>(&self, cs: &ColorScheme, stats: &LibraryStatistics) -> Element<'a, Message> {
        let max_count = stats
            .score_distribution
            .iter()
            .map(|(_, c)| *c)
            .max()
            .unwrap_or(1)
            .max(1);

        let mut bars = column![].spacing(style::SPACE_XS).width(Length::Fill);

        // Show 1-10, always in order
        for bucket in 1..=10u8 {
            let count = stats
                .score_distribution
                .iter()
                .find(|(b, _)| *b == bucket)
                .map(|(_, c)| *c)
                .unwrap_or(0);

            let bar_fraction = count as f32 / max_count as f32;
            bars = bars.push(score_bar(bucket, count, bar_fraction, cs));
        }

        let content = column![
            text("Score Distribution")
                .size(style::TEXT_LG)
                .font(style::FONT_HEADING)
                .line_height(style::LINE_HEIGHT_TIGHT),
            Space::new().height(style::SPACE_SM),
            bars,
        ]
        .spacing(style::SPACE_XS)
        .width(Length::Fill);

        container(content)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }

    /// Top genres card.
    fn genres_card<'a>(&self, cs: &ColorScheme, stats: &LibraryStatistics) -> Element<'a, Message> {
        let mut items = column![
            text("Top Genres")
                .size(style::TEXT_LG)
                .font(style::FONT_HEADING)
                .line_height(style::LINE_HEIGHT_TIGHT),
            Space::new().height(style::SPACE_SM),
        ]
        .spacing(style::SPACE_XS)
        .width(Length::Fill);

        if stats.top_genres.is_empty() {
            items = items.push(
                text("No genre data available")
                    .size(style::TEXT_SM)
                    .color(cs.on_surface_variant),
            );
        } else {
            for (genre, count) in &stats.top_genres {
                items = items.push(stat_row(genre, &count.to_string(), cs));
            }
        }

        container(items)
            .style(theme::card(cs))
            .padding(style::SPACE_LG)
            .width(Length::Fill)
            .into()
    }
}

// ── Helper widgets ────────────────────────────────────────────────

/// A label + value row.
fn stat_row<'a>(label: &str, value: &str, cs: &ColorScheme) -> Element<'a, Message> {
    row![
        text(label.to_string())
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .width(Length::Fill),
        text(value.to_string())
            .size(style::TEXT_SM)
            .font(style::FONT_HEADING),
    ]
    .spacing(style::SPACE_SM)
    .into()
}

/// A status row with colored dot, label, count, and percentage.
fn status_row<'a>(
    label: &str,
    count: usize,
    pct: f32,
    color: iced::Color,
    cs: &ColorScheme,
) -> Element<'a, Message> {
    let dot = container(Space::new().width(8).height(8))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .center_y(Length::Shrink);

    row![
        dot,
        text(label.to_string())
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant)
            .width(Length::Fill),
        text(format!("{count}"))
            .size(style::TEXT_SM)
            .font(style::FONT_HEADING),
        text(format!("{pct:.0}%"))
            .size(style::TEXT_XS)
            .color(cs.outline)
            .width(Length::Fixed(40.0)),
    ]
    .spacing(style::SPACE_SM)
    .align_y(iced::Alignment::Center)
    .into()
}

/// A horizontal bar for score distribution.
fn score_bar<'a>(
    bucket: u8,
    count: usize,
    fraction: f32,
    cs: &ColorScheme,
) -> Element<'a, Message> {
    let bar_color = cs.primary;
    let bar_width = (fraction * 120.0).max(if count > 0 { 4.0 } else { 0.0 });

    let bar = container(Space::new().width(bar_width).height(style::PROGRESS_HEIGHT)).style(
        move |_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(bar_color)),
            border: iced::Border {
                radius: (style::PROGRESS_HEIGHT / 2.0).into(),
                ..Default::default()
            },
            ..Default::default()
        },
    );

    row![
        text(format!("{bucket:>2}"))
            .size(style::TEXT_XS)
            .color(cs.on_surface_variant)
            .width(Length::Fixed(20.0)),
        container(bar)
            .width(Length::Fixed(124.0))
            .center_y(Length::Shrink),
        text(count.to_string())
            .size(style::TEXT_XS)
            .color(cs.outline),
    ]
    .spacing(style::SPACE_SM)
    .align_y(iced::Alignment::Center)
    .into()
}
