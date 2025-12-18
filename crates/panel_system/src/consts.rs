//! パネルシステムの定数と色設定

use iced::Color;

pub const TAB_HEIGHT: f32 = 32.0;
pub const RESIZE_HANDLE_SIZE: f32 = 6.0;
pub const DROP_ZONE_SIZE: f32 = 50.0;
pub const DRAG_THRESHOLD: f32 = 5.0;

pub mod colors {
	use super::Color;

	pub const BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.12);
	pub const PANEL_BG: Color = Color::from_rgb(0.15, 0.15, 0.15);
	pub const TAB_BG: Color = Color::from_rgb(0.18, 0.18, 0.18);
	pub const TAB_ACTIVE: Color = Color::from_rgb(0.25, 0.25, 0.28);
	pub const TAB_HOVER: Color = Color::from_rgb(0.22, 0.22, 0.22);
	pub const TAB_BAR_BG: Color = Color::from_rgb(0.13, 0.13, 0.13);
	pub const BORDER: Color = Color::from_rgb(0.08, 0.08, 0.08);
	pub const TEXT_PRIMARY: Color = Color::from_rgb(0.9, 0.9, 0.9);
	pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.6);
	pub const ACCENT: Color = Color::from_rgb(0.3, 0.5, 0.9);
	pub const RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.1);
	pub const RESIZE_HANDLE_HOVER: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.8);
	pub const RESIZE_HANDLE_ACTIVE: Color = Color::from_rgba(0.4, 0.6, 1.0, 1.0);
	pub const DROP_ZONE_HIGHLIGHT: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.3);
	pub const DROP_ZONE_BORDER: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.8);
	pub const FLOATING_PANEL_BG: Color = Color::from_rgba(0.2, 0.2, 0.22, 0.95);
}
