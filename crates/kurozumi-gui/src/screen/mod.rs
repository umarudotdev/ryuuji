pub mod library;
pub mod now_playing;
pub mod search;
pub mod settings;

use iced::Task;

use crate::app;

/// Which page is currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    NowPlaying,
    Library,
    Search,
    Settings,
}

/// Actions that a screen can request from the app router.
///
/// Screens return these from `update()` instead of directly mutating
/// shared state — the app interprets them in one place.
#[allow(dead_code)]
pub enum Action {
    /// No side-effect.
    None,
    /// Navigate to a different page.
    NavigateTo(Page),
    /// Refresh the library entries (e.g. after a DB write).
    RefreshLibrary,
    /// Update the status bar message.
    SetStatus(String),
    /// Show a modal dialog.
    ShowModal(ModalKind),
    /// Dismiss the current modal.
    DismissModal,
    /// Run an async Iced task that eventually produces an app::Message.
    RunTask(Task<app::Message>),
}

/// What kind of modal is currently shown.
#[derive(Debug, Clone)]
pub enum ModalKind {
    ConfirmDelete {
        anime_id: i64,
        title: String,
        source: Page,
    },
}
