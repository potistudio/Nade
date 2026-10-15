//! パネルシステムの定数と色設定
//!
//! Colors and sizes follow `constants::style` (Graphite).

use constants::style;
use iced::Color;

/// Equal padding on all sides and spacing between panel header controls.
pub const HEADER_PADDING: f32 = style::SPACE_1;
/// 20px controls with 2px padding on all sides give a 24px header.
pub const HEADER_HEIGHT: f32 = style::CONTROL_CONTENT_HEIGHT + HEADER_PADDING * 2.0;

/// Visible gutter between tiled panels.
pub const PANEL_GAP: f32 = 6.0;

/// Width of the outline around panels (inside).
pub const PANEL_OUTLINE_WIDTH: f32 = 1.0;

/// Total resize hit target across the gutter (extends over panel edges; does not widen the gap).
pub const RESIZE_HIT_SIZE: f32 = 12.0;
/// Minimum pane size along a split axis (px).
pub const MIN_PANEL_SIZE: f32 = 100.0;
pub const CORNER_SIZE: f32 = 12.0;
pub const CORNER_DRAG_THRESHOLD: f32 = 24.0;
/// 移動ドロップ時、中央この範囲（正規化座標の半幅）は全体移動
pub const MOVE_CENTER_ZONE: f32 = 0.22;

pub mod colors {
	use super::{Color, style};

	pub const PANEL_OUTLINE_COLOR: Color = style::BORDER_SUBTLE_COLOR;
	pub const HEADER_BG: Color = style::HEADER_COLOR;
	pub const RESIZE_HANDLE: Color = style::BACKGROUND_COLOR;
	pub const CORNER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);
	pub const CORNER_ACTIVE: Color = style::ACCENT_COLOR;
	pub const SPLIT_PREVIEW: Color = Color {
		a: 0.16,
		..style::ACCENT_COLOR
	};
	pub const SPLIT_PREVIEW_BORDER: Color = Color {
		a: 0.7,
		..style::ACCENT_COLOR
	};
	pub const HEADER_SEPARATOR: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.28);
	pub const FOCUS_RING: Color = Color {
		a: 0.75,
		..style::ACCENT_COLOR
	};
	pub const GUTTER_ACTIVE: Color = Color {
		a: 0.85,
		..style::ACCENT_COLOR
	};
}
