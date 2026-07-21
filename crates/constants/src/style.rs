//! Shared cool dark design system.
//!
//! Flat chrome, compact density, blue-grey surfaces, cyan accent.
//! Prefer these tokens and [`crate::widgets`] style fns over inline magic numbers.

use iced::theme::Palette;
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Accent
// ---------------------------------------------------------------------------

/// Active tool / playhead / highlight (cool cyan).
pub const ACCENT_COLOR: Color = Color::from_rgb8(72, 196, 220);

/// List / dock selection (cool blue).
pub const SELECTION_COLOR: Color = Color::from_rgb8(64, 140, 196);

pub const SUCCESS_COLOR: Color = Color::from_rgb8(72, 168, 132);
pub const WARNING_COLOR: Color = Color::from_rgb8(196, 156, 72);
pub const DANGER_COLOR: Color = Color::from_rgb8(196, 84, 96);

// ---------------------------------------------------------------------------
// Background hierarchy (window → panel → widget)
// ---------------------------------------------------------------------------

/// App / dock chrome background (`#0f1318`).
pub const BACKGROUND_COLOR: Color = Color::from_rgb8(15, 19, 24);

/// Raised panel surface (`#1c222a`).
pub const PANEL_COLOR: Color = Color::from_rgb8(28, 34, 42);

/// Header / toolbar strip (`#252c36`).
pub const HEADER_COLOR: Color = Color::from_rgb8(37, 44, 54);

/// Hover / alternating row (`#222933`).
pub const BACKGROUND_SELECTED_COLOR: Color = Color::from_rgb8(34, 41, 51);

/// Selected row fill (same as selection blue).
pub const BACKGROUND_ACTIVE_COLOR: Color = SELECTION_COLOR;

/// Recessed widget / input fill (`#12171d`).
pub const WIDGET_COLOR: Color = Color::from_rgb8(18, 23, 29);

/// Widget hover (`#323a46`).
pub const WIDGET_HOVER_COLOR: Color = Color::from_rgb8(50, 58, 70);

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub const TEXT_PRIMARY_COLOR: Color = Color::from_rgb8(220, 228, 236);
pub const TEXT_SECONDARY_COLOR: Color = Color::from_rgb8(140, 152, 168);
pub const TEXT_MUTED_COLOR: Color = Color::from_rgb8(96, 108, 124);
/// Text on selected / accent backgrounds.
pub const TEXT_PRIMARY_COLOR_INVERTED: Color = Color::from_rgb8(255, 255, 255);

// ---------------------------------------------------------------------------
// Borders / radius
// ---------------------------------------------------------------------------

pub const BORDER_COLOR: Color = Color::from_rgb8(18, 23, 29);
pub const BORDER_SUBTLE_COLOR: Color = Color::from_rgb8(36, 44, 54);
pub const BORDER_ACTIVE_COLOR: Color = Color::from_rgb8(112, 128, 148);

pub const BORDER_THICKNESS: f32 = 1.0;
/// Compact widget radius.
pub const BORDER_RADIUS: f32 = 3.0;
/// Rounded panel chrome.
pub const PANEL_RADIUS: f32 = 6.0;

// ---------------------------------------------------------------------------
// Density (compact UI)
// ---------------------------------------------------------------------------

pub const SPACE_1: f32 = 2.0;
pub const SPACE_2: f32 = 4.0;
pub const SPACE_3: f32 = 6.0;
pub const SPACE_4: f32 = 8.0;

pub const FONT_TINY: f32 = 10.0;
pub const FONT_UI: f32 = 11.0;
pub const FONT_LABEL: f32 = 12.0;
pub const FONT_TITLE: f32 = 12.0;

pub const ROW_HEIGHT: f32 = 20.0;
pub const INPUT_HEIGHT: f32 = 18.0;
pub const HEADER_HEIGHT: f32 = 24.0;
pub const TOOLBAR_HEIGHT: f32 = 22.0;

pub const PAD_BUTTON: [u16; 2] = [1, 6];
pub const PAD_INPUT: [u16; 2] = [1, 4];
pub const PAD_HEADER: [u16; 2] = [0, 4];
pub const PAD_PANEL: [u16; 2] = [4, 6];
pub const PAD_TOOLBAR: [u16; 2] = [2, 6];

pub const TREE_INDENT: f32 = 14.0;

// ---------------------------------------------------------------------------
// Iced Theme
// ---------------------------------------------------------------------------

/// Cool dark palette for stock iced widgets (buttons, pick lists, etc.).
pub const PALETTE: Palette = Palette {
	background: BACKGROUND_COLOR,
	text: TEXT_PRIMARY_COLOR,
	primary: SELECTION_COLOR,
	success: SUCCESS_COLOR,
	warning: WARNING_COLOR,
	danger: DANGER_COLOR,
};

/// Application theme — custom cool dark palette.
pub fn app_theme() -> Theme {
	Theme::custom("NadeCoolDark", PALETTE)
}
