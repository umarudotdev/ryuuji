pub mod debug;
pub mod history;
pub mod library;
pub mod now_playing;
pub mod search;
pub mod seasons;
pub mod settings;
pub mod stats;
pub mod torrents;

use iced::Task;

use ryuuji_core::models::WatchStatus;

use crate::app;
use crate::toast::ToastKind;

/// Which page is currently displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    NowPlaying,
    Library,
    History,
    Search,
    Seasons,
    Torrents,
    Stats,
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
    /// Show a toast notification.
    ShowToast(String, ToastKind),
}

/// Actions available in context menus for library entries.
///
/// Shared between Library and Search screens.
#[derive(Debug, Clone)]
pub enum ContextAction {
    ChangeStatus(WatchStatus),
    Delete,
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
