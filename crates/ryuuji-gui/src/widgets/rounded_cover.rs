use iced::widget::container;
use iced::{ContentFit, Element, Length};

use crate::cover_cache::{CoverCache, CoverState};
use crate::style;
use crate::theme::{self, ColorScheme};

/// Render a cover image with rounded corners, or a styled placeholder.
///
/// Uses `ContentFit::Cover` so the image fills the frame completely,
/// cropping any overflow rather than leaving gaps. The container always
/// has the placeholder background so a failed/blank image still shows
/// a visible frame.
pub fn rounded_cover<'a, Message: 'static>(
    cs: &ColorScheme,
    covers: &'a CoverCache,
    anime_id: i64,
    width: f32,
    height: f32,
    radius: f32,
) -> Element<'a, Message> {
    if let Some(CoverState::Loaded(path)) = covers.get(anime_id) {
        container(
            iced::widget::image(path.as_path())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(ContentFit::Cover)
                .border_radius(radius),
        )
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(theme::cover_placeholder(cs, radius))
        .into()
    } else {
        let icon_size = if width <= style::THUMB_WIDTH {
            style::TEXT_BASE
        } else {
            style::TEXT_3XL
        };
        container(
            lucide_icons::iced::icon_film()
                .size(icon_size)
                .color(cs.outline)
                .center(),
        )
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .center_x(Length::Fixed(width))
        .center_y(Length::Fixed(height))
        .style(theme::cover_placeholder(cs, radius))
        .into()
    }
}
