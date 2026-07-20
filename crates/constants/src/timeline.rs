//! Colors and layout constants for the timeline UI.
//!
//! Palette aligns with `crate::style` (Blender Dark).

use iced::Color;

use crate::style;

// Layout constants
pub const TRACK_HEIGHT: f32 = 18.0;
pub const TRACK_PADDING: f32 = 1.0;
pub const TRACK_LABEL_WIDTH: f32 = 120.0;
pub const RULER_HEIGHT: f32 = 24.0;
pub const RANGE_SLIDER_HEIGHT: f32 = 16.0;
pub const PIXELS_PER_SECOND: f32 = 100.0;

// Zooming limits
pub const MIN_SCALE: f32 = 0.1;
pub const MAX_SCALE: f32 = 10.0;

// Interaction constants
pub const RESIZE_HANDLE_WIDTH: f32 = 8.0;
pub const RESIZE_HANDLE_VISUAL_WIDTH: f32 = 4.0;
pub const SCROLL_MULTIPLIER: f32 = 20.0;
pub const MIN_CLIP_DURATION: f32 = 0.1;
/// Flat clips — Blender VSE style (no soft rounded cards).
pub const CLIP_CORNER_RADIUS: f32 = 2.0;
pub const TRACK_REORDER_HANDLE_HEIGHT: f32 = 6.0;

// Color constants
pub mod colors {
	use super::{Color, style};

	pub const BACKGROUND: Color = style::BACKGROUND_COLOR;
	pub const RULER_BG: Color = style::HEADER_COLOR;
	pub const TRACK_LABEL_BG: Color = style::PANEL_COLOR;
	pub const TRACK_LABEL_BG_ALT: Color = style::BACKGROUND_SELECTED_COLOR;
	pub const TIMELINE_BG: Color = Color::from_rgb8(40, 40, 40);
	pub const TIMELINE_BG_ALT: Color = Color::from_rgb8(44, 44, 44);
	pub const TIMELINE_BG_ALT2: Color = Color::from_rgb8(48, 48, 48);

	pub const BORDER: Color = style::BORDER_SUBTLE_COLOR;
	pub const BORDER_SUBTLE: Color = style::BORDER_SUBTLE_COLOR;
	pub const BORDER_DARK: Color = style::BORDER_COLOR;

	pub const TEXT_PRIMARY: Color = style::TEXT_PRIMARY_COLOR;
	pub const TEXT_SECONDARY: Color = style::TEXT_SECONDARY_COLOR;
	pub const TEXT_MUTED: Color = style::TEXT_MUTED_COLOR;

	pub const TICK_MAJOR: Color = Color::from_rgb8(140, 140, 140);
	pub const TICK_MINOR: Color = Color::from_rgb8(72, 72, 72);

	/// Playhead — Blender orange.
	pub const PLAYHEAD: Color = style::ACCENT_COLOR;

	pub const CLIP_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.12);
	pub const CLIP_SELECTED_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.85);
	pub const CLIP_HOVERED_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.40);
	pub const CLIP_RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.70);
	pub const CLIP_SHRINKING_OUTLINE: Color = Color {
		a: 0.90,
		..style::ACCENT_COLOR
	};
	pub const TRACK_REORDER_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.10);

	pub const RANGE_SLIDER_BG: Color = style::PANEL_COLOR;
	pub const RANGE_SLIDER_TRACK: Color = style::WIDGET_COLOR;
	pub const RANGE_SLIDER_HANDLE: Color = Color {
		a: 0.55,
		..style::SELECTION_COLOR
	};
	pub const RANGE_SLIDER_HANDLE_BORDER: Color = Color {
		a: 0.85,
		..style::SELECTION_COLOR
	};

	pub const SELECTION_FILL: Color = Color {
		a: 0.15,
		..style::SELECTION_COLOR
	};
	pub const SELECTION_STROKE: Color = Color {
		a: 0.75,
		..style::SELECTION_COLOR
	};
	pub const SELECTION: Color = style::SELECTION_COLOR;

	/// Muted clip strip palette (Blender VSE-like).
	pub const CLIP_PALETTE: [Color; 8] = [
		Color::from_rgb8(72, 102, 148),
		Color::from_rgb8(72, 128, 96),
		Color::from_rgb8(148, 112, 64),
		Color::from_rgb8(112, 88, 140),
		Color::from_rgb8(64, 128, 128),
		Color::from_rgb8(140, 88, 96),
		Color::from_rgb8(120, 120, 72),
		Color::from_rgb8(88, 104, 140),
	];
}
