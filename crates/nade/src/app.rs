//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use iced::keyboard;
use iced::time;
use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::sync::{Arc, Mutex};

use timeline_panel::TimelineWidget;

use crossbeam_channel::{Receiver, Sender};
use nade_core::{Model, Msg};

use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;
use crate::panels;
use crate::services::core_service::{CoreConnection, build_core_stream};
use crate::theme;

// =============================================================================
// テーマ設定
// =============================================================================

/// アプリケーションのテーマを返す
pub fn theme(_state: &NadeApp) -> Theme {
	Theme::Dark
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
	///
	/// Icedの要件(Sync)を満たすためMutexでラップするが、
	/// 実際の受信ループではロックしない（初期化時のみ）。
	core_rx: Arc<Mutex<Receiver<Model>>>,
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
			core_rx: Arc::new(Mutex::new(core_rx)),
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
					background: Some(theme::BACKGROUND.into()),
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
			PanelContent::MainPreview => panels::preview::view(&self.current_model.preview),
			PanelContent::Timeline => panels::timeline::view(&self.timeline),
			PanelContent::Properties => panels::properties::view(),
			PanelContent::Composition => panels::composition::view(),
			PanelContent::Console => panels::console::view(),
		}
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
