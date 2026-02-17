use iced::widget::{button, container, row, text_input};
use iced::{Element, Length};

use crate::style;
use crate::theme::{self, ColorScheme};

/// A number stepper: `[ - ]  [ input ]  [ + ]`
///
/// Standalone icon buttons flank a standard text input.
/// Uses a text buffer so intermediate typing states (e.g. clearing "5" to type "10")
/// don't emit premature value changes. Values commit on Enter (on_submit).
pub fn stepper<'a, Message: Clone + 'a>(
    cs: &ColorScheme,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: Message,
    on_decrement: Option<Message>,
    on_increment: Option<Message>,
) -> Element<'a, Message> {
    let btn_size = style::INPUT_HEIGHT;

    // ── Minus button ───────────────────────────────────────────
    let icon_minus = container(
        lucide_icons::iced::icon_minus()
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant),
    )
    .center(Length::Fill);

    let mut minus_btn = button(icon_minus)
        .width(Length::Fixed(btn_size))
        .height(Length::Fixed(btn_size))
        .padding(0)
        .style(theme::stepper_button(cs));

    if let Some(msg) = on_decrement {
        minus_btn = minus_btn.on_press(msg);
    }

    // ── Center input ───────────────────────────────────────────
    let center_input = text_input("0", value)
        .on_input(on_input)
        .on_submit(on_submit)
        .size(style::INPUT_FONT_SIZE)
        .padding(style::INPUT_PADDING)
        .width(Length::Fill)
        .style(theme::text_input_style(cs));

    // ── Plus button ────────────────────────────────────────────
    let icon_plus = container(
        lucide_icons::iced::icon_plus()
            .size(style::TEXT_SM)
            .color(cs.on_surface_variant),
    )
    .center(Length::Fill);

    let mut plus_btn = button(icon_plus)
        .width(Length::Fixed(btn_size))
        .height(Length::Fixed(btn_size))
        .padding(0)
        .style(theme::stepper_button(cs));

    if let Some(msg) = on_increment {
        plus_btn = plus_btn.on_press(msg);
    }

    row![minus_btn, center_input, plus_btn]
        .spacing(style::SPACE_XS)
        .width(Length::Fixed(style::INPUT_STEPPER_WIDTH))
        .align_y(iced::Alignment::Center)
        .into()
}
