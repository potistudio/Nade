//! パネルシステムの定数と色設定

use iced::Color;

pub const HEADER_HEIGHT: f32 = 28.0;
pub const RESIZE_HANDLE_SIZE: f32 = 4.0;
pub const CORNER_SIZE: f32 = 14.0;
pub const CORNER_DRAG_THRESHOLD: f32 = 24.0;
/// 移動ドロップ時、中央この範囲（正規化座標の半幅）は全体移動
pub const MOVE_CENTER_ZONE: f32 = 0.22;

pub mod colors {
	use super::Color;

	pub const BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.12);
	pub const PANEL_BG: Color = Color::from_rgb(0.15, 0.15, 0.15);
	pub const HEADER_BG: Color = Color::from_rgb(0.18, 0.18, 0.18);
	pub const BORDER: Color = Color::from_rgb(0.08, 0.08, 0.08);
	pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.6);
	pub const ACCENT: Color = Color::from_rgb(0.3, 0.5, 0.9);
	pub const RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
	pub const RESIZE_HANDLE_HOVER: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.8);
	pub const RESIZE_HANDLE_ACTIVE: Color = Color::from_rgba(0.4, 0.6, 1.0, 1.0);
	pub const CORNER: Color = Color::from_rgba(0.55, 0.55, 0.58, 0.9);
	pub const CORNER_ACTIVE: Color = Color::from_rgb(0.4, 0.6, 1.0);
	pub const SPLIT_PREVIEW: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.25);
}
