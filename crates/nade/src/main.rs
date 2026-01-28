//! # Nade - Entry Point
//!
//! Nadeアプリケーションのエントリーポイント。

mod app;
mod composition;
mod encoder;
mod message;
mod panel_content;
mod panels;
mod renderer;
mod services;
mod theme;
mod widgets;

/// Entry point of the application (desktop)
pub fn main() -> iced::Result {
	// TODO: 環境変数の設定を抽出する
	// NOTE: 複雑化した際に検討する
	#[allow(unsafe_code)]
	unsafe {
		if std::env::var("RUST_LOG").is_err() {
			#[cfg(debug_assertions)]
			std::env::set_var("RUST_LOG", "nade=debug");

			#[cfg(not(debug_assertions))]
			std::env::set_var("RUST_LOG", "nade=info");
		}

		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}

	env_logger::init();

	// UIスレッドを中心にし、重い処理のみをバックグラウンドへ逃がす
	app::run_ui()
}
