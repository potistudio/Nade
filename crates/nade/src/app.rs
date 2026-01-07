//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use iced::keyboard;
use iced::time;
use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::sync::{Arc, Mutex};

use timeline_pane::TimelineWidget;

use crossbeam_channel::{Receiver, Sender};
use nade_core::{Model, Msg};

use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;
use crate::panels;
use crate::panels::project::{ProjectData, ProjectUiState};
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

	/// Serviceシャットダウン用送信機
	service_shutdown_tx: Option<Sender<()>>,
	/// Serviceシャットダウン用受信機（Serviceへ渡す）
	service_shutdown_rx: Arc<Mutex<Receiver<()>>>,

	/// プロジェクトデータ (Mock)
	project_data: ProjectData,
	/// プロジェクトUI状態
	project_ui: ProjectUiState,
}

impl NadeApp {
	/// アプリケーションの初期化
	pub fn new(core_tx: Sender<Msg>, core_rx: Receiver<Model>) -> (Self, Task<Message>) {
		let panel_system = Self::create_panel_layout();
		let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded(1);

		let app = Self {
			panel_system,
			core_tx,
			core_rx: Arc::new(Mutex::new(core_rx)),
			current_model: Model::default(),
			timeline: TimelineWidget::new(),
			status_bar: status_bar::StatusBar::new(),
			service_shutdown_tx: Some(shutdown_tx),
			service_shutdown_rx: Arc::new(Mutex::new(shutdown_rx)),
			project_data: ProjectData::default(),
			project_ui: ProjectUiState::default(),
		};

		// 初期フレームを描画するためのトリガー
		app.core_tx.send(Msg::SetTime(0.0)).ok();

		(app, Task::none())
	}

	/// デフォルトのパネルレイアウトを作成
	fn create_panel_layout() -> PanelSystem<PanelContent> {
		let mut builder = LayoutBuilder::new();

		// 左側: Project + Properties (縦分割)
		let project = builder.panel("Project", PanelContent::Project);
		let properties = builder.panel("Properties", PanelContent::Properties);
		let left_side = LayoutBuilder::<PanelContent>::vsplit(project, properties, 0.5);

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
				// プレイヘッドを同期（ドラッグ中は同期しない）
				if !self.timeline.is_dragging_playhead() {
					self.timeline.state_mut().playhead_time = self.current_model.preview.time;
				}
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

							// シーク操作をCodeに通知
							if let timeline_pane::TimelineMessage::PlayheadChanged(time) =
								timeline_msg
							{
								self.core_tx.send(Msg::SetTime(*time)).ok();
							}
						}
						AppPanelMessage::Property(prop_msg) => {
							if let Some(selection) = &mut self.current_model.preview.selection {
								match prop_msg {
									crate::message::PropertyMessage::PositionChanged(axis, val) => {
										selection.position[*axis] = *val;
									}
									crate::message::PropertyMessage::RotationChanged(axis, val) => {
										selection.rotation[*axis] = *val;
									}
									crate::message::PropertyMessage::ScaleChanged(axis, val) => {
										selection.scale[*axis] = *val;
									}
									crate::message::PropertyMessage::OpacityChanged(val) => {
										selection.opacity = *val;
									}
								}
								// 変更をCoreに通知
								self.core_tx
									.send(Msg::UpdateTransform(selection.clone()))
									.ok();
							}
						}
						AppPanelMessage::Project(proj_msg) => {
							match proj_msg {
								crate::message::ProjectMessage::ToggleExpand(id) => {
									if self.project_ui.expanded_ids.contains(id) {
										self.project_ui.expanded_ids.remove(id);
									} else {
										self.project_ui.expanded_ids.insert(*id);
									}
								}
								crate::message::ProjectMessage::Select(id) => {
									self.project_ui.selected_id = Some(*id);
								}
								crate::message::ProjectMessage::OpenItem(_id) => {
									// TODO: Implement open logic
								}
							}
						}
					}
				}
				// システムメッセージはパネルシステムへ
				self.panel_system.update(msg);
				Task::none()
			}
			Message::WindowClosed(id) => {
				log::info!("App: WindowClosed event received. ID: {:?}", id);
				// Coreにシャットダウン信号を送信
				log::info!("App: Sending Msg::Shutdown to Core...");
				if let Err(e) = self.core_tx.send(Msg::Shutdown) {
					log::error!("App: Failed to send Msg::Shutdown: {:?}", e);
				} else {
					log::info!("App: Msg::Shutdown sent successfully.");
				}
				// ウィンドウを閉じる
				log::info!("App: Closing window...");
				iced::window::close(id)
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
			PanelContent::Properties => {
				panels::properties::view(self.current_model.preview.selection.as_ref())
			}
			PanelContent::Project => panels::project::view(&self.project_data, &self.project_ui),
			PanelContent::Console => panels::console::view(),
		}
	}

	/// サブスクリプション
	pub fn subscription(&self) -> Subscription<Message> {
		let core_rx = self.core_rx.clone();
		let shutdown_rx = self.service_shutdown_rx.clone();
		let core_subscription =
			Subscription::run_with(CoreConnection(core_rx, shutdown_rx), build_core_stream);

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

		// ウィンドウイベントの監視
		let window_subscription = iced::event::listen_with(|event, _status, id| {
			if let iced::Event::Window(iced::window::Event::CloseRequested) = event {
				Some(Message::WindowClosed(id))
			} else {
				None
			}
		});

		Subscription::batch([
			core_subscription,
			keyboard_subscription,
			tick_subscription,
			window_subscription,
		])
	}
}

// =============================================================================
// UI実行
// =============================================================================

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
	.exit_on_close_request(false)
	.run()
}
