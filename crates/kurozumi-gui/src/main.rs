mod app;
mod pages;
mod style;
mod subscription;
mod theme;
mod widgets;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("kurozumi=debug")
        .init();

    iced::application(app::Kurozumi::new, app::Kurozumi::update, app::Kurozumi::view)
        .title(app::Kurozumi::title)
        .subscription(app::Kurozumi::subscription)
        .theme(app::Kurozumi::theme)
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .window_size((960.0, 640.0))
        .centered()
        .run()
}
