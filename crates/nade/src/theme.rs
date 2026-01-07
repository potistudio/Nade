use iced::Color;

pub const BACKGROUND: Color = Color::from_rgb(0.1, 0.1, 0.1);
pub const TEXT_PRIMARY: Color = Color::WHITE;
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.8, 0.8, 0.8);
pub const TEXT_MUTED: Color = Color::from_rgb(0.6, 0.6, 0.6);

pub mod panel {
	use super::*;

	pub const PREVIEW_BG: Color = Color::from_rgb(0.128, 0.128, 0.128);
	pub const TIMELINE_BG: Color = Color::from_rgb(0.128, 0.128, 0.128);
	pub const PROPERTIES_BG: Color = Color::from_rgb(0.128, 0.128, 0.128);
	pub const COMPOSITION_BG: Color = Color::from_rgb(0.128, 0.128, 0.128);
	pub const CONSOLE_BG: Color = Color::from_rgb(0.128, 0.128, 0.128);
}

pub mod spacing {
	pub const SMALL: f32 = 5.0;
	pub const MEDIUM: f32 = 10.0;
}
