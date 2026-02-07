mod app;
mod pages;
mod subscription;
mod theme;
mod widgets;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter("kurozumi=debug")
        .init();

    iced::application(app::Kurozumi::title, app::Kurozumi::update, app::Kurozumi::view)
        .subscription(app::Kurozumi::subscription)
        .theme(app::Kurozumi::theme)
        .run()
}
