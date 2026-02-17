//! Design tokens: spacing, typography, and layout constants.
//!
//! All spacing is based on a 4px grid. Typography uses a limited scale
//! so every page draws from the same visual hierarchy.

// ── Spacing (4px base grid) ──────────────────────────────────────

pub const SPACE_XXS: f32 = 2.0;
pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 16.0;
pub const SPACE_XL: f32 = 24.0;
pub const SPACE_2XL: f32 = 32.0;
pub const SPACE_3XL: f32 = 48.0;

// ── Typography ───────────────────────────────────────────────────

pub const TEXT_XS: f32 = 11.0;
pub const TEXT_SM: f32 = 12.0;
pub const TEXT_BASE: f32 = 15.0;
pub const TEXT_LG: f32 = 16.0;
pub const TEXT_XL: f32 = 22.0;
pub const TEXT_2XL: f32 = 28.0;
pub const TEXT_3XL: f32 = 36.0;

// Line heights (multipliers for `LineHeight::Relative`)
pub const LINE_HEIGHT_TIGHT: f32 = 1.2; // headings, display text
pub const LINE_HEIGHT_NORMAL: f32 = 1.45; // body text, labels
pub const LINE_HEIGHT_LOOSE: f32 = 1.6; // small/caption text

// Font weight presets
pub const FONT_HEADING: iced::Font = iced::Font {
    family: iced::font::Family::Name("Geist"),
    weight: iced::font::Weight::Medium,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

// ── Layout ───────────────────────────────────────────────────────

pub const NAV_RAIL_WIDTH: f32 = 80.0;
pub const STATUS_BAR_HEIGHT: f32 = 28.0;
pub const COVER_WIDTH: f32 = 130.0;
pub const COVER_HEIGHT: f32 = 185.0;
pub const THUMB_WIDTH: f32 = 40.0;
pub const THUMB_HEIGHT: f32 = 57.0;

// ── Navigation rail ──────────────────────────────────────────────

pub const NAV_ICON_SIZE: f32 = 22.0;
pub const NAV_LABEL_SIZE: f32 = 12.0;

// ── Filter chips ─────────────────────────────────────────────────

pub const CHIP_HEIGHT: f32 = 32.0;
pub const CHIP_RADIUS: f32 = 8.0;

// ── Progress bars ────────────────────────────────────────────────

pub const PROGRESS_HEIGHT: f32 = 6.0;

// ── Badge dimensions ─────────────────────────────────────────────

pub const BADGE_HEIGHT: f32 = 22.0;
pub const BADGE_PADDING_H: f32 = 8.0;

// ── Input components ────────────────────────────────────────────
pub const INPUT_HEIGHT: f32 = 32.0;
pub const INPUT_FONT_SIZE: f32 = TEXT_SM; // 12.0 — all inputs use TEXT_SM
pub const INPUT_PADDING: [f32; 2] = [SPACE_SM, SPACE_MD]; // [8, 12]
pub const INPUT_LABEL_WIDTH: f32 = 120.0; // consistent label column
pub const INPUT_DATE_WIDTH: f32 = 140.0;
pub const INPUT_STEPPER_WIDTH: f32 = 110.0;
pub const TOGGLER_SIZE: f32 = TEXT_BASE; // 15.0

// ── Hero cover (Now Playing) ────────────────────────────────────
pub const HERO_COVER_WIDTH: f32 = 180.0;
pub const HERO_COVER_HEIGHT: f32 = 256.0;

// ── Settings sidebar ────────────────────────────────────────────
pub const SETTINGS_SIDEBAR_WIDTH: f32 = 180.0;

// ── Border radii ─────────────────────────────────────────────────

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 8.0;
pub const RADIUS_LG: f32 = 12.0;
pub const RADIUS_XL: f32 = 16.0;
#[allow(dead_code)]
pub const RADIUS_FULL: f32 = 9999.0;
