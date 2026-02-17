pub mod anime_card;
pub mod anime_list_item;
pub mod detail_panel;
pub mod empty_state;
pub mod form_row;
pub mod modal;
pub mod rounded_cover;
pub mod stepper;

pub use anime_card::{library_card, online_card};
pub use anime_list_item::anime_list_item;
pub use detail_panel::{detail_panel, online_detail_panel};
pub use empty_state::empty_state;
pub use form_row::form_row;
pub use modal::modal;
pub use rounded_cover::rounded_cover;
pub use stepper::stepper;

use iced::widget::scrollable;
use iced::Element;

use crate::theme::{self, ColorScheme};

/// A scrollable with consistent direction and style across the application.
pub fn styled_scrollable<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    cs: &ColorScheme,
) -> scrollable::Scrollable<'a, Message> {
    scrollable(content)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(6)
                .scroller_width(4)
                .margin(2),
        ))
        .style(theme::overlay_scrollbar(cs))
}
