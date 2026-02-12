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

use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const GEIST_SANS: &[u8] = include_bytes!("../assets/fonts/Geist-Variable.ttf");
const GEIST_MONO: &[u8] = include_bytes!("../assets/fonts/GeistMono-Variable.ttf");
const APP_ICON: &[u8] = include_bytes!("../assets/icon.png");

#[derive(Parser)]
#[command(name = "ryuuji", about = "Desktop anime tracker")]
struct Cli {
    /// Enable verbose logging (debug level)
    #[arg(short, long)]
    verbose: bool,

    /// Enable trace-level logging (most detailed)
    #[arg(long)]
    trace: bool,

    /// Set log level explicitly
    #[arg(long, value_parser = ["error", "warn", "info", "debug", "trace"])]
    log_level: Option<String>,
}

fn main() -> iced::Result {
    let cli = Cli::parse();
    let config = ryuuji_core::config::AppConfig::load().unwrap_or_default();

    // CLI flags take highest priority, always.
    let cli_override = cli.trace || cli.verbose || cli.log_level.is_some();

    // Priority: CLI flag > RUST_LOG env > config.toml > default ("info")
    let level = if cli.trace {
        "trace"
    } else if cli.verbose {
        "debug"
    } else if let Some(ref lvl) = cli.log_level {
        lvl.as_str()
    } else {
        config.logging.level.as_str()
    };

    let env_filter = if cli_override {
        EnvFilter::new(format!("ryuuji={level}"))
    } else if std::env::var("RUST_LOG").is_ok() {
        EnvFilter::from_default_env()
    } else {
        EnvFilter::new(format!("ryuuji={level}"))
    };

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_names(false);

    // File logging layer (daily rotation, 7-day retention).
    let _file_guard = if config.logging.file_logging {
        let log_dir = ryuuji_core::config::AppConfig::log_dir();
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            eprintln!(
                "Warning: could not create log directory {}: {e}",
                log_dir.display()
            );
        }

        let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("ryuuji")
            .filename_suffix("log")
            .max_log_files(7)
            .build(&log_dir)
            .expect("failed to create log file appender");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_writer(non_blocking);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .with(file_layer)
            .init();

        Some(guard)
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .init();

        None
    };

    tracing::info!(
        config_path = %ryuuji_core::config::AppConfig::config_path().display(),
        db_path = %ryuuji_core::config::AppConfig::db_path().display(),
        log_level = %level,
        file_logging = config.logging.file_logging,
        "Ryuuji starting"
    );

    let ws = window_state::WindowState::load();
    let icon = iced::window::icon::from_file_data(APP_ICON, None).ok();

    let mut win = iced::window::Settings {
        size: ws.size(),
        icon,
        ..Default::default()
    };

    if let Some(pos) = ws.position() {
        win.position = iced::window::Position::Specific(pos);
    } else {
        win.position = iced::window::Position::Centered;
    }

    iced::application(app::Ryuuji::new, app::Ryuuji::update, app::Ryuuji::view)
        .title(app::Ryuuji::title)
        .subscription(app::Ryuuji::subscription)
        .theme(app::Ryuuji::theme)
        .font(GEIST_SANS)
        .font(GEIST_MONO)
        .font(lucide_icons::LUCIDE_FONT_BYTES)
        .default_font(iced::Font::with_name("Geist"))
        .window(win)
        .run()
}
