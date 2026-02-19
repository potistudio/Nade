//! # Nade - Entry Point
//!
//! Nadeアプリケーションのエントリーポイント。

mod app;
mod message;
mod panel_content;
mod panels;
mod services;
mod theme;
mod widgets;

const INTER_FONT: &[u8] = include_bytes!("../../../assets/fonts/Inter/Inter_regular.otf");

fn load_app_icon() -> Option<iced::window::Icon> {
	// アイコンファイルのパス（実行ファイルからの相対パス）
	let icon_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
		.parent()?
		.parent()?
		.join("assets/icons/x1024.png");

	let image = image::open(&icon_path).ok()?.into_rgba8();
	let (width, height) = image.dimensions();
	let rgba = image.into_raw();

	iced::window::icon::from_rgba(rgba, width, height).ok()
}

pub fn run_app() -> iced::Result {
	let window_settings = iced::window::Settings {
		icon: load_app_icon(),
		size: iced::Size::new(1280.0, 720.0),
		..Default::default()
	};

	iced::application(app::NadeApp::new, app::NadeApp::update, app::NadeApp::view)
		.subscription(app::NadeApp::subscription)
		.theme(app::theme)
		.font(INTER_FONT)
		.default_font(iced::Font::with_name("Inter"))
		.title("Nade")
		.window(window_settings)
		.exit_on_close_request(false)
		.run()
}

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
	run_app()
}
