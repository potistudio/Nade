//! Colors and layout constants for the timeline UI.

use iced::Color;

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
pub const CLIP_CORNER_RADIUS: f32 = 6.0;
pub const TRACK_REORDER_HANDLE_HEIGHT: f32 = 6.0;

// Color constants
pub mod colors {
	use super::Color;

	pub const BACKGROUND: Color = Color::from_rgb(0.110, 0.110, 0.114);
	pub const RULER_BG: Color = Color::from_rgb(0.165, 0.165, 0.172);
	pub const TRACK_LABEL_BG: Color = Color::from_rgb(0.148, 0.148, 0.155);
	pub const TRACK_LABEL_BG_ALT: Color = Color::from_rgb(0.132, 0.132, 0.138);
	pub const TIMELINE_BG: Color = Color::from_rgb(0.094, 0.094, 0.098);
	pub const TIMELINE_BG_ALT: Color = Color::from_rgb(0.104, 0.104, 0.108);
	pub const TIMELINE_BG_ALT2: Color = Color::from_rgb(0.114, 0.114, 0.120);

	pub const BORDER: Color = Color::from_rgb(0.220, 0.220, 0.230);
	pub const BORDER_SUBTLE: Color = Color::from_rgb(0.178, 0.178, 0.186);
	pub const BORDER_DARK: Color = Color::from_rgb(0.148, 0.148, 0.155);

	pub const TEXT_PRIMARY: Color = Color::from_rgb(0.898, 0.898, 0.910);
	pub const TEXT_SECONDARY: Color = Color::from_rgb(0.650, 0.650, 0.670);
	pub const TEXT_MUTED: Color = Color::from_rgb(0.380, 0.380, 0.400);

	pub const TICK_MAJOR: Color = Color::from_rgb(0.560, 0.560, 0.580);
	pub const TICK_MINOR: Color = Color::from_rgb(0.290, 0.290, 0.310);

	pub const PLAYHEAD: Color = Color::from_rgb(1.0, 0.322, 0.322);

	pub const CLIP_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.20);
	pub const CLIP_SELECTED_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.92);
	pub const CLIP_HOVERED_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.55);
	pub const CLIP_RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.80);
	pub const CLIP_SHRINKING_OUTLINE: Color = Color::from_rgba(1.0, 0.55, 0.1, 0.90);
	pub const TRACK_REORDER_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.12);

	pub const RANGE_SLIDER_BG: Color = Color::from_rgb(0.130, 0.130, 0.136);
	pub const RANGE_SLIDER_TRACK: Color = Color::from_rgb(0.068, 0.068, 0.072);
	pub const RANGE_SLIDER_HANDLE: Color = Color::from_rgba(0.35, 0.45, 0.75, 0.55);
	pub const RANGE_SLIDER_HANDLE_BORDER: Color = Color::from_rgba(0.55, 0.65, 0.95, 0.85);
}
