//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use browser_panel::{ProjectPaneMessage, ProjectPaneState};
use iced::widget::{container, text};
use iced::{Element, Length, Size, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use timeline_panel::{TimelineClip, TimelineInteraction, TimelineMessage, TimelineModel, TimelineTrack};

use domain::{AssetType, Project};

use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;
use crate::panels;

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

	/// 現在の再生時間
	current_time: f32,

	/// 再生中かどうか
	is_playing: bool,

	/// タイムラインパネル
	timeline: TimelinePanelState,

	/// プロジェクトパネル
	project_pane: ProjectPaneState,

	/// パネルシステム（レイアウト・リサイズ管理）
	panel_system: PanelSystem<PanelContent>,
}

//==== Iced API ================================================================
impl NadeApp {
	pub(super) fn new() -> (Self, Task<Message>) {
		let mut project = Project::default();
		project.create_asset("hogehoge image", AssetType::Image);
		project.create_asset("0001-0004.mov", AssetType::Video);
		project.create_asset("16mm Burn 6.mov", AssetType::Video);

		let panel_system = Self::create_panel_layout();

		let app = Self {
			project,
			current_time: 0.0,
			is_playing: false,
			timeline: TimelinePanelState::default(),
			project_pane: ProjectPaneState::default(),
			panel_system,
		};

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

	pub(super) fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::RenderCompleted(frame) => {
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

			Message::WindowClosed(id) => iced::window::close(id),
		}
	}

	fn apply_timeline_message(&mut self, msg: TimelineMessage) {
		let timeline_update = self
			.timeline
			.state
			.apply_message(&mut self.timeline.model, msg, self.current_time);

		if let Some(time) = timeline_update.playhead_time {
			self.current_time = time;
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
			PanelContent::Timeline => {
				panels::timeline::view(panel_id, &self.timeline.state, &self.timeline.model, self.current_time)
			}
			PanelContent::MainPreview => container(
				text("Preview")
					.size(constants::style::FONT_LABEL)
					.color(constants::style::TEXT_MUTED_COLOR),
			)
			.width(Length::Fill)
			.height(Length::Fill)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.into(),
			PanelContent::Inspector => panels::inspector::view(None),
			PanelContent::Project => panels::browser::view(&self.project, &self.project_pane),
		}
	}

	pub(super) fn subscription(&self) -> Subscription<Message> {
		let tick = iced::time::every(std::time::Duration::from_millis(50)).map(|_| Message::Tick);
		let resize = iced::event::listen_with(on_window_resize);
		let editor_menu = if self.panel_system.is_editor_menu_animating() {
			iced::time::every(std::time::Duration::from_millis(16))
				.map(|_| Message::PanelSystem(PanelSystemMessage::AnimTick))
		} else {
			Subscription::none()
		};
		Subscription::batch([tick, resize, editor_menu])
	}
}
