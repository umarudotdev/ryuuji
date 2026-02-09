use iced::widget::{center, column, container, row, text, Space};
use iced::{Element, Length, Task};

use chrono::{Local, NaiveDate};
use kurozumi_core::storage::HistoryRow;

use crate::app;
use crate::cover_cache::CoverCache;
use crate::db::DbHandle;
use crate::screen::Action;
use crate::style;
use crate::theme::ColorScheme;
use crate::widgets;

/// History screen state.
pub struct History {
    pub entries: Vec<HistoryRow>,
}

/// Messages handled by the History screen.
#[derive(Debug, Clone)]
pub enum Message {
    HistoryRefreshed(Result<Vec<HistoryRow>, String>),
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Handle a history message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::HistoryRefreshed(Ok(entries)) => {
                self.entries = entries;
                Action::None
            }
            Message::HistoryRefreshed(Err(e)) => Action::SetStatus(format!("History error: {e}")),
        }
    }

    /// Fire a task to load watch history from the DB.
    pub fn load_history(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                db.get_watch_history(500)
                    .await
                    .map_err(|e| e.to_string())
            },
            |result| app::Message::History(Message::HistoryRefreshed(result)),
        ))
    }

    pub fn view<'a>(
        &'a self,
        cs: &ColorScheme,
        cover_cache: &'a CoverCache,
    ) -> Element<'a, Message> {
        if self.entries.is_empty() {
            return empty_state(cs);
        }

        let today = Local::now().date_naive();
        let yesterday = today.pred_opt().unwrap_or(today);

        let mut content = column![].spacing(style::SPACE_XS).width(Length::Fill);
        let mut current_date: Option<NaiveDate> = None;
        let mut is_first_section = true;

        for entry in &self.entries {
            let entry_date = entry.watched_at.with_timezone(&Local).date_naive();

            // Insert date header when the date changes.
            if current_date != Some(entry_date) {
                current_date = Some(entry_date);
                let label = if entry_date == today {
                    "Today".to_string()
                } else if entry_date == yesterday {
                    "Yesterday".to_string()
                } else {
                    entry_date.format("%B %d, %Y").to_string()
                };

                if !is_first_section {
                    content = content.push(Space::new().height(style::SPACE_SM));
                }
                is_first_section = false;
                content = content.push(
                    text(label)
                        .size(style::TEXT_SM)
                        .font(style::FONT_HEADING)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_TIGHT),
                );
            }

            content = content.push(history_item(entry, cs, cover_cache));
        }

        let scrollable_content = crate::widgets::styled_scrollable(
            container(content)
                .padding([style::SPACE_LG, style::SPACE_XL])
                .width(Length::Fill),
            cs,
        )
        .height(Length::Fill);

        container(
            column![
                // Header
                container(
                    text("History")
                        .size(style::TEXT_XL)
                        .font(style::FONT_HEADING)
                        .line_height(style::LINE_HEIGHT_TIGHT),
                )
                .padding(iced::Padding::new(style::SPACE_XL)
                    .top(style::SPACE_LG)
                    .bottom(style::SPACE_SM)),
                scrollable_content,
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

/// A single history item row.
fn history_item<'a>(
    entry: &'a HistoryRow,
    cs: &ColorScheme,
    cover_cache: &'a CoverCache,
) -> Element<'a, Message> {
    let time_str = entry
        .watched_at
        .with_timezone(&Local)
        .format("%H:%M")
        .to_string();
    let title = entry.anime.title.preferred();
    let episode_text = format!("Episode {}", entry.episode);

    let thumb = widgets::rounded_cover(
        cs,
        cover_cache,
        entry.anime.id,
        style::THUMB_WIDTH,
        style::THUMB_HEIGHT,
        style::RADIUS_SM,
    );

    let info = column![
        text(title)
            .size(style::TEXT_BASE)
            .line_height(style::LINE_HEIGHT_NORMAL)
            .color(cs.on_surface),
        text(episode_text)
            .size(style::TEXT_SM)
            .line_height(style::LINE_HEIGHT_LOOSE)
            .color(cs.on_surface_variant),
    ]
    .spacing(style::SPACE_XXS);

    container(
        row![
            text(time_str)
                .size(style::TEXT_SM)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_NORMAL)
                .width(Length::Fixed(48.0)),
            thumb,
            info,
        ]
        .spacing(style::SPACE_MD)
        .align_y(iced::Alignment::Center),
    )
    .padding([style::SPACE_SM, style::SPACE_MD])
    .width(Length::Fill)
    .into()
}

/// Empty state when no history exists.
fn empty_state(cs: &ColorScheme) -> Element<'static, Message> {
    center(
        column![
            text("No watch history yet")
                .size(style::TEXT_LG)
                .font(style::FONT_HEADING)
                .color(cs.on_surface_variant)
                .line_height(style::LINE_HEIGHT_TIGHT),
            text("Start watching anime and your history will appear here.")
                .size(style::TEXT_SM)
                .color(cs.outline)
                .line_height(style::LINE_HEIGHT_LOOSE),
        ]
        .spacing(style::SPACE_SM)
        .align_x(iced::Alignment::Center),
    )
    .into()
}
