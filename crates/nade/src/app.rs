//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use core::id::InstanceId;
use iced::widget::{column, container, row, text};
use iced::{Element, Length, Subscription, Task, Theme};
use std::collections::HashMap;

use timeline_panel::{TimelineClip, TimelineInteraction, TimelineModel, TimelineTrack, TimelineWidget};

use core::{
	CoreEffect, FrameData, Model, Msg, RectangleObject, TimelineClip as CoreClip, TimelineModel as CoreModel,
	TimelineTrack as CoreTrack,
};
use domain::{AssetType, Composition, Project};

use crate::message::{AppPanelMessage, Message};
use crate::services::render_service::{RenderConnection, build_render_stream};

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

/// タイムラインパネル状態
struct TimelinePanelState {
	state: TimelineInteraction,
	model: TimelineModel,
}

impl Default for TimelinePanelState {
	fn default() -> Self {
		let mut model = TimelineModel::new();
		// サンプルトラックとクリップを追加
		let track1 = TimelineTrack::new("Video 1");
		let track2 = TimelineTrack::new("Audio 1");
		let track3 = TimelineTrack::new("Video 2");
		model.add_track(track1);
		model.add_track(track2);
		model.add_track(track3);

		// サンプルクリップを追加
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

	/// 現在の再生時間
	current_time: f32,

	/// 再生中かどうか
	is_playing: bool,

	/// タイムラインパネル
	timeline: TimelinePanelState,
}

//==== Iced API ================================================================
impl NadeApp {
	/// Initializes the application. Origin point for all state setup and background tasks.
	pub(super) fn new() -> (Self, Task<Message>) {
		//TODO: Generating Sample Project
		let mut project = Project::default();
		project.create_asset("hogehoge image", AssetType::Image);
		project.create_asset("0001-0004.mov", AssetType::Video);
		project.create_asset("16mm Burn 6.mov", AssetType::Video);

		// let panel_system = Self::create_panel_layout();
		// let (render_tx, render_rx) = unbounded::<FrameData>();
		// let (shutdown_tx, shutdown_rx) = bounded(1);
		// let now = Instant::now();
		// let composition = Arc::new(std::sync::Mutex::new(Self::create_sample_composition()));
		// let timeline_panels = {
		// 	let comp = composition
		// 		.lock()
		// 		.expect("composition lock should be available");
		// 	// Self::build_timeline_panel_instances(panel_system.root(), &comp)
		// };

		let app = Self {
			project,
			current_time: 0.0,
			is_playing: false,
			timeline: TimelinePanelState::default(),
		};

		// 初期フレームを描画するためのトリガー
		// app.apply_core_msg(Msg::SetTime(0.0));

		(app, Task::none())
	}

	pub(super) fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::RenderCompleted(frame) => {
				// self.handle_render_completed(frame);

				log::debug!("Render completed: {:?}", frame);
				Task::none()
			}

			Message::TogglePlay => {
				self.is_playing = !self.is_playing;
				log::debug!("Toggling play/pause: {}", self.is_playing);
				Task::none()
			}

			Message::Tick => {
				if self.is_playing {
					self.current_time += 1.0 / 60.0;
				}
				// Decay glow animation
				log::debug!("Tick: {}", self.current_time);
				Task::none()
			}

			Message::Timeline(msg) => {
				let timeline_update =
					self.timeline
						.state
						.apply_message(&mut self.timeline.model, msg, self.current_time);

				if let Some(time) = timeline_update.playhead_time {
					self.current_time = time;
				}

				// Handle pending track reorder (after mouse release)
				if let Some((from_index, to_index)) = self.timeline.state.take_reorder()
					&& from_index != to_index
					&& from_index < self.timeline.model.tracks.len()
					&& to_index < self.timeline.model.tracks.len()
				{
					let track = self.timeline.model.tracks.remove(from_index);
					self.timeline.model.tracks.insert(to_index, track);
				}

				log::debug!("Timeline message: {:?}", timeline_update);
				Task::none()
			}

			Message::PanelSystem(msg) => {
				// アプリケーションメッセージのルーティング
				// if let PanelSystemMessage::AppMessage(app_msg) = &msg {
				// 	match app_msg {
				// 		AppPanelMessage::Timeline { panel_id, message } => {
				// 			self.handle_timeline_message(*panel_id, message.clone());
				// 		}
				// 		AppPanelMessage::Inspector(inspector_msg) => {
				// 			self.handle_inspector_message(inspector_msg.clone());
				// 		}
				// 		AppPanelMessage::Project(proj_msg) => match proj_msg {
				// 			crate::message::ProjectPaneMessage::ToggleExpand(id) => {
				// 				if self.project_ui.expanded_ids.contains(id) {
				// 					self.project_ui.expanded_ids.remove(id);
				// 				} else {
				// 					self.project_ui.expanded_ids.insert(*id);
				// 				}
				// 			}
				// 			crate::message::ProjectPaneMessage::Select(id) => {
				// 				self.project_ui.selected_id = Some(*id);
				// 			}
				// 			crate::message::ProjectPaneMessage::ClearSelection => {
				// 				self.project_ui.selected_id = None;
				// 			}
				// 			crate::message::ProjectPaneMessage::OpenItem(_id) => {}
				// 		},
				// 	}
				// }
				// // システムメッセージはパネルシステムへ
				// self.panel_system.update(msg);
				// self.reconcile_timeline_panels();

				log::debug!("Panel system message: {:?}", msg);
				Task::none()
			}

			Message::WindowClosed(id) => {
				// log::info!("App: WindowClosed event received. ID: {:?}", id);
				// // レンダリングServiceへシャットダウンを通知
				// if let Some(tx) = self.render_shutdown_tx.take()
				// 	&& let Err(e) = tx.send(())
				// {
				// 	log::error!("App: Failed to send render shutdown: {:?}", e);
				// }
				// // ウィンドウを閉じる
				// log::info!("App: Closing window...");

				log::debug!("Window closed: {:?}", id);
				iced::window::close(id)
			}
		}
	}

	pub(super) fn view(&self) -> Element<'_, Message> {
		let reorder_preview = self.timeline.state.get_reorder_indices();

		TimelineWidget::with_reorder_preview(
			&self.timeline.state,
			&self.timeline.model,
			self.current_time,
			reorder_preview,
		)
		.view_internal()
		.map(Message::Timeline)
	}
	/*
	let panel_view = self
		.panel_system
		.view(|panel_id, content| self.view_panel_content(panel_id, content));

	container(panel_view.map(Message::PanelSystem))
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	*/

	// / パネルコンテンツをレンダリング
	// fn view_panel_content<'a>(
	// 	&'a self,
	// 	panel_id: usize,
	// 	content: &PanelContent,
	// ) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	// 	match content {
	// 		PanelContent::MainPreview => panels::preview::view(&self.global_state.preview),
	// 		PanelContent::Timeline => {
	// 			let timeline_state = self
	// 				.timeline_panels
	// 				.get(&panel_id)
	// 				.expect("timeline panel state should be initialized");
	// 			panels::timeline::view(
	// 				panel_id,
	// 				&timeline_state.state,
	// 				&timeline_state.model,
	// 				self.global_state.preview.time,
	// 			)
	// 		}
	// 		PanelContent::Inspector => inspector_panel::view(
	// 			self.editor_session.graph(),
	// 			self.editor_session.operator_ids(),
	// 		)
	// 		.map(|msg| PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(msg))),
	// 		PanelContent::Project => panels::project::view(&self.project, &self.project_ui),
	// 	}
	// }

	pub(super) fn subscription(&self) -> Subscription<Message> {
		let tick = iced::time::every(std::time::Duration::from_millis(50)).map(|_| Message::Tick);
		let pinch = crate::pinch::pinch_subscription();
		Subscription::batch([tick, pinch])
	}
}
