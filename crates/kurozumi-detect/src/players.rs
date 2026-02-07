/// Known media player definitions for detection and filtering.

/// A known media player with its detection identifiers.
pub struct KnownPlayer {
    pub name: &'static str,
    /// MPRIS bus name substring (Linux).
    pub mpris_identity: &'static str,
    /// Window class or process name (Windows).
    pub window_class: &'static str,
    /// Whether this player typically shows the filename in its title.
    pub title_has_filename: bool,
}

/// Registry of known media players.
pub const KNOWN_PLAYERS: &[KnownPlayer] = &[
    KnownPlayer {
        name: "mpv",
        mpris_identity: "mpv",
        window_class: "mpv",
        title_has_filename: true,
    },
    KnownPlayer {
        name: "VLC",
        mpris_identity: "vlc",
        window_class: "vlc",
        title_has_filename: true,
    },
    KnownPlayer {
        name: "MPC-HC",
        mpris_identity: "",
        window_class: "MediaPlayerClassicW",
        title_has_filename: true,
    },
    KnownPlayer {
        name: "MPC-BE",
        mpris_identity: "",
        window_class: "MediaPlayerClassicW",
        title_has_filename: true,
    },
    KnownPlayer {
        name: "PotPlayer",
        mpris_identity: "",
        window_class: "PotPlayer64",
        title_has_filename: true,
    },
];
