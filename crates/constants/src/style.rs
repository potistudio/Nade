//! Shared "Graphite" design system.
//!
//! Apple Pro app inspired: neutral graphite surfaces layered by elevation,
//! a single system-blue accent, hierarchy carried by color rather than weight,
//! continuous (squircle) corners and compact-but-breathable density.
//! Prefer these tokens and [`crate::widgets`] style fns over inline magic numbers.

use iced::theme::Palette;
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Accent
// ---------------------------------------------------------------------------

/// Active tool / playhead / focus (system blue).
pub const ACCENT_COLOR: Color = Color::from_rgb8(10, 132, 255);

/// Brighter accent for strokes drawn on dark surfaces.
pub const ACCENT_BRIGHT_COLOR: Color = Color::from_rgb8(64, 156, 255);

/// List / dock selection (system blue).
pub const SELECTION_COLOR: Color = ACCENT_COLOR;

/// Keyframes and animated-property markers (system yellow).
pub const KEYFRAME_COLOR: Color = Color::from_rgb8(255, 214, 10);

pub const SUCCESS_COLOR: Color = Color::from_rgb8(48, 209, 88);
pub const WARNING_COLOR: Color = Color::from_rgb8(255, 159, 10);
pub const DANGER_COLOR: Color = Color::from_rgb8(255, 69, 58);

/// Per-axis tints (X / Y / Z) for vector fields and gizmos.
pub const AXIS_X_COLOR: Color = Color::from_rgb8(255, 105, 97);
pub const AXIS_Y_COLOR: Color = Color::from_rgb8(103, 214, 120);
pub const AXIS_Z_COLOR: Color = Color::from_rgb8(90, 166, 255);

/// Layer label colors (After Effects style label palette, Apple system hues).
pub const LABEL_COLORS: [Color; 8] = [
	Color::from_rgb8(10, 132, 255),
	Color::from_rgb8(191, 90, 242),
	Color::from_rgb8(255, 55, 95),
	Color::from_rgb8(255, 159, 10),
	Color::from_rgb8(48, 209, 88),
	Color::from_rgb8(64, 200, 224),
	Color::from_rgb8(94, 92, 230),
	Color::from_rgb8(255, 214, 10),
];

// ---------------------------------------------------------------------------
// Background hierarchy (window → panel → widget)
// ---------------------------------------------------------------------------

/// Window base: gutters, toolbar and status bar (`#111113`).
pub const BACKGROUND_COLOR: Color = Color::from_rgb8(17, 17, 19);

/// Panel body surface (`#1b1b1e`).
pub const PANEL_COLOR: Color = Color::from_rgb8(27, 27, 30);

/// Panel header strip, slightly raised (`#212124`).
pub const HEADER_COLOR: Color = Color::from_rgb8(33, 33, 36);

/// Popovers, menus and HUDs (`#2a2a2e`).
pub const RAISED_COLOR: Color = Color::from_rgb8(42, 42, 46);

/// Hover / alternating row.
pub const BACKGROUND_SELECTED_COLOR: Color = Color::from_rgb8(38, 38, 42);

/// Selected row fill (same as selection blue).
pub const BACKGROUND_ACTIVE_COLOR: Color = SELECTION_COLOR;

/// Field / control fill (`#2c2c30`).
pub const WIDGET_COLOR: Color = Color::from_rgb8(44, 44, 48);

/// Control hover.
pub const WIDGET_HOVER_COLOR: Color = Color::from_rgb8(56, 56, 61);

/// Control pressed.
pub const WIDGET_PRESSED_COLOR: Color = Color::from_rgb8(68, 68, 74);

/// Recessed wells: viewer backdrop, track lanes (`#0e0e10`).
pub const WELL_COLOR: Color = Color::from_rgb8(14, 14, 16);

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub const TEXT_PRIMARY_COLOR: Color = Color::from_rgb8(236, 236, 240);
pub const TEXT_SECONDARY_COLOR: Color = Color::from_rgb8(152, 152, 160);
pub const TEXT_MUTED_COLOR: Color = Color::from_rgb8(99, 99, 106);
/// Text on selected / accent backgrounds.
pub const TEXT_PRIMARY_COLOR_INVERTED: Color = Color::WHITE;

// ---------------------------------------------------------------------------
// Borders / radius
// ---------------------------------------------------------------------------

/// Hairline between surfaces.
pub const BORDER_COLOR: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
/// Separators inside panels.
pub const BORDER_SUBTLE_COLOR: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.07);
/// Hover / emphasized outline.
pub const BORDER_ACTIVE_COLOR: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.18);

pub const BORDER_THICKNESS: f32 = 1.0;
/// Control radius.
pub const BORDER_RADIUS: f32 = 5.0;
/// Small chip / tag radius.
pub const CHIP_RADIUS: f32 = 4.0;
/// Rounded panel chrome (continuous corners).
pub const PANEL_RADIUS: f32 = 10.0;
/// Header/body squircle radius inside panel outline padding.
pub const PANEL_INNER_RADIUS: f32 = PANEL_RADIUS - BORDER_THICKNESS;
/// Superellipse exponent for panel squircles (`5.0` ≈ iOS continuous corners).
pub const PANEL_SQUIRCLE_EXPONENT: f32 = 5.0;

// ---------------------------------------------------------------------------
// Density
// ---------------------------------------------------------------------------

pub const SPACE_1: f32 = 2.0;
pub const SPACE_2: f32 = 4.0;
pub const SPACE_3: f32 = 6.0;
pub const SPACE_4: f32 = 8.0;
pub const SPACE_5: f32 = 12.0;
pub const SPACE_6: f32 = 16.0;

/// Micro caps: section captions, unit suffixes.
pub const FONT_TINY: f32 = 9.0;
/// Default control text.
pub const FONT_UI: f32 = 11.0;
pub const FONT_LABEL: f32 = 11.0;
pub const FONT_TITLE: f32 = 12.0;
/// Large numeric readouts (timecode).
pub const FONT_DISPLAY: f32 = 15.0;

/// Usable control area, excluding vertical padding.
pub const CONTROL_CONTENT_HEIGHT: f32 = 20.0;
/// Outer height: 20px content plus 2px padding on each side.
pub const ROW_HEIGHT: f32 = CONTROL_CONTENT_HEIGHT + SPACE_1 * 2.0;
pub const INPUT_HEIGHT: f32 = ROW_HEIGHT;
pub const HEADER_HEIGHT: f32 = ROW_HEIGHT;
pub const TOOLBAR_HEIGHT: f32 = ROW_HEIGHT;
/// Icon size inside compact controls.
pub const ICON_SIZE: f32 = 14.0;
pub const ICON_SIZE_SMALL: f32 = 12.0;

pub const PAD_BUTTON: [u16; 2] = [2, 8];
pub const PAD_INPUT: [u16; 2] = [2, 6];
pub const PAD_HEADER: [u16; 2] = [0, 6];
pub const PAD_PANEL: [u16; 2] = [8, 10];
pub const PAD_TOOLBAR: [u16; 2] = [4, 8];

pub const TREE_INDENT: f32 = 14.0;

// ---------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------

/// Micro-interaction duration (hover, toggles).
pub const MOTION_FAST_MS: u64 = 140;
/// Standard transition duration (sliding pills, focus rings).
pub const MOTION_STANDARD_MS: u64 = 240;
/// Emphasized transition duration (HUDs, toasts).
pub const MOTION_SLOW_MS: u64 = 380;

// ---------------------------------------------------------------------------
// Iced Theme
// ---------------------------------------------------------------------------

/// Graphite palette for stock iced widgets (buttons, pick lists, etc.).
pub const PALETTE: Palette = Palette {
	background: BACKGROUND_COLOR,
	text: TEXT_PRIMARY_COLOR,
	primary: ACCENT_COLOR,
	success: SUCCESS_COLOR,
	warning: WARNING_COLOR,
	danger: DANGER_COLOR,
};

/// Application theme — Graphite.
pub fn app_theme() -> Theme {
	Theme::custom("Graphite", PALETTE)
}

/// Linear blend between two colors (`t = 0` → `a`).
pub fn mix(a: Color, b: Color, t: f32) -> Color {
	let t = t.clamp(0.0, 1.0);
	Color {
		r: a.r + (b.r - a.r) * t,
		g: a.g + (b.g - a.g) * t,
		b: a.b + (b.b - a.b) * t,
		a: a.a + (b.a - a.a) * t,
	}
}

/// Label color for an index (wraps around the palette).
pub fn label_color(index: usize) -> Color {
	LABEL_COLORS[index % LABEL_COLORS.len()]
}
