//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use iced::keyboard;
use iced::time;
use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;

use timeline_pane::TimelineWidget;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use nade_core::{CoreEffect, FrameData, Model, Msg, update};

use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;
use crate::panels;
use crate::panels::project::{ProjectData, ProjectUiState};
use crate::renderer;
use crate::services::render_service::{RenderConnection, build_render_stream};
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

	/// レンダリング結果送信用チャンネル
	render_tx: Sender<FrameData>,
	/// レンダリング結果受信用チャンネル（Subscriptionで使用）
	render_rx: Arc<std::sync::Mutex<Receiver<FrameData>>>,
	/// レンダリングServiceのシャットダウン通知
	render_shutdown_tx: Option<Sender<()>>,
	/// レンダリングServiceのシャットダウン受信機（Serviceへ渡す）
	render_shutdown_rx: Arc<std::sync::Mutex<Receiver<()>>>,

	/// 現在のモデル（UIスレッドで更新）
	current_model: Model,
	/// バックグラウンドレンダリング中かどうか
	is_rendering: Arc<AtomicBool>,
	/// FPS計算用カウンタ
	frame_count: u32,
	/// FPS更新の基準時刻
	fps_update_time: Instant,

	/// タイムラインウィジェット (UI State)
	timeline: TimelineWidget,

	/// ステータスバー (UI State)
	status_bar: status_bar::StatusBar,

	/// プロジェクトデータ (Mock)
	project_data: ProjectData,
	/// プロジェクトUI状態
	project_ui: ProjectUiState,
}

impl NadeApp {
	/// アプリケーションの初期化
	pub fn new() -> (Self, Task<Message>) {
		let panel_system = Self::create_panel_layout();
		let (render_tx, render_rx) = unbounded::<FrameData>();
		let (shutdown_tx, shutdown_rx) = bounded(1);
		let now = Instant::now();

		let mut app = Self {
			panel_system,
			render_tx,
			render_rx: Arc::new(std::sync::Mutex::new(render_rx)),
			render_shutdown_tx: Some(shutdown_tx),
			render_shutdown_rx: Arc::new(std::sync::Mutex::new(shutdown_rx)),
			current_model: Model::default(),
			is_rendering: Arc::new(AtomicBool::new(false)),
			frame_count: 0,
			fps_update_time: now,
			timeline: TimelineWidget::new(),
			status_bar: status_bar::StatusBar::new(),
			project_data: ProjectData::default(),
			project_ui: ProjectUiState::default(),
		};

		// 初期フレームを描画するためのトリガー
		app.apply_core_msg(Msg::SetTime(0.0));

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

	/// CoreロジックをUIスレッドで適用し、副作用のみを非同期実行する
	fn apply_core_msg(&mut self, msg: Msg) {
		let (next_model, effects) = update(self.current_model.clone(), msg);
		self.current_model = next_model;
		self.handle_effects(effects);
	}

	/// CoreEffectの実行（重い処理のみバックグラウンドへ）
	fn handle_effects(&mut self, effects: Vec<CoreEffect>) {
		for effect in effects {
			match effect {
				CoreEffect::RenderFrame {
					time,
					width,
					height,
				} => {
					if self.is_rendering.load(Ordering::SeqCst) {
						log::trace!("Skipping frame render - previous render still in progress");
						continue;
					}
					self.spawn_render(time, width, height);
				}
			}
		}
	}

	/// レンダリングだけを別スレッドで実行する
	fn spawn_render(&self, time: f32, width: u32, height: u32) {
		self.is_rendering.store(true, Ordering::SeqCst);
		let is_rendering = Arc::clone(&self.is_rendering);
		let render_tx = self.render_tx.clone();

		thread::spawn(move || {
			let frame_num = (time * 60.0) as u32;
			let img_buffer = renderer::render_frame(frame_num, width, height);
			let raw = img_buffer.into_raw();

			let frame_data = FrameData {
				width,
				height,
				pixels: bytes::Bytes::from(raw),
			};

			if render_tx.send(frame_data).is_err() {
				log::info!("Render thread: receiver dropped before frame delivery.");
			}
			is_rendering.store(false, Ordering::SeqCst);
		});
	}

	/// バックグラウンドレンダリング完了を処理する
	fn handle_render_completed(&mut self, frame: FrameData) {
		self.apply_core_msg(Msg::FrameRendered(frame));

		// FPS計算
		self.frame_count += 1;
		let now = Instant::now();
		let elapsed = now.duration_since(self.fps_update_time).as_secs_f32();
		if elapsed >= 0.5 {
			self.current_model.preview.fps = self.frame_count as f32 / elapsed;
			self.frame_count = 0;
			self.fps_update_time = now;
		}
	}

	/// メッセージを処理
	pub fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::RenderCompleted(frame) => {
				self.handle_render_completed(frame);
				// プレイヘッドを同期（ドラッグ中は同期しない）
				if !self.timeline.is_dragging_playhead() {
					self.timeline.state_mut().playhead_time = self.current_model.preview.time;
				}
				Task::none()
			}
			Message::TimeChanged(value) => {
				self.apply_core_msg(Msg::SetTime(value));
				Task::none()
			}
			Message::TogglePlay => {
				self.apply_core_msg(Msg::TogglePlay);
				Task::none()
			}
			Message::Tick => {
				// 再生中のみフレームを進める
				if self.current_model.preview.is_playing {
					self.apply_core_msg(Msg::Tick);
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
								self.apply_core_msg(Msg::SetTime(*time));
							}
						}
						AppPanelMessage::Property(prop_msg) => {
							let updated_selection = if let Some(selection) =
								&mut self.current_model.preview.selection
							{
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
								Some(selection.clone())
							} else {
								None
							};

							if let Some(selection) = updated_selection {
								// 変更をCoreに通知
								self.apply_core_msg(Msg::UpdateTransform(selection));
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
				// レンダリングServiceへシャットダウンを通知
				if let Some(tx) = self.render_shutdown_tx.take() {
					if let Err(e) = tx.send(()) {
						log::error!("App: Failed to send render shutdown: {:?}", e);
					}
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
		let render_rx = self.render_rx.clone();
		let shutdown_rx = self.render_shutdown_rx.clone();
		let render_subscription =
			Subscription::run_with(RenderConnection(render_rx, shutdown_rx), build_render_stream);

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
			render_subscription,
			keyboard_subscription,
			tick_subscription,
			window_subscription,
		])
	}
}

// =============================================================================
// UI実行
// =============================================================================

pub fn run_ui() -> iced::Result {
	iced::application(NadeApp::new, NadeApp::update, NadeApp::view)
	.subscription(NadeApp::subscription)
	.theme(theme)
	.title("Nade")
	.window_size((1600.0, 900.0))
	.exit_on_close_request(false)
	.run()
}
