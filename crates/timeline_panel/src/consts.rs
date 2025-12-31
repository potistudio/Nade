//! タイムラインの定数と色設定

use iced::Color;

// レイアウト
pub const TRACK_HEIGHT: f32 = 18.0;
pub const TRACK_PADDING: f32 = 1.0;
pub const TRACK_LABEL_WIDTH: f32 = 120.0;
pub const RULER_HEIGHT: f32 = 24.0;
pub const PIXELS_PER_SECOND: f32 = 100.0;

// スケール制限
pub const MIN_SCALE: f32 = 0.1;
pub const MAX_SCALE: f32 = 10.0;

// インタラクション
pub const RESIZE_HANDLE_WIDTH: f32 = 8.0;
pub const RESIZE_HANDLE_VISUAL_WIDTH: f32 = 4.0;
pub const SCROLL_MULTIPLIER: f32 = 20.0;
pub const MIN_CLIP_DURATION: f32 = 0.1;
pub const CLIP_CORNER_RADIUS: f32 = 1.0;

// 色
pub mod colors {
	use super::Color;

	pub const BACKGROUND: Color = Color::from_rgb(0.118, 0.118, 0.118);
	pub const RULER_BG: Color = Color::from_rgb(0.176, 0.176, 0.176);
	pub const TRACK_LABEL_BG: Color = Color::from_rgb(0.157, 0.157, 0.157);
	pub const TRACK_LABEL_BG_ALT: Color = Color::from_rgb(0.137, 0.137, 0.137);
	pub const TIMELINE_BG: Color = Color::from_rgb(0.098, 0.098, 0.098);
	pub const TIMELINE_BG_ALT: Color = Color::from_rgb(0.110, 0.110, 0.110);
	pub const TIMELINE_BG_ALT2: Color = Color::from_rgb(0.125, 0.125, 0.125);

	pub const BORDER: Color = Color::from_rgb(0.235, 0.235, 0.235);
	pub const BORDER_SUBTLE: Color = Color::from_rgb(0.196, 0.196, 0.196);
	pub const BORDER_DARK: Color = Color::from_rgb(0.157, 0.157, 0.157);

	pub const TEXT_PRIMARY: Color = Color::from_rgb(0.863, 0.863, 0.863);
	pub const TEXT_SECONDARY: Color = Color::from_rgb(0.706, 0.706, 0.706);
	pub const TEXT_MUTED: Color = Color::from_rgb(0.392, 0.392, 0.392);

	pub const TICK_MAJOR: Color = Color::from_rgb(0.588, 0.588, 0.588);
	pub const TICK_MINOR: Color = Color::from_rgb(0.314, 0.314, 0.314);

	pub const PLAYHEAD: Color = Color::from_rgb(1.0, 0.322, 0.322);

	pub const CLIP_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.2);
	pub const RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.4);
}
