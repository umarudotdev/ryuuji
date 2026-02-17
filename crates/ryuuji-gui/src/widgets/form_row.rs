use iced::widget::{row, text};
use iced::{Alignment, Element, Length};

use crate::style;
use crate::theme::ColorScheme;

/// A consistent label:control row used in detail panels and settings.
///
/// Renders as: `[ label (fill) | control ]` with consistent font size,
/// color, alignment, and spacing.
pub fn form_row<'a, Message: 'a>(
    cs: &ColorScheme,
    label: &str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        text(label.to_string())
            .size(style::INPUT_FONT_SIZE)
            .color(cs.on_surface)
            .line_height(style::LINE_HEIGHT_NORMAL)
            .width(Length::Fill),
        control,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACE_SM)
    .into()
}
