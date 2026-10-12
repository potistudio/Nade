//! App shell layout constants. Colors and widget styles live in
//! `constants::style` / `constants::widgets`.

/// Height of the transparent title strip that hosts the traffic lights.
/// Other platforms keep their native title bar.
pub const TITLE_BAR_HEIGHT: f32 = if cfg!(target_os = "macos") { 28.0 } else { 0.0 };
/// Height of the unified toolbar under the title strip.
pub const SHELL_TOOLBAR_HEIGHT: f32 = constants::style::ROW_HEIGHT;
/// Height of the status bar at the bottom of the window.
pub const STATUS_BAR_HEIGHT: f32 = constants::style::ROW_HEIGHT;
/// Gutter between the window edge and the panel grid.
pub const SHELL_GUTTER: f32 = 6.0;

/// Segment size of the tool palette (icon-only).
pub const TOOL_SEGMENT: [f32; 2] = [32.0, constants::style::ROW_HEIGHT];

/// How long a toast stays fully visible.
pub const TOAST_HOLD_SECS: f32 = 1.8;

/// Size available to the panel grid inside a window of `window`.
pub fn panel_area(window: iced::Size) -> iced::Size {
	iced::Size::new(
		(window.width - SHELL_GUTTER * 2.0).max(0.0),
		(window.height - TITLE_BAR_HEIGHT - SHELL_TOOLBAR_HEIGHT - STATUS_BAR_HEIGHT).max(0.0),
	)
}
