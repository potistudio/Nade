//! Colors and layout constants for the timeline UI.
//!
//! Palette aligns with `crate::style` (cool dark).

use iced::Color;

use crate::style;

// Layout constants
pub const TRACK_HEIGHT: f32 = 18.0;
pub const TRACK_PADDING: f32 = 1.0;
pub const TRACK_LABEL_WIDTH: f32 = 120.0;
pub const TRACK_HANDLE_WIDTH: f32 = 8.0;
pub const TRACK_CTRL_BTN_SIZE: f32 = 14.0;
pub const TRACK_CTRL_BTN_GAP: f32 = 1.0;
pub const RULER_HEIGHT: f32 = 24.0;
pub const RANGE_SLIDER_HEIGHT: f32 = 16.0;
pub const PIXELS_PER_SECOND: f32 = 100.0;

/// トラックラベル内の可視ボタン矩形
pub fn track_visible_btn_rect(track_left: f32, track_y: f32) -> iced::Rectangle {
	let x = track_left + TRACK_HANDLE_WIDTH + TRACK_CTRL_BTN_GAP;
	let y = track_y + (TRACK_HEIGHT - TRACK_CTRL_BTN_SIZE) * 0.5;
	iced::Rectangle::new(
		iced::Point::new(x, y),
		iced::Size::new(TRACK_CTRL_BTN_SIZE, TRACK_CTRL_BTN_SIZE),
	)
}

/// トラックラベル内のソロボタン矩形
pub fn track_solo_btn_rect(track_left: f32, track_y: f32) -> iced::Rectangle {
	let x = track_left + TRACK_HANDLE_WIDTH + TRACK_CTRL_BTN_GAP * 2.0 + TRACK_CTRL_BTN_SIZE;
	let y = track_y + (TRACK_HEIGHT - TRACK_CTRL_BTN_SIZE) * 0.5;
	iced::Rectangle::new(
		iced::Point::new(x, y),
		iced::Size::new(TRACK_CTRL_BTN_SIZE, TRACK_CTRL_BTN_SIZE),
	)
}

/// トラック名テキスト開始 X
pub fn track_name_x(track_left: f32) -> f32 {
	track_left + TRACK_HANDLE_WIDTH + TRACK_CTRL_BTN_GAP * 3.0 + TRACK_CTRL_BTN_SIZE * 2.0 + 2.0
}

// Zooming limits
pub const MIN_SCALE: f32 = 0.1;
pub const MAX_SCALE: f32 = 10.0;

// Interaction constants
pub const RESIZE_HANDLE_WIDTH: f32 = 8.0;
pub const RESIZE_HANDLE_VISUAL_WIDTH: f32 = 4.0;
pub const SCROLL_MULTIPLIER: f32 = 20.0;
pub const MIN_CLIP_DURATION: f32 = 0.1;
/// Flat clips (no soft rounded cards).
pub const CLIP_CORNER_RADIUS: f32 = 2.0;
pub const TRACK_REORDER_HANDLE_HEIGHT: f32 = 6.0;

// Color constants
pub mod colors {
	use super::{Color, style};

	pub const BACKGROUND: Color = style::BACKGROUND_COLOR;
	pub const RULER_BG: Color = style::HEADER_COLOR;
	pub const TRACK_LABEL_BG: Color = style::PANEL_COLOR;
	pub const TRACK_LABEL_BG_ALT: Color = style::BACKGROUND_SELECTED_COLOR;
	pub const TIMELINE_BG: Color = Color::from_rgb8(24, 30, 38);
	pub const TIMELINE_BG_ALT: Color = Color::from_rgb8(28, 34, 42);
	pub const TIMELINE_BG_ALT2: Color = Color::from_rgb8(32, 40, 50);

	pub const BORDER: Color = style::BORDER_SUBTLE_COLOR;
	pub const BORDER_SUBTLE: Color = style::BORDER_SUBTLE_COLOR;
	pub const BORDER_DARK: Color = style::BORDER_COLOR;

	pub const TEXT_PRIMARY: Color = style::TEXT_PRIMARY_COLOR;
	pub const TEXT_SECONDARY: Color = style::TEXT_SECONDARY_COLOR;
	pub const TEXT_MUTED: Color = style::TEXT_MUTED_COLOR;

	pub const TICK_MAJOR: Color = Color::from_rgb8(132, 148, 168);
	pub const TICK_MINOR: Color = Color::from_rgb8(56, 68, 84);

	/// Playhead — cool cyan.
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

	/// Muted cool clip strip palette.
	pub const CLIP_PALETTE: [Color; 8] = [
		Color::from_rgb8(56, 108, 156),
		Color::from_rgb8(48, 132, 128),
		Color::from_rgb8(72, 120, 168),
		Color::from_rgb8(88, 96, 156),
		Color::from_rgb8(44, 140, 148),
		Color::from_rgb8(108, 88, 140),
		Color::from_rgb8(64, 116, 140),
		Color::from_rgb8(80, 112, 152),
	];
}
