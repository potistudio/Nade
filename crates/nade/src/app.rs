//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use browser_panel::{ProjectPaneMessage, ProjectPaneState};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use iced::keyboard;
use iced::widget::container;
use iced::{Element, Length, Size, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;
use timeline_panel::{TimelineClip, TimelineInteraction, TimelineMessage, TimelineModel, TimelineTrack};

use core::{CoreEffect, FrameData, Model, Msg, update};
use domain::{AssetType, Project};

use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;
use crate::panels;
use crate::services::render_service::{RenderConnection, build_render_stream};

// =============================================================================
// ウィンドウイベント
// =============================================================================

fn on_window_resize(event: iced::Event, _status: iced::event::Status, _id: iced::window::Id) -> Option<Message> {
	if let iced::Event::Window(iced::window::Event::Resized(size)) = event {
		Some(Message::PanelSystem(PanelSystemMessage::WindowResized(size)))
	} else {
		None
	}
}

// =============================================================================
// テーマ設定
// =============================================================================

/// アプリケーションのテーマを返す
pub fn theme(_state: &NadeApp) -> Theme {
	constants::style::app_theme()
}

// =============================================================================
// アプリケーション状態
// =============================================================================

/// タイムラインパネル状態
struct TimelinePanelState {
	state: TimelineInteraction,
	model: TimelineModel,
}

impl Default for TimelinePanelState {
	fn default() -> Self {
		let mut model = TimelineModel::new();
		let track1 = TimelineTrack::new("Video 1");
		let track2 = TimelineTrack::new("Audio 1");
		let track3 = TimelineTrack::new("Video 2");
		model.add_track(track1);
		model.add_track(track2);
		model.add_track(track3);

		if let Some(track) = model.tracks.get_mut(0) {
			track.add_clip(TimelineClip::new(0, "Clip A", 0.0, 3.0));
			track.add_clip(TimelineClip::new(1, "Clip B", 4.0, 2.5));
			track.add_clip(TimelineClip::new(2, "Clip D", 7.0, 1.0));
		}
		if let Some(track) = model.tracks.get_mut(1) {
			track.add_clip(TimelineClip::new(2, "Audio Clip 1", 0.5, 4.0));
		}
		if let Some(track) = model.tracks.get_mut(2) {
			track.add_clip(TimelineClip::new(3, "Clip C", 2.0, 5.0));
		}

		Self {
			state: TimelineInteraction::new(),
			model,
		}
	}
}

/// Main application state managing the UI, rendering, and project data
pub(super) struct NadeApp {
	/// Project data (assets, metadata, etc.)
	project: Project,

	/// Core preview / playback model
	current_model: Model,

	/// タイムラインパネル
	timeline: TimelinePanelState,

	/// プロジェクトパネル
	project_pane: ProjectPaneState,

	/// パネルシステム（レイアウト・リサイズ管理）
	panel_system: PanelSystem<PanelContent>,

	/// レンダリング結果送信用チャンネル
	render_tx: Sender<FrameData>,
	/// レンダリング結果受信用チャンネル（Subscriptionで使用）
	render_rx: Arc<std::sync::Mutex<Receiver<FrameData>>>,
	/// レンダリングServiceのシャットダウン通知
	render_shutdown_tx: Option<Sender<()>>,
	/// レンダリングServiceのシャットダウン受信機（Serviceへ渡す）
	render_shutdown_rx: Arc<std::sync::Mutex<Receiver<()>>>,

	/// バックグラウンドレンダリング中かどうか
	is_rendering: Arc<AtomicBool>,
	/// FPS計算用カウンタ
	frame_count: u32,
	/// FPS更新の基準時刻
	fps_update_time: Instant,
}

//==== Iced API ================================================================
impl NadeApp {
	pub(super) fn new() -> (Self, Task<Message>) {
		let mut project = Project::default();
		project.create_asset("hogehoge image", AssetType::Image);
		project.create_asset("0001-0004.mov", AssetType::Video);
		project.create_asset("16mm Burn 6.mov", AssetType::Video);

		let panel_system = Self::create_panel_layout();
		let (render_tx, render_rx) = unbounded::<FrameData>();
		let (shutdown_tx, shutdown_rx) = bounded(1);

		let mut app = Self {
			project,
			current_model: Model::default(),
			timeline: TimelinePanelState::default(),
			project_pane: ProjectPaneState::default(),
			panel_system,
			render_tx,
			render_rx: Arc::new(std::sync::Mutex::new(render_rx)),
			render_shutdown_tx: Some(shutdown_tx),
			render_shutdown_rx: Arc::new(std::sync::Mutex::new(shutdown_rx)),
			is_rendering: Arc::new(AtomicBool::new(false)),
			frame_count: 0,
			fps_update_time: Instant::now(),
		};

		// 初期フレームを描画
		app.apply_core_msg(Msg::SetTime(0.0));

		(app, Task::none())
	}

	/// 初期パネルレイアウトを構築
	///
	/// ```text
	/// ┌─────────┬──────────────────┬───────────┐
	/// │ Project │     Preview      │ Inspector │
	/// │         ├──────────────────┤           │
	/// │         │    Timeline      │           │
	/// └─────────┴──────────────────┴───────────┘
	/// ```
	fn create_panel_layout() -> PanelSystem<PanelContent> {
		let mut builder = LayoutBuilder::new();
		let project = builder.area(PanelContent::Project);
		let preview = builder.area(PanelContent::MainPreview);
		let timeline = builder.area(PanelContent::Timeline);
		let inspector = builder.area(PanelContent::Inspector);

		let center = LayoutBuilder::vsplit(preview, timeline, 0.65);
		let main = LayoutBuilder::hsplit(center, inspector, 0.78);
		let layout = LayoutBuilder::hsplit(project, main, 0.18);

		let mut system = PanelSystem::new().with_layout(layout).with_ids(builder.next_area_id());
		// 初期ウィンドウサイズを設定（main.rsのwindow_settingsと合わせる）
		system.update(PanelSystemMessage::<PanelContent, AppPanelMessage>::WindowResized(
			Size::new(1280.0, 720.0),
		));
		system
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
				CoreEffect::RenderFrame { time, width, height } => {
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
			let frame_num = (time * 60.0).max(0.0) as u32;
			let img_buffer = renderer::render_frame(frame_num, width, height);
			let frame_data = FrameData {
				width,
				height,
				pixels: bytes::Bytes::from(img_buffer.into_raw()),
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

		self.frame_count += 1;
		let now = Instant::now();
		let elapsed = now.duration_since(self.fps_update_time).as_secs_f32();
		if elapsed >= 0.5 {
			self.current_model.preview.fps = self.frame_count as f32 / elapsed;
			self.frame_count = 0;
			self.fps_update_time = now;
		}
	}

	pub(super) fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::RenderCompleted(frame) => {
				self.handle_render_completed(frame);
				Task::none()
			}

			Message::TogglePlay => {
				self.apply_core_msg(Msg::TogglePlay);
				log::debug!("Toggling play/pause: {}", self.current_model.preview.is_playing);
				Task::none()
			}

			Message::Tick => {
				if self.current_model.preview.is_playing {
					self.apply_core_msg(Msg::Tick);
				}
				if self.panel_system.is_editor_menu_animating() {
					self.panel_system
						.update(PanelSystemMessage::<PanelContent, AppPanelMessage>::AnimTick);
				}
				Task::none()
			}

			Message::Timeline(msg) => {
				self.apply_timeline_message(msg);
				Task::none()
			}

			Message::PanelSystem(msg) => {
				match &msg {
					PanelSystemMessage::AppMessage(AppPanelMessage::Timeline { message, .. }) => {
						self.apply_timeline_message(message.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::Project(project_msg)) => {
						self.apply_project_message(project_msg.clone());
					}
					_ => {}
				}
				self.panel_system.update(msg);
				Task::none()
			}

			Message::WindowClosed(id) => {
				if let Some(tx) = self.render_shutdown_tx.take()
					&& let Err(e) = tx.send(())
				{
					log::error!("App: Failed to send render shutdown: {:?}", e);
				}
				iced::window::close(id)
			}
		}
	}

	fn apply_timeline_message(&mut self, msg: TimelineMessage) {
		let current_time = self.current_model.preview.time;
		let timeline_update = self
			.timeline
			.state
			.apply_message(&mut self.timeline.model, msg, current_time);

		if let Some(time) = timeline_update.playhead_time {
			self.apply_core_msg(Msg::SetTime(time));
		}

		if let Some((from_index, to_index)) = self.timeline.state.take_reorder()
			&& from_index != to_index
			&& from_index < self.timeline.model.tracks.len()
			&& to_index < self.timeline.model.tracks.len()
		{
			let track = self.timeline.model.tracks.remove(from_index);
			self.timeline.model.tracks.insert(to_index, track);
		}
	}

	fn apply_project_message(&mut self, msg: ProjectPaneMessage) {
		match msg {
			ProjectPaneMessage::ToggleExpand(id) => {
				if self.project_pane.expanded_ids.contains(&id) {
					self.project_pane.expanded_ids.remove(&id);
				} else {
					self.project_pane.expanded_ids.insert(id);
				}
			}
			ProjectPaneMessage::Select(id) => {
				self.project_pane.selected_id = Some(id);
			}
			ProjectPaneMessage::ClearSelection => {
				self.project_pane.selected_id = None;
			}
			ProjectPaneMessage::OpenItem(id) => {
				self.project_pane.selected_id = Some(id);
			}
		}
	}

	pub(super) fn view(&self) -> Element<'_, Message> {
		let panel_view = self
			.panel_system
			.view(|panel_id, content| self.view_panel_content(panel_id, content));

		container(panel_view.map(Message::PanelSystem))
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn view_panel_content<'a>(
		&'a self,
		panel_id: usize,
		content: &PanelContent,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		match content {
			PanelContent::Timeline => panels::timeline::view(
				panel_id,
				&self.timeline.state,
				&self.timeline.model,
				self.current_model.preview.time,
			),
			PanelContent::MainPreview => panels::preview::view(&self.current_model.preview),
			PanelContent::Inspector => panels::inspector::view(self.current_model.preview.selection.as_ref()),
			PanelContent::Project => panels::browser::view(&self.project, &self.project_pane),
		}
	}

	pub(super) fn subscription(&self) -> Subscription<Message> {
		let render_subscription = Subscription::run_with(
			RenderConnection(self.render_rx.clone(), self.render_shutdown_rx.clone()),
			build_render_stream,
		);

		let keyboard_subscription = iced::event::listen_with(|event, _status, _id| {
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

		// 再生中・メニューアニメ中は ~60fps、それ以外は間引き
		let tick_ms = if self.current_model.preview.is_playing || self.panel_system.is_editor_menu_animating() {
			16
		} else {
			50
		};
		let tick = iced::time::every(std::time::Duration::from_millis(tick_ms)).map(|_| Message::Tick);
		let resize = iced::event::listen_with(on_window_resize);

		Subscription::batch([render_subscription, keyboard_subscription, tick, resize])
	}
}
