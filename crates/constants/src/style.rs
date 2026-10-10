//! Shared Catppuccin Mocha design system.
//!
//! Flat chrome, compact density, warm dark surfaces, sky accent.
//! Prefer these tokens and [`crate::widgets`] style fns over inline magic numbers.

use iced::theme::Palette;
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Accent
// ---------------------------------------------------------------------------

/// Active tool / playhead / highlight (sky).
pub const ACCENT_COLOR: Color = Color::from_rgb8(137, 220, 235);

/// List / dock selection (blue).
pub const SELECTION_COLOR: Color = Color::from_rgb8(137, 180, 250);

pub const SUCCESS_COLOR: Color = Color::from_rgb8(166, 227, 161);
pub const WARNING_COLOR: Color = Color::from_rgb8(249, 226, 175);
pub const DANGER_COLOR: Color = Color::from_rgb8(243, 139, 168);

// ---------------------------------------------------------------------------
// Background hierarchy (window → panel → widget)
// ---------------------------------------------------------------------------

/// App / dock chrome background (base, `#1e1e2e`).
pub const BACKGROUND_COLOR: Color = Color::from_rgb8(30, 30, 46);

/// Raised panel surface (mantle, `#181825`).
pub const PANEL_COLOR: Color = Color::from_rgb8(24, 24, 37);

/// Header / toolbar strip (surface0, `#313244`).
pub const HEADER_COLOR: Color = Color::from_rgb8(49, 50, 68);

/// Hover / alternating row (surface0, `#313244`).
pub const BACKGROUND_SELECTED_COLOR: Color = Color::from_rgb8(49, 50, 68);

/// Selected row fill (same as selection blue).
pub const BACKGROUND_ACTIVE_COLOR: Color = SELECTION_COLOR;

/// Recessed widget / input fill (crust, `#11111b`).
pub const WIDGET_COLOR: Color = Color::from_rgb8(17, 17, 27);

/// Widget hover (surface1, `#45475a`).
pub const WIDGET_HOVER_COLOR: Color = Color::from_rgb8(69, 71, 90);

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub const TEXT_PRIMARY_COLOR: Color = Color::from_rgb8(205, 214, 244);
pub const TEXT_SECONDARY_COLOR: Color = Color::from_rgb8(166, 173, 200);
pub const TEXT_MUTED_COLOR: Color = Color::from_rgb8(108, 112, 134);
/// Text on selected / accent backgrounds.
pub const TEXT_PRIMARY_COLOR_INVERTED: Color = Color::from_rgb8(30, 30, 46);

// ---------------------------------------------------------------------------
// Borders / radius
// ---------------------------------------------------------------------------

pub const BORDER_COLOR: Color = Color::from_rgb8(17, 17, 27);
pub const BORDER_SUBTLE_COLOR: Color = Color::from_rgb8(49, 50, 68);
pub const BORDER_ACTIVE_COLOR: Color = Color::from_rgb8(127, 132, 156);

pub const BORDER_THICKNESS: f32 = 1.0;
/// Compact widget radius.
pub const BORDER_RADIUS: f32 = 3.0;
/// Rounded panel chrome.
pub const PANEL_RADIUS: f32 = 6.0;
/// Header/body squircle radius inside panel outline padding.
pub const PANEL_INNER_RADIUS: f32 = PANEL_RADIUS - BORDER_THICKNESS;
/// Superellipse exponent for panel squircles (`4.0` = classic squircle).
pub const PANEL_SQUIRCLE_EXPONENT: f32 = 4.0;

// ---------------------------------------------------------------------------
// Density (compact UI)
// ---------------------------------------------------------------------------

pub const SPACE_1: f32 = 2.0;
pub const SPACE_2: f32 = 4.0;
pub const SPACE_3: f32 = 6.0;
pub const SPACE_4: f32 = 8.0;

pub const FONT_TINY: f32 = 8.0;
pub const FONT_UI: f32 = 10.0;
pub const FONT_LABEL: f32 = 10.0;
pub const FONT_TITLE: f32 = 10.0;

pub const ROW_HEIGHT: f32 = 14.0;
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

/// Catppuccin Mocha palette for stock iced widgets (buttons, pick lists, etc.).
pub const PALETTE: Palette = Palette {
	background: BACKGROUND_COLOR,
	text: TEXT_PRIMARY_COLOR,
	primary: SELECTION_COLOR,
	success: SUCCESS_COLOR,
	warning: WARNING_COLOR,
	danger: DANGER_COLOR,
};

/// Application theme — Catppuccin Mocha.
pub fn app_theme() -> Theme {
	Theme::custom("CatppuccinMocha", PALETTE)
}
