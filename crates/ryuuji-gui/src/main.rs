mod app;
mod cover_cache;
mod db;
mod format;
mod screen;
mod style;
mod subscription;
mod theme;
mod widgets;
mod window_state;

const GEIST_SANS: &[u8] = include_bytes!("../assets/fonts/Geist-Variable.ttf");
const GEIST_MONO: &[u8] = include_bytes!("../assets/fonts/GeistMono-Variable.ttf");

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("ryuuji=debug")
        .init();

    let ws = window_state::WindowState::load();

    let mut app = iced::application(
        app::Ryuuji::new,
        app::Ryuuji::update,
        app::Ryuuji::view,
    )
    .title(app::Ryuuji::title)
    .subscription(app::Ryuuji::subscription)
    .theme(app::Ryuuji::theme)
    .font(GEIST_SANS)
    .font(GEIST_MONO)
    .font(lucide_icons::LUCIDE_FONT_BYTES)
    .default_font(iced::Font::with_name("Geist"))
    .window_size(ws.size());

    if let Some(pos) = ws.position() {
        app = app.position(iced::window::Position::Specific(pos));
    } else {
        app = app.centered();
    }

    app.run()
}
