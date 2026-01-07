//! # Nade (Desktop Application)

mod app;

use app::NadeApp;
use nade_constants::{APP_TITLE, WINDOW_HEIGHT, WINDOW_WIDTH};

fn init_logger() {
	#[allow(unsafe_code)]
	unsafe {
		if std::env::var("RUST_LOG").is_err() {
			#[cfg(debug_assertions)]
			std::env::set_var("RUST_LOG", "nade=debug");

			#[cfg(not(debug_assertions))]
			std::env::set_var("RUST_LOG", "nade=error");
		}

		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}

	env_logger::init();
}

/// Entry point for the desktop application
pub fn main() -> iced::Result {
	init_logger();

	iced::application(NadeApp::new, NadeApp::update, NadeApp::view)
		.subscription(NadeApp::subscription)
		.title(APP_TITLE)
		.window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
		.run()
}
