//! # Nade - Entry Point

mod app;
#[cfg(debug_assertions)]
mod devtools;
mod message;
mod panel_content;
mod panels;
mod services;
mod shell;
mod theme;
mod tool;
mod widgets;
mod workspace;

use app::NadeApp;

const INTER_FONT: &[u8] = include_bytes!("../../../assets/fonts/Inter/Inter_regular.otf");

fn load_app_icon() -> Option<iced::window::Icon> {
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
		size: iced::Size::new(constants::WINDOW_WIDTH, constants::WINDOW_HEIGHT),
		min_size: Some(iced::Size::new(960.0, 600.0)),
		// 透明タイトルバー + フルサイズコンテンツで、ツールバーと地続きの一枚窓にする
		#[cfg(target_os = "macos")]
		platform_specific: iced::window::settings::PlatformSpecific {
			title_hidden: true,
			titlebar_transparent: true,
			fullsize_content_view: true,
		},
		..Default::default()
	};

	iced::application(NadeApp::new, NadeApp::update, NadeApp::view)
		.subscription(NadeApp::subscription)
		.theme(constants::style::app_theme())
		.font(INTER_FONT)
		.default_font(iced::Font::with_name("Inter"))
		.title("Nade")
		.window(window_settings)
		// CloseRequested をアプリ側で受け取り、バックグラウンドスレッドを
		// 止めてから iced::exit() する（true だと購読より先に閉じられハングする）
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

		// fifo = vsync。immediate はティアリングで描画が不安定に見える。
		if std::env::var("ICED_PRESENT_MODE").is_err() {
			std::env::set_var("ICED_PRESENT_MODE", "fifo");
		}
	}

	env_logger::init();

	run_app()
}
