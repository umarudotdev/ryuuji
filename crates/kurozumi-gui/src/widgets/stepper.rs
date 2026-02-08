use iced::widget::{button, container, row, text_input};
use iced::{Border, Element, Length};

use crate::style;
use crate::theme::{self, ColorScheme};

/// A pill-group number stepper: `[ - | text_input | + ]`
///
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
    // ── Left button (minus) ──────────────────────────────────────
    let icon_minus =
        container(lucide_icons::iced::icon_minus().size(style::TEXT_BASE)).center(Length::Fill);

    let mut minus_btn = button(icon_minus)
        .width(Length::Fixed(32.0))
        .height(Length::Fill)
        .padding(0)
        .style(theme::stepper_button_style(cs, true, false));

    if let Some(msg) = on_decrement {
        minus_btn = minus_btn.on_press(msg);
    }

    // ── Center input ─────────────────────────────────────────────
    // Vertical padding sized to fill the 32px row height (32 - 12px font = 20 / 2 = 10px)
    let center_input = text_input("0", value)
        .on_input(on_input)
        .on_submit(on_submit)
        .size(style::TEXT_SM)
        .padding([10.0, style::SPACE_SM])
        .width(Length::Fill)
        .style(stepper_input_style(cs));

    // ── Right button (plus) ──────────────────────────────────────
    let icon_plus =
        container(lucide_icons::iced::icon_plus().size(style::TEXT_BASE)).center(Length::Fill);

    let mut plus_btn = button(icon_plus)
        .width(Length::Fixed(32.0))
        .height(Length::Fill)
        .padding(0)
        .style(theme::stepper_button_style(cs, false, true));

    if let Some(msg) = on_increment {
        plus_btn = plus_btn.on_press(msg);
    }

    row![minus_btn, center_input, plus_btn]
        .width(Length::Fixed(110.0))
        .height(Length::Fixed(32.0))
        .into()
}

/// Text input style for the stepper center segment: flat edges (0 radius) to
/// sit flush between the pill-group buttons.
fn stepper_input_style(
    cs: &ColorScheme,
) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
    let primary = cs.primary;
    let outline = cs.outline;
    let outline_variant = cs.outline_variant;
    let surface_container_low = cs.surface_container_low;
    let on_surface_variant = cs.on_surface_variant;
    let on_surface = cs.on_surface;

    move |_theme, status| {
        let border_color = match status {
            text_input::Status::Focused { .. } => primary,
            text_input::Status::Hovered => outline,
            _ => outline_variant,
        };
        text_input::Style {
            background: iced::Background::Color(surface_container_low),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 0.0.into(),
            },
            icon: on_surface_variant,
            placeholder: outline,
            value: on_surface,
            selection: primary,
        }
    }
}
