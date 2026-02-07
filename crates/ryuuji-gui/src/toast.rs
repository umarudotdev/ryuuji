use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::style;
use crate::theme::{self, ColorScheme};

/// Kind of toast notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// A single toast notification.
#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
}

/// Auto-dismiss delay in seconds.
pub const AUTO_DISMISS_SECS: u64 = 4;

/// Render the toast overlay — a column of toasts anchored top-right.
pub fn toast_overlay<'a, Message: Clone + 'a>(
    cs: &ColorScheme,
    toasts: &'a [Toast],
    on_dismiss: impl Fn(u64) -> Message + 'a,
) -> Element<'a, Message> {
    if toasts.is_empty() {
        return iced::widget::Space::new().width(0).height(0).into();
    }

    let mut toast_column = column![]
        .spacing(style::SPACE_SM)
        .width(Length::Fixed(320.0));

    for toast in toasts {
        let (icon, bg_color) = match toast.kind {
            ToastKind::Success => (lucide_icons::iced::icon_circle_check(), cs.status_completed),
            ToastKind::Error => (lucide_icons::iced::icon_circle_x(), cs.error),
            ToastKind::Info => (lucide_icons::iced::icon_info(), cs.primary),
        };

        let dismiss_msg = on_dismiss(toast.id);

        let toast_card = container(
            row![
                icon.size(style::TEXT_LG).color(bg_color),
                text(toast.message.as_str())
                    .size(style::TEXT_SM)
                    .line_height(style::LINE_HEIGHT_NORMAL)
                    .width(Length::Fill),
                button(
                    lucide_icons::iced::icon_x()
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant),
                )
                .on_press(dismiss_msg)
                .padding(style::SPACE_XXS)
                .style(theme::icon_button(cs)),
            ]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
        )
        .style(theme::card(cs))
        .padding([style::SPACE_SM, style::SPACE_MD])
        .width(Length::Fill);

        toast_column = toast_column.push(toast_card);
    }

    container(toast_column)
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .padding([style::SPACE_MD, style::SPACE_XL])
        .into()
}
