//! Gizmos Demo - Bezier curve editor using Iced canvas
//!
//! Demonstrates interactive bezier handle editing with pan/zoom canvas.

mod gizmos;

use gizmos::GizmosApp;
use iced::Theme;

fn main() -> iced::Result {
	env_logger::init();

	iced::application(GizmosApp::default, GizmosApp::update, GizmosApp::view)
		.theme(theme)
		.title("Demo - Gizmos")
		.window_size((800.0, 600.0))
		.run()
}

fn theme(_state: &GizmosApp) -> Theme {
	constants::style::app_theme()
}
