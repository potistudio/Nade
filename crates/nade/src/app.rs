//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use asset_browser::{AssetBrowserMessage, AssetBrowserState};
use browser_panel::{ObjectId, ProjectPaneMessage, ProjectPaneState};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use iced::keyboard;
use iced::widget::container;
use iced::{Element, Length, Size, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;
use timeline_panel::{TimelineClip, TimelineInteraction, TimelineMessage, TimelineModel, TimelineTrack};

use core::{AssetId, CompositionId, CoreEffect, FrameData, Model, Msg, update};
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

	/// オブジェクトブラウザ
	project_pane: ProjectPaneState,

	/// アセットブラウザ
	asset_browser: AssetBrowserState,

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
		let (project, main_comp, assets_folder) = Self::create_sample_project();
		let mut project_pane = ProjectPaneState::default();
		project_pane.expanded_ids.insert(main_comp);
		project_pane.selected_id = Some(ObjectId::Composition(main_comp));

		let mut asset_browser = AssetBrowserState::default();
		asset_browser.expanded_ids.insert(assets_folder);

		let panel_system = Self::create_panel_layout();
		let (render_tx, render_rx) = unbounded::<FrameData>();
		let (shutdown_tx, shutdown_rx) = bounded(1);

		let mut app = Self {
			project,
			current_model: Model::default(),
			timeline: TimelinePanelState::default(),
			project_pane,
			asset_browser,
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

	fn create_sample_project() -> (Project, CompositionId, AssetId) {
		let mut project = Project::default();

		let assets = project.create_folder("Assets", None);
		project.create_asset_in(Some(assets), "Background.png", AssetType::Image);
		project.create_asset_in(Some(assets), "0001-0004.mov", AssetType::Video);
		project.create_asset_in(Some(assets), "BGM.mp3", AssetType::Audio);

		let main = project.create_composition("Main Composition", 1920, 1080, 60.0);
		let scene = project.create_composition("Scene 1", 1280, 720, 30.0);

		let node_a = project.create_node();
		let node_b = project.create_node();
		if let Some(comp) = project.composition_mut(&main) {
			comp.add_instance_named(node_a, "Rectangle");
			comp.add_instance_named(node_b, "Title");
		}

		let node_c = project.create_node();
		if let Some(comp) = project.composition_mut(&scene) {
			comp.add_instance_named(node_c, "Background");
		}

		(project, main, assets)
	}

	/// 初期パネルレイアウトを構築
	///
	/// ```text
	/// ┌─────────┬──────────────────┬───────────┐
	/// │ Objects │     Preview      │ Inspector │
	/// ├─────────┼──────────────────┤           │
	/// │ Assets  │    Timeline      │           │
	/// └─────────┴──────────────────┴───────────┘
	/// ```
	fn create_panel_layout() -> PanelSystem<PanelContent> {
		let mut builder = LayoutBuilder::new();
		let objects = builder.area(PanelContent::Project);
		let assets = builder.area(PanelContent::Assets);
		let preview = builder.area(PanelContent::MainPreview);
		let timeline = builder.area(PanelContent::Timeline);
		let inspector = builder.area(PanelContent::Inspector);

		let left = LayoutBuilder::vsplit(objects, assets, 0.55);
		let center = LayoutBuilder::vsplit(preview, timeline, 0.65);
		let main = LayoutBuilder::hsplit(center, inspector, 0.78);
		let layout = LayoutBuilder::hsplit(left, main, 0.18);

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
				let mut task = Task::none();
				match &msg {
					PanelSystemMessage::AppMessage(AppPanelMessage::Timeline { message, .. }) => {
						self.apply_timeline_message(message.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::Project(project_msg)) => {
						self.apply_project_message(project_msg.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::Assets(assets_msg)) => {
						task = self.apply_assets_message(assets_msg.clone());
					}
					_ => {}
				}
				self.panel_system.update(msg);
				task
			}

			Message::MediaImported(path) => {
				if let Some(path) = path {
					self.import_media_path(path);
				}
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
				self.toggle_expand(id);
			}
			ProjectPaneMessage::Select(id) => {
				self.project_pane.selected_id = Some(id);
			}
			ProjectPaneMessage::ClearSelection => {
				self.project_pane.selected_id = None;
			}
			ProjectPaneMessage::OpenItem(id) => {
				self.open_item(id);
			}
			ProjectPaneMessage::NewComposition => {
				self.create_composition();
			}
			ProjectPaneMessage::AddObject => {
				self.add_object_under_selection();
			}
		}
	}

	fn toggle_expand(&mut self, id: CompositionId) {
		if self.project_pane.expanded_ids.contains(&id) {
			self.project_pane.expanded_ids.remove(&id);
		} else {
			self.project_pane.expanded_ids.insert(id);
		}
	}

	fn open_item(&mut self, id: ObjectId) {
		self.project_pane.selected_id = Some(id);
		if let ObjectId::Composition(composition_id) = id {
			self.project_pane.expanded_ids.insert(composition_id);
		}
	}

	fn create_composition(&mut self) {
		let name = unique_composition_name(&self.project);
		let id = self.project.create_composition(name, 1920, 1080, 60.0);
		self.project_pane.expanded_ids.insert(id);
		self.project_pane.selected_id = Some(ObjectId::Composition(id));
	}

	fn add_object_under_selection(&mut self) {
		let Some(composition_id) = self.target_composition() else {
			log::debug!("Add object ignored: no composition selected");
			return;
		};

		let name = unique_object_name(&self.project, composition_id);
		let node_id = self.project.create_node();
		let instance_id = {
			let Some(comp) = self.project.composition_mut(&composition_id) else {
				return;
			};
			let instance = comp.add_instance_named(node_id, name);
			instance.id()
		};

		self.project_pane.expanded_ids.insert(composition_id);
		self.project_pane.selected_id = Some(ObjectId::Instance {
			composition: composition_id,
			instance: instance_id,
		});
	}

	fn target_composition(&self) -> Option<CompositionId> {
		match self.project_pane.selected_id? {
			ObjectId::Composition(id) => Some(id),
			ObjectId::Instance { composition, .. } => Some(composition),
		}
	}

	fn apply_assets_message(&mut self, msg: AssetBrowserMessage) -> Task<Message> {
		match msg {
			AssetBrowserMessage::ToggleExpand(id) => {
				if self.asset_browser.expanded_ids.contains(&id) {
					self.asset_browser.expanded_ids.remove(&id);
				} else {
					self.asset_browser.expanded_ids.insert(id);
				}
				Task::none()
			}
			AssetBrowserMessage::Select(id) => {
				self.asset_browser.selected_id = Some(id);
				Task::none()
			}
			AssetBrowserMessage::ClearSelection => {
				self.asset_browser.selected_id = None;
				Task::none()
			}
			AssetBrowserMessage::OpenItem(id) => {
				self.asset_browser.selected_id = Some(id);
				if let Some(asset) = self.project.asset(id)
					&& asset.kind() == AssetType::Folder
				{
					self.asset_browser.expanded_ids.insert(id);
				}
				Task::none()
			}
			AssetBrowserMessage::ImportMedia => Self::pick_media_file(),
			AssetBrowserMessage::NewFolder => {
				self.create_asset_folder();
				Task::none()
			}
			AssetBrowserMessage::AddToTimeline => {
				self.add_selected_asset_to_timeline();
				Task::none()
			}
		}
	}

	fn pick_media_file() -> Task<Message> {
		Task::perform(
			async {
				rfd::AsyncFileDialog::new()
					.set_title("Import Media")
					.add_filter(
						"Media",
						&[
							"png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "mov", "mp4", "mkv", "avi",
							"webm", "m4v", "mp3", "wav", "flac", "aac", "ogg", "m4a",
						],
					)
					.pick_file()
					.await
					.map(|handle| handle.path().to_path_buf())
			},
			Message::MediaImported,
		)
	}

	fn import_media_path(&mut self, path: PathBuf) {
		let parent = self.asset_import_parent();
		match self.project.import_media(&path, parent) {
			Some(id) => {
				if let Some(parent_id) = parent {
					self.asset_browser.expanded_ids.insert(parent_id);
				}
				self.asset_browser.selected_id = Some(id);
				log::info!("Imported media: {}", path.display());
			}
			None => {
				log::warn!("Unsupported media type: {}", path.display());
			}
		}
	}

	fn asset_import_parent(&self) -> Option<AssetId> {
		let selected = self.asset_browser.selected_id?;
		let asset = self.project.asset(selected)?;
		match asset.kind() {
			AssetType::Folder => Some(selected),
			_ => asset.parent,
		}
	}

	fn create_asset_folder(&mut self) {
		let parent = self.asset_import_parent();
		let name = unique_asset_folder_name(&self.project, parent);
		let id = self.project.create_folder(name, parent);
		if let Some(parent_id) = parent {
			self.asset_browser.expanded_ids.insert(parent_id);
		}
		self.asset_browser.selected_id = Some(id);
		self.asset_browser.expanded_ids.insert(id);
	}

	fn add_selected_asset_to_timeline(&mut self) {
		let Some(id) = self.asset_browser.selected_id else {
			return;
		};
		let Some(asset) = self.project.asset(id) else {
			return;
		};
		if !asset.kind().is_media() {
			log::debug!("Add to timeline ignored for non-media asset");
			return;
		}

		let name = asset.name.clone();
		let start_time = self.current_model.preview.time;
		let duration = 5.0;
		let clip_id = next_clip_id(&self.timeline.model);
		let track_index = if asset.kind() == AssetType::Audio { 1 } else { 0 };

		if let Some(track) = self.timeline.model.tracks.get_mut(track_index) {
			track.add_clip(TimelineClip::new(clip_id, &name, start_time, duration));
			log::info!("Added `{name}` to timeline at {start_time:.2}s");
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
			PanelContent::Assets => panels::assets::view(&self.project, &self.asset_browser),
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

fn unique_composition_name(project: &Project) -> String {
	let existing: Vec<String> = project
		.compositions()
		.iter()
		.filter_map(|id| project.composition(id).map(|comp| comp.name().to_string()))
		.collect();

	unique_numbered_name("Composition", &existing)
}

fn unique_object_name(project: &Project, composition_id: CompositionId) -> String {
	let existing: Vec<String> = project
		.composition(&composition_id)
		.map(|comp| comp.all_objects().map(|obj| obj.name().to_string()).collect())
		.unwrap_or_default();

	unique_numbered_name("Object", &existing)
}

fn unique_asset_folder_name(project: &Project, parent: Option<AssetId>) -> String {
	let existing: Vec<String> = project
		.assets()
		.iter()
		.filter_map(|id| project.asset(*id))
		.filter(|asset| asset.parent == parent && asset.kind() == AssetType::Folder)
		.map(|asset| asset.name.clone())
		.collect();

	unique_numbered_name("New Folder", &existing)
}

fn next_clip_id(model: &TimelineModel) -> usize {
	model
		.tracks
		.iter()
		.flat_map(|track| track.clips.iter().map(|clip| clip.id))
		.max()
		.map(|id| id + 1)
		.unwrap_or(0)
}

fn unique_numbered_name(base: &str, existing: &[String]) -> String {
	if !existing.iter().any(|name| name == base) {
		return base.to_string();
	}

	for index in 2.. {
		let candidate = format!("{base} {index}");
		if !existing.iter().any(|name| name == &candidate) {
			return candidate;
		}
	}

	unreachable!()
}
