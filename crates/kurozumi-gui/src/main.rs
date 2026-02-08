mod app;
mod db;
mod screen;
mod style;
mod subscription;
mod theme;
mod widgets;
mod window_state;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("kurozumi=debug")
        .init();

    let ws = window_state::WindowState::load();

    let mut app = iced::application(app::Kurozumi::new, app::Kurozumi::update, app::Kurozumi::view)
        .title(app::Kurozumi::title)
        .subscription(app::Kurozumi::subscription)
        .theme(app::Kurozumi::theme)
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .window_size(ws.size());

    if let Some(pos) = ws.position() {
        app = app.position(iced::window::Position::Specific(pos));
    } else {
        app = app.centered();
    }

    app.run()
}
