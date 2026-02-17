use iced::widget::{center, column, text};
use iced::{Alignment, Element, Length};

use crate::style;
use crate::theme::ColorScheme;

/// A centered empty state with icon, title, and subtitle.
///
/// Used across screens for "nothing here yet" placeholders.
pub fn empty_state<'a, Message: 'a>(
    cs: &ColorScheme,
    icon: Element<'a, Message>,
    title: &'a str,
    subtitle: &'a str,
) -> Element<'a, Message> {
    let content = column![
        icon,
        text(title)
            .size(style::TEXT_XL)
            .font(style::FONT_HEADING)
            .color(cs.on_surface_variant)
            .line_height(style::LINE_HEIGHT_TIGHT),
        text(subtitle)
            .size(style::TEXT_SM)
            .color(cs.outline)
            .line_height(style::LINE_HEIGHT_LOOSE),
    ]
    .spacing(style::SPACE_MD)
    .align_x(Alignment::Center);

    center(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
