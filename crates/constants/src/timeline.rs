//! Colors and layout constants for the timeline UI.

use iced::Color;

// Layout constants
pub const TRACK_HEIGHT: f32 = 32.0;
pub const TRACK_PADDING: f32 = 3.0;
pub const TRACK_LABEL_WIDTH: f32 = 140.0;
pub const RULER_HEIGHT: f32 = 28.0;
pub const PIXELS_PER_SECOND: f32 = 100.0;

// Zooming limits
pub const MIN_SCALE: f32 = 0.1;
pub const MAX_SCALE: f32 = 10.0;

// Interaction constants
pub const RESIZE_HANDLE_WIDTH: f32 = 8.0;
pub const RESIZE_HANDLE_VISUAL_WIDTH: f32 = 4.0;
pub const SCROLL_MULTIPLIER: f32 = 20.0;
pub const MIN_CLIP_DURATION: f32 = 0.1;
pub const CLIP_CORNER_RADIUS: f32 = 5.0;
pub const CLIP_TEXT_PADDING: f32 = 6.0;
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

	pub const CLIP_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.18);
	pub const CLIP_RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.80);
	pub const TRACK_REORDER_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.12);
}
