//! # Nade - Entry Point
//!
//! Nadeアプリケーションのエントリーポイント。
//! パネルシステムを使用したマルチペインUIを提供します。

mod composition;
mod encoder;
mod renderer;

use iced::widget::{Image, column, container, image, row, text};
use iced::{Color, Element, Length, Subscription, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::time::Instant;
use timeline_widget::{TimelineMessage, TimelineWidget};

// =============================================================================
// テーマ設定
// =============================================================================

/// アプリケーションのテーマを返す
fn theme(_state: &NadeApp) -> Theme {
	Theme::Dark
}

// =============================================================================
// パネルコンテンツ定義
// =============================================================================

/// パネルに表示するコンテンツの種類
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelContent {
	/// メインプレビュー（ピクセルビューア）
	MainPreview,
	/// タイムライン
	Timeline,
	/// プロパティ（パラメータ編集）
	Properties,
	/// コンポジション（レイヤー/エフェクトツリー）
	Composition,
	/// コンソール（ログ出力）
	Console,
}

impl PanelContent {
	/// パネルの色を返す（識別用）
	pub fn color(&self) -> Color {
		match self {
			PanelContent::MainPreview => Color::from_rgb(0.2, 0.3, 0.4),
			PanelContent::Timeline => Color::from_rgb(0.3, 0.2, 0.4),
			PanelContent::Properties => Color::from_rgb(0.2, 0.4, 0.3),
			PanelContent::Composition => Color::from_rgb(0.4, 0.3, 0.2),
			PanelContent::Console => Color::from_rgb(0.15, 0.15, 0.15),
		}
	}
}

// =============================================================================
// メッセージ定義
// =============================================================================

/// アプリケーションメッセージ
#[derive(Debug, Clone)]
pub enum Message {
	/// フレームティック
	Tick(Instant),
	/// タイムスライダー変更
	TimeChanged(f32),
	/// パネルシステムメッセージ
	PanelSystem(PanelSystemMessage<PanelContent>),
	/// タイムラインメッセージ
	Timeline(TimelineMessage),
}

// =============================================================================
// ピクセルビューア状態
// =============================================================================

/// ピクセルバッファ管理
#[derive(Debug)]
struct PreviewState {
	width: u32,
	height: u32,
	pixels: Vec<u8>,
	time: f32,
	last_frame: Option<Instant>,
	fps: f32,
}

impl Default for PreviewState {
	fn default() -> Self {
		let width = 320;
		let height = 240;
		let pixels = vec![0u8; (width * height * 4) as usize];
		Self {
			width,
			height,
			pixels,
			time: 0.0,
			last_frame: None,
			fps: 0.0,
		}
	}
}

impl PreviewState {
	fn update_tick(&mut self, now: Instant) {
		let dt = if let Some(last) = self.last_frame {
			now.duration_since(last).as_secs_f32()
		} else {
			0.016
		};
		self.last_frame = Some(now);

		if dt > 0.0 {
			let current_fps = 1.0 / dt;
			self.fps = self.fps * 0.9 + current_fps * 0.1;
		}

		self.time += dt;
		self.update_pixels();
	}

	fn set_time(&mut self, value: f32) {
		self.time = value;
		self.update_pixels();
	}

	fn update_pixels(&mut self) {
		let frame_num = (self.time * 60.0) as u32;
		let rgb_image = renderer::render_frame(frame_num, self.width, self.height);
		let rgb_data = rgb_image.into_raw();

		let pixel_count = (self.width * self.height) as usize;
		for i in 0..pixel_count {
			let src_idx = i * 3;
			let dst_idx = i * 4;
			self.pixels[dst_idx] = rgb_data[src_idx];
			self.pixels[dst_idx + 1] = rgb_data[src_idx + 1];
			self.pixels[dst_idx + 2] = rgb_data[src_idx + 2];
			self.pixels[dst_idx + 3] = 255;
		}
	}
}

// =============================================================================
// アプリケーション状態
// =============================================================================

/// Nadeアプリケーション
#[derive(Debug)]
struct NadeApp {
	/// パネルシステム
	panel_system: PanelSystem<PanelContent>,
	/// プレビュー状態
	preview: PreviewState,
	/// タイムラインウィジェット
	timeline: TimelineWidget,
	/// ステータスバー
	status_bar: status_bar::StatusBar,
}

impl Default for NadeApp {
	fn default() -> Self {
		let panel_system = Self::create_panel_layout();

		Self {
			panel_system,
			preview: PreviewState::default(),
			timeline: TimelineWidget::new(),
			status_bar: status_bar::StatusBar::new(),
		}
	}
}

impl NadeApp {
	/// デフォルトのパネルレイアウトを作成
	///
	/// ```text
	/// ┌─────────────┬────────────────────────────┐
	/// │ Composition │                            │
	/// │             │      Main Preview          │
	/// ├─────────────┤                            │
	/// │ Properties  ├────────────────────────────┤
	/// │             │        Timeline            │
	/// └─────────────┴────────────────────────────┘
	/// ```
	fn create_panel_layout() -> PanelSystem<PanelContent> {
		let mut builder = LayoutBuilder::new();

		// 左側: Composition + Properties (縦分割)
		let composition = builder.panel("Composition", PanelContent::Composition);
		let properties = builder.panel("Properties", PanelContent::Properties);
		let left_side = LayoutBuilder::<PanelContent>::vsplit(composition, properties, 0.5);

		// 右側: Preview + Timeline (縦分割)
		let preview = builder.panel("Preview", PanelContent::MainPreview);
		let timeline = builder.panel("Timeline", PanelContent::Timeline);
		let right_side = LayoutBuilder::<PanelContent>::vsplit(preview, timeline, 0.65);

		// メインレイアウト: 左 | 右 (水平分割)
		let layout = LayoutBuilder::<PanelContent>::hsplit(left_side, right_side, 0.25);

		PanelSystem::new().with_layout(layout)
	}

	/// メッセージを処理
	fn update(&mut self, message: Message) {
		match message {
			Message::Tick(now) => {
				self.preview.update_tick(now);
			}
			Message::TimeChanged(value) => {
				self.preview.set_time(value);
			}
			Message::PanelSystem(msg) => {
				self.panel_system.update(msg);
			}
			Message::Timeline(msg) => {
				self.timeline.update(msg);
			}
		}
	}

	/// ビューを生成
	fn view(&self) -> Element<'_, Message> {
		let panel_view = self
			.panel_system
			.view(|content| self.view_panel_content(content));

		let main_layout = column![
			container(panel_view.map(Message::PanelSystem))
				.width(Length::Fill)
				.height(Length::Fill),
			container(self.status_bar.view())
				.width(Length::Fill)
				.style(|_theme| container::Style {
					background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
					..Default::default()
				}),
		]
		.width(Length::Fill)
		.height(Length::Fill);

		main_layout.into()
	}

	/// パネルコンテンツをレンダリング
	fn view_panel_content<'a>(
		&self,
		content: &PanelContent,
	) -> Element<'a, PanelSystemMessage<PanelContent>> {
		match content {
			PanelContent::MainPreview => self.view_preview(),
			PanelContent::Timeline => self.view_timeline(),
			PanelContent::Properties => self.view_properties(),
			PanelContent::Composition => self.view_composition(),
			PanelContent::Console => self.view_console(),
		}
	}

	/// プレビューパネルのビュー
	fn view_preview<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent>> {
		let handle = image::Handle::from_rgba(
			self.preview.width,
			self.preview.height,
			self.preview.pixels.clone(),
		);

		let fps_text = text(format!("{:.1} FPS", self.preview.fps))
			.size(12)
			.color(Color::from_rgb(0.4, 0.8, 1.0));

		let time_text = text(format!("Time: {:.2}s", self.preview.time))
			.size(12)
			.color(Color::from_rgb(0.8, 0.8, 0.8));

		let img = Image::new(handle)
			.content_fit(iced::ContentFit::Contain)
			.width(Length::Fill)
			.height(Length::Fill);

		column![
			row![fps_text, text(" | ").size(12), time_text].spacing(5),
			container(img).width(Length::Fill).height(Length::Fill),
		]
		.spacing(5)
		.padding(5)
		.into()
	}

	/// タイムラインパネルのビュー
	fn view_timeline<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent>> {
		// Note: For now, show a placeholder since Timeline widget needs its own message handling
		// A full integration would require a more complex message routing system
		let content = column![
			text("Timeline").size(14).color(Color::WHITE),
			text("🎬 Video  |  🎵 Audio  |  ✨ Effects")
				.size(11)
				.color(Color::from_rgb(0.7, 0.7, 0.7)),
		]
		.spacing(5)
		.padding(10);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(PanelContent::Timeline.color().into()),
				..Default::default()
			})
			.into()
	}

	/// プロパティパネルのビュー
	fn view_properties<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent>> {
		let content = column![
			text("Properties").size(14).color(Color::WHITE),
			text("No selection")
				.size(12)
				.color(Color::from_rgb(0.6, 0.6, 0.6)),
		]
		.spacing(10)
		.padding(10);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(PanelContent::Properties.color().into()),
				..Default::default()
			})
			.into()
	}

	/// コンポジションパネルのビュー
	fn view_composition<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent>> {
		let content = column![
			text("Composition").size(14).color(Color::WHITE),
			text("└─ Layer 1")
				.size(12)
				.color(Color::from_rgb(0.8, 0.8, 0.8)),
			text("   └─ Effect: Sine Wave")
				.size(11)
				.color(Color::from_rgb(0.6, 0.6, 0.6)),
		]
		.spacing(5)
		.padding(10);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(PanelContent::Composition.color().into()),
				..Default::default()
			})
			.into()
	}

	/// コンソールパネルのビュー
	fn view_console<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent>> {
		let content = column![
			text("Console").size(14).color(Color::WHITE),
			text("[INFO] Application started")
				.size(11)
				.color(Color::from_rgb(0.7, 0.7, 0.7)),
		]
		.spacing(5)
		.padding(10);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(PanelContent::Console.color().into()),
				..Default::default()
			})
			.into()
	}

	/// サブスクリプション
	fn subscription(&self) -> Subscription<Message> {
		iced::window::frames().map(Message::Tick)
	}
}

// =============================================================================
// エントリーポイント
// =============================================================================

/// アプリケーションのエントリーポイント
pub fn main() -> iced::Result {
	#[allow(unsafe_code)]
	unsafe {
		std::env::set_var("RUST_LOG", "debug");
		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}

	env_logger::init();

	iced::application(NadeApp::default, NadeApp::update, NadeApp::view)
		.subscription(NadeApp::subscription)
		.theme(theme)
		.title("Nade")
		.window_size((1600.0, 900.0))
		.run()
}
