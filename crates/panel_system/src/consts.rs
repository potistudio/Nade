//! パネルシステムの定数と色設定
//!
//! Colors and sizes follow `constants::style` (Blender Dark).

use constants::style;
use iced::Color;

pub use constants::style::HEADER_HEIGHT;

pub const RESIZE_HANDLE_SIZE: f32 = 4.0;
pub const CORNER_SIZE: f32 = 12.0;
pub const CORNER_DRAG_THRESHOLD: f32 = 24.0;
/// 移動ドロップ時、中央この範囲（正規化座標の半幅）は全体移動
pub const MOVE_CENTER_ZONE: f32 = 0.22;

pub mod colors {
	use super::{Color, style};

	pub const HEADER_BG: Color = style::HEADER_COLOR;
	pub const TEXT_SECONDARY: Color = style::TEXT_SECONDARY_COLOR;
	pub const ACCENT: Color = style::SELECTION_COLOR;
	pub const RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);
	pub const RESIZE_HANDLE_HOVER: Color = Color {
		a: 0.80,
		..style::SELECTION_COLOR
	};
	pub const RESIZE_HANDLE_ACTIVE: Color = style::SELECTION_COLOR;
	pub const CORNER: Color = Color::from_rgba(0.55, 0.55, 0.55, 0.85);
	pub const CORNER_ACTIVE: Color = style::SELECTION_COLOR;
	pub const SPLIT_PREVIEW: Color = Color {
		a: 0.22,
		..style::SELECTION_COLOR
	};
}
