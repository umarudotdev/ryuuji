use iced::widget::{button, column, container, row, scrollable, text, Rule};
use iced::{Element, Length};

use kurozumi_core::models::WatchStatus;
use kurozumi_core::storage::LibraryRow;

use crate::app::Message;

/// Render the Library page.
pub fn view<'a>(
    active_tab: WatchStatus,
    entries: &'a [LibraryRow],
    selected_anime_id: Option<i64>,
) -> Element<'a, Message> {
    let tabs = tab_bar(active_tab);

    let list: Element<'_, Message> = if entries.is_empty() {
        container(text("No anime in this list.").size(14))
            .padding(20)
            .into()
    } else {
        let items: Vec<Element<'a, Message>> = entries
            .iter()
            .map(|row| anime_list_item(row, selected_anime_id))
            .collect();

        scrollable(
            column(items).spacing(2).padding(10),
        )
        .height(Length::Fill)
        .into()
    };

    let content = column![tabs, Rule::horizontal(1), list]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    // If an anime is selected, show detail panel on the right.
    if let Some(anime_id) = selected_anime_id {
        if let Some(row) = entries.iter().find(|r| r.anime.id == anime_id) {
            let detail = anime_detail(row);
            return row![
                container(content).width(Length::FillPortion(3)),
                Rule::vertical(1),
                container(detail).width(Length::FillPortion(2)),
            ]
            .height(Length::Fill)
            .into();
        }
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render the tab bar for watch status filtering.
fn tab_bar(active: WatchStatus) -> Element<'static, Message> {
    let tabs: Vec<Element<'_, Message>> = WatchStatus::ALL
        .iter()
        .map(|&status| {
            let label = match status {
                WatchStatus::PlanToWatch => "Plan",
                other => other.as_str(),
            };
            let btn = button(text(label).size(12))
                .on_press(Message::LibraryTabChanged(status));
            if status == active {
                btn.style(button::primary).into()
            } else {
                btn.style(button::text).into()
            }
        })
        .collect();

    row(tabs).spacing(4).padding(10).into()
}

/// Render a single anime in the list.
fn anime_list_item<'a>(row: &'a LibraryRow, selected: Option<i64>) -> Element<'a, Message> {
    let title = row.anime.title.preferred();
    let progress = match row.anime.episodes {
        Some(total) => format!("{}/{}", row.entry.watched_episodes, total),
        None => format!("{}", row.entry.watched_episodes),
    };

    let is_selected = selected == Some(row.anime.id);

    let content = row![
        text(title).size(14).width(Length::Fill),
        text(progress).size(14),
    ]
    .spacing(8)
    .padding(6);

    let btn = button(content)
        .width(Length::Fill)
        .on_press(Message::AnimeSelected(row.anime.id));

    if is_selected {
        btn.style(button::secondary).into()
    } else {
        btn.style(button::text).into()
    }
}

/// Render the detail panel for a selected anime.
fn anime_detail<'a>(row: &'a LibraryRow) -> Element<'a, Message> {
    let title = row.anime.title.preferred();
    let ep_text = match row.anime.episodes {
        Some(total) => format!("Progress: {} / {}", row.entry.watched_episodes, total),
        None => format!("Progress: {}", row.entry.watched_episodes),
    };

    let mut detail = column![
        text(title).size(22),
        text(ep_text).size(16),
        text(format!("Status: {}", row.entry.status.as_str())).size(14),
    ]
    .spacing(8)
    .padding(16);

    if let Some(english) = &row.anime.title.english {
        if Some(english.as_str()) != row.anime.title.romaji.as_deref() {
            detail = detail.push(text(english.as_str()).size(12));
        }
    }

    if let Some(score) = row.entry.score {
        detail = detail.push(text(format!("Score: {score:.1}")).size(14));
    }

    // Episode increment/decrement buttons.
    let ep_buttons = row![
        button(text("-").size(14))
            .on_press(Message::EpisodeDecrement(row.anime.id))
            .style(button::secondary),
        text(format!(" {} ", row.entry.watched_episodes)).size(16),
        button(text("+").size(14))
            .on_press(Message::EpisodeIncrement(row.anime.id))
            .style(button::secondary),
    ]
    .spacing(8);

    detail = detail.push(Rule::horizontal(1));
    detail = detail.push(ep_buttons);

    detail.into()
}
