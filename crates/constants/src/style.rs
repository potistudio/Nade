//! Shared Blender-inspired design system.
//!
//! Flat chrome, compact density, muted greys, blue selection, orange accent.
//! Prefer these tokens and [`crate::widgets`] style fns over inline magic numbers.

use iced::theme::Palette;
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Accent
// ---------------------------------------------------------------------------

/// Active tool / highlight accent (Blender orange).
pub const ACCENT_COLOR: Color = Color::from_rgb8(232, 125, 13);

/// List / dock selection (Blender blue).
pub const SELECTION_COLOR: Color = Color::from_rgb8(86, 128, 194);

pub const SUCCESS_COLOR: Color = Color::from_rgb8(108, 153, 79);
pub const WARNING_COLOR: Color = Color::from_rgb8(204, 153, 51);
pub const DANGER_COLOR: Color = Color::from_rgb8(184, 72, 72);

// ---------------------------------------------------------------------------
// Background hierarchy (window → panel → widget)
// ---------------------------------------------------------------------------

/// App / dock chrome background (`#303030`).
pub const BACKGROUND_COLOR: Color = Color::from_rgb8(48, 48, 48);

/// Raised panel surface (`#383838`).
pub const PANEL_COLOR: Color = Color::from_rgb8(56, 56, 56);

/// Header / toolbar strip (`#424242`).
pub const HEADER_COLOR: Color = Color::from_rgb8(66, 66, 66);

/// Hover / alternating row (`#3e3e3e`).
pub const BACKGROUND_SELECTED_COLOR: Color = Color::from_rgb8(62, 62, 62);

/// Selected row fill (same as selection blue).
pub const BACKGROUND_ACTIVE_COLOR: Color = SELECTION_COLOR;

/// Recessed widget / input fill (`#1d1d1d`).
pub const WIDGET_COLOR: Color = Color::from_rgb8(29, 29, 29);

/// Widget hover (`#545454`).
pub const WIDGET_HOVER_COLOR: Color = Color::from_rgb8(84, 84, 84);

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub const TEXT_PRIMARY_COLOR: Color = Color::from_rgb8(230, 230, 230);
pub const TEXT_SECONDARY_COLOR: Color = Color::from_rgb8(153, 153, 153);
pub const TEXT_MUTED_COLOR: Color = Color::from_rgb8(102, 102, 102);
/// Text on selected / accent backgrounds.
pub const TEXT_PRIMARY_COLOR_INVERTED: Color = Color::from_rgb8(255, 255, 255);

// ---------------------------------------------------------------------------
// Borders / radius
// ---------------------------------------------------------------------------

pub const BORDER_COLOR: Color = Color::from_rgb8(29, 29, 29);
pub const BORDER_SUBTLE_COLOR: Color = Color::from_rgb8(42, 42, 42);
pub const BORDER_ACTIVE_COLOR: Color = Color::from_rgb8(120, 120, 120);

pub const BORDER_THICKNESS: f32 = 1.0;
/// Blender widgets use a slight radius; panels stay square.
pub const BORDER_RADIUS: f32 = 3.0;
pub const PANEL_RADIUS: f32 = 0.0;

// ---------------------------------------------------------------------------
// Density (Blender-like compact UI)
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

/// Blender Dark palette for stock iced widgets (buttons, pick lists, etc.).
pub const PALETTE: Palette = Palette {
	background: BACKGROUND_COLOR,
	text: TEXT_PRIMARY_COLOR,
	primary: SELECTION_COLOR,
	success: SUCCESS_COLOR,
	warning: WARNING_COLOR,
	danger: DANGER_COLOR,
};

/// Application theme — custom palette so default widgets match Blender Dark.
pub fn app_theme() -> Theme {
	Theme::custom("BlenderDark", PALETTE)
}
