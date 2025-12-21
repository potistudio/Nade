//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use iced::keyboard;
use iced::time;
use iced::widget::{Image, column, container, image, row, text};
use iced::{Color, Element, Length, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::sync::Arc;

use timeline_panel::TimelineWidget;

use crossbeam_channel::{Receiver, Sender};
use nade_core::{Model, Msg};

use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;

// =============================================================================
// テーマ設定
// =============================================================================

/// アプリケーションのテーマを返す
pub fn theme(_state: &NadeApp) -> Theme {
	Theme::Dark
}

// =============================================================================
// Core接続
// =============================================================================

/// Coreへの接続状態（Subscription用）
#[derive(Clone)]
pub struct CoreConnection(pub Arc<std::sync::Mutex<Receiver<Model>>>);

impl std::hash::Hash for CoreConnection {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(Arc::as_ptr(&self.0) as usize).hash(state);
	}
}

impl PartialEq for CoreConnection {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

impl Eq for CoreConnection {}

/// Coreストリームの構築
pub fn build_core_stream(
	conn: &CoreConnection,
) -> iced::futures::stream::BoxStream<'static, Message> {
	use iced::futures::{SinkExt, StreamExt};
	use std::time::Duration;

	let rx_mutex = conn.0.clone();

	iced::stream::channel(
		100,
		move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
			let rx = rx_mutex;
			loop {
				let result = tokio::task::spawn_blocking({
					let rx = rx.clone();
					move || {
						if let Ok(guard) = rx.lock() {
							// タイムアウト付きで受信することで、iced終了時にループを抜けられる
							match guard.recv_timeout(Duration::from_millis(100)) {
								Ok(model) => Some(Some(model)),
								Err(crossbeam_channel::RecvTimeoutError::Timeout) => Some(None),
								Err(crossbeam_channel::RecvTimeoutError::Disconnected) => None,
							}
						} else {
							None
						}
					}
				})
				.await
				.ok()
				.flatten();

				match result {
					Some(Some(model)) => {
						// Model受信成功
						if output.send(Message::CoreUpdated(model)).await.is_err() {
							// iced側がチャンネルを閉じた
							break;
						}
					}
					Some(None) => {
						// タイムアウト - 継続（ただしicedがシャットダウン中なら自然に終了する）
						continue;
					}
					None => {
						// チャンネル切断またはロック失敗
						break;
					}
				}
			}
		},
	)
	.boxed()
}

// =============================================================================
// アプリケーション状態
// =============================================================================

/// Nadeアプリケーション
pub struct NadeApp {
	/// パネルシステム
	panel_system: PanelSystem<PanelContent>,
	/// Coreへの送信チャンネル
	core_tx: Sender<Msg>,
	/// Coreからの受信チャンネル (Subscriptionで使用)
	core_rx: Arc<std::sync::Mutex<Receiver<Model>>>,
	/// 現在のモデル（表示用キャッシュ）
	current_model: Model,

	/// タイムラインウィジェット (UI State)
	timeline: TimelineWidget,
	/// ステータスバー (UI State)
	status_bar: status_bar::StatusBar,
}

impl NadeApp {
	/// アプリケーションの初期化
	pub fn new(core_tx: Sender<Msg>, core_rx: Receiver<Model>) -> (Self, Task<Message>) {
		let panel_system = Self::create_panel_layout();

		let app = Self {
			panel_system,
			core_tx,
			core_rx: Arc::new(std::sync::Mutex::new(core_rx)),
			current_model: Model::default(),
			timeline: TimelineWidget::new(),
			status_bar: status_bar::StatusBar::new(),
		};

		// 初期フレームを描画するためのトリガー
		app.core_tx.send(Msg::SetTime(0.0)).ok();

		(app, Task::none())
	}

	/// デフォルトのパネルレイアウトを作成
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
	pub fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::CoreUpdated(model) => {
				self.current_model = model;
				Task::none()
			}
			Message::TimeChanged(value) => {
				self.core_tx.send(Msg::SetTime(value)).ok();
				Task::none()
			}
			Message::TogglePlay => {
				self.core_tx.send(Msg::TogglePlay).ok();
				Task::none()
			}
			Message::Tick => {
				// 再生中のみフレームを進める
				if self.current_model.preview.is_playing {
					self.core_tx.send(Msg::Tick).ok();
				}
				Task::none()
			}
			Message::PanelSystem(msg) => {
				// アプリケーションメッセージのルーティング
				if let PanelSystemMessage::AppMessage(app_msg) = &msg {
					match app_msg {
						AppPanelMessage::Timeline(timeline_msg) => {
							self.timeline.update(timeline_msg.clone());
							// Timelineの変更をCoreに通知する場合
							// self.core_tx.send(Msg::...);\
						}
					}
				}
				// システムメッセージはパネルシステムへ
				self.panel_system.update(msg);
				Task::none()
			}
		}
	}

	/// ビューを生成
	pub fn view(&self) -> Element<'_, Message> {
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
		&'a self,
		content: &PanelContent,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		match content {
			PanelContent::MainPreview => self.view_preview(),
			PanelContent::Timeline => self.view_timeline(),
			PanelContent::Properties => self.view_properties(),
			PanelContent::Composition => self.view_composition(),
			PanelContent::Console => self.view_console(),
		}
	}

	/// プレビューパネルのビュー
	fn view_preview<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		let preview = &self.current_model.preview;

		let content = if let Some(frame) = &preview.frame {
			// bytes::Bytes を使用してコピーを回避
			let handle = image::Handle::from_rgba(frame.width, frame.height, frame.pixels.clone());

			let img = Image::new(handle)
				.content_fit(iced::ContentFit::Contain)
				.width(Length::Fill)
				.height(Length::Fill);

			container(img).width(Length::Fill).height(Length::Fill)
		} else {
			container(text("No Signal").color(Color::WHITE))
				.width(Length::Fill)
				.height(Length::Fill)
				.center_x(Length::Fill)
				.center_y(Length::Fill)
		};

		let fps_text = text(format!("{:.1} FPS", preview.fps))
			.size(12)
			.color(Color::from_rgb(0.4, 0.8, 1.0));

		let time_text = text(format!("Time: {:.2}s", preview.time))
			.size(12)
			.color(Color::from_rgb(0.8, 0.8, 0.8));

		column![
			row![fps_text, text(" | ").size(12), time_text].spacing(5),
			content,
		]
		.spacing(5)
		.padding(5)
		.into()
	}

	/// タイムラインパネルのビュー
	fn view_timeline<'a>(
		&'a self,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		self.timeline
			.view()
			.map(AppPanelMessage::Timeline)
			.map(PanelSystemMessage::AppMessage)
	}

	/// プロパティパネルのビュー
	fn view_properties<'a>(
		&self,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
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
	fn view_composition<'a>(
		&self,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
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
	fn view_console<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
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
	pub fn subscription(&self) -> Subscription<Message> {
		let core_rx = self.core_rx.clone();
		let core_subscription = Subscription::run_with(CoreConnection(core_rx), build_core_stream);

		// キーボードサブスクリプション：スペースキーで再生/一時停止
		let keyboard_subscription: Subscription<Message> =
			iced::event::listen_with(|event, _status, _id| {
				if let iced::Event::Keyboard(keyboard::Event::KeyPressed {
					key: keyboard::Key::Named(keyboard::key::Named::Space),
					..
				}) = event
				{
					Some(Message::TogglePlay)
				} else {
					None
				}
			});

		// 再生中のみTickを送信 (60fps)
		let tick_subscription: Subscription<Message> = if self.current_model.preview.is_playing {
			time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
		} else {
			Subscription::none()
		};

		Subscription::batch([core_subscription, keyboard_subscription, tick_subscription])
	}
}

// =============================================================================
// UI実行
// =============================================================================

/// UIを実行する
pub fn run_ui(ui_tx: Sender<Msg>, ui_rx: Receiver<Model>) -> iced::Result {
	iced::application(
		move || NadeApp::new(ui_tx.clone(), ui_rx.clone()),
		NadeApp::update,
		NadeApp::view,
	)
	.subscription(NadeApp::subscription)
	.theme(theme)
	.title("Nade")
	.window_size((1600.0, 900.0))
	.run()
}
