//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use asset_browser::{AssetBrowserMessage, AssetBrowserState};
use browser_panel::{ObjectId, ProjectPaneMessage, ProjectPaneState};
use constants::icons::Icon;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use curve_editor_panel::{CurveEditorMessage, CurveEditorState};
use iced::animation::Easing;
use iced::keyboard;
use iced::time::{Duration, Instant};
use iced::widget::{column, container, stack};
use iced::{Animation, Element, Length, Size, Subscription, Task};
use inspector_panel::{InspectorMessage, PropertyGroup};
use panel_system::{PanelSystem, PanelSystemMessage};
use std::path::PathBuf;
use std::sync::Arc;
use timeline_panel::{TimelineClip, TimelineInteraction, TimelineMessage, TimelineModel, TimelineTrack};

use core::{AssetId, CompositionId, CoreEffect, FrameData, InstanceId, Model, Msg, SceneObjectId, TextObject, update};
use domain::{AssetType, Instance, InstanceContent, Project, TransformAnimation, TransformProperty};

use crate::message::{AppPanelMessage, Message, PreviewMessage, ShellMessage, TransportMessage};
use crate::panel_content::PanelContent;
use crate::panels;
use crate::services::render_service::{RenderConnection, build_render_stream};
use crate::services::render_worker::{RenderJob, RenderWorker};
use crate::shell::{self, toast::Toast};
use crate::theme::{SHELL_GUTTER, STATUS_BAR_HEIGHT, TITLE_BAR_HEIGHT, panel_area};
use crate::tool::Tool;
use crate::viewer::ViewerSettings;
use crate::widgets::viewer_overlay::LayerBox;
use crate::widgets::viewport::Viewport;
use crate::workspace::Workspace;

// =============================================================================
// ウィンドウ・キーボードイベント
// =============================================================================

/// ウィンドウイベントをアプリのメッセージへ写像する（キャプチャ済みのキーは無視）
fn on_window_event(event: iced::Event, status: iced::event::Status, _id: iced::window::Id) -> Option<Message> {
	match event {
		iced::Event::Window(iced::window::Event::Resized(size)) => Some(Message::WindowResized(size)),
		// 押下（キャプチャ済みを含む）でポインタ下のパネルへフォーカス
		iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_)) => {
			Some(Message::PanelSystem(PanelSystemMessage::FocusPointer))
		}
		iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
			if status == iced::event::Status::Ignored =>
		{
			shortcut(&key, modifiers)
		}
		_ => None,
	}
}

/// After Effects / Final Cut 風の単キーショートカット
fn shortcut(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
	use keyboard::key::Named;

	if modifiers.command() || modifiers.control() || modifiers.alt() {
		return None;
	}

	match key.as_ref() {
		keyboard::Key::Named(Named::Space) => Some(Message::Transport(TransportMessage::TogglePlay)),
		keyboard::Key::Named(Named::Home) => Some(Message::Transport(TransportMessage::GoToStart)),
		keyboard::Key::Named(Named::End) => Some(Message::Transport(TransportMessage::GoToEnd)),
		keyboard::Key::Named(Named::ArrowLeft) => Some(Message::Transport(TransportMessage::StepBackward)),
		keyboard::Key::Named(Named::ArrowRight) => Some(Message::Transport(TransportMessage::StepForward)),
		keyboard::Key::Named(Named::Delete | Named::Backspace) => Some(Message::DeleteSelection),
		keyboard::Key::Character(c) => Tool::from_key(c).map(|tool| Message::Shell(ShellMessage::SelectTool(tool))),
		_ => None,
	}
}

fn pill_animation(index: usize) -> Animation<f32> {
	Animation::new(index as f32)
		.duration(Duration::from_millis(constants::style::MOTION_STANDARD_MS))
		.easing(Easing::EaseOutCubic)
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
		model.add_track(TimelineTrack::new("Video 1"));

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

	/// アニメーションカーブエディター
	curve_editor: CurveEditorState,

	/// オブジェクトブラウザ
	project_pane: ProjectPaneState,

	/// アセットブラウザ
	asset_browser: AssetBrowserState,

	/// パネルシステム（レイアウト・リサイズ管理）
	panel_system: PanelSystem<PanelContent>,

	/// 常駐レンダリングワーカー
	render_worker: RenderWorker,
	/// レンダリング結果受信用チャンネル（Subscriptionで使用）
	render_rx: Arc<std::sync::Mutex<Receiver<FrameData>>>,
	/// レンダリングServiceのシャットダウン通知
	render_shutdown_tx: Option<Sender<()>>,
	/// レンダリングServiceのシャットダウン受信機（Serviceへ渡す）
	render_shutdown_rx: Arc<std::sync::Mutex<Receiver<()>>>,

	/// FPS計算用カウンタ
	frame_count: u32,
	/// FPS更新の基準時刻
	fps_update_time: Instant,
	/// 実測のレンダリングフレームレート
	render_fps: f32,

	/// ビューアの現在のツール
	tool: Tool,
	/// ツールパレットの選択ピル位置
	tool_pill: Animation<f32>,
	/// 現在のワークスペース
	workspace: Workspace,
	/// ループ再生
	loop_playback: bool,
	/// HUD 通知
	toast: Option<Toast>,
	/// 直近の update 時刻（アニメーション評価用）
	now: Instant,
	/// アプリ起動時刻（ステータスの呼吸アニメーション用）
	started: Instant,
	/// ウィンドウの内寸
	window_size: Size,
	/// ビューアの表示設定
	viewer: ViewerSettings,
	/// ビューア領域のサイズ（倍率表示用）
	viewer_size: Size,
	/// プロパティパネルの UI 状態
	inspector_ui: panels::inspector::InspectorUi,
	/// デバッグ用スクリーンショット（`NADE_SCREENSHOT`）
	#[cfg(debug_assertions)]
	capture: Option<crate::devtools::Capture>,
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

		let window_size = Size::new(constants::WINDOW_WIDTH, constants::WINDOW_HEIGHT);
		let panel_system = Self::create_panel_layout(Workspace::default(), window_size);
		let (render_tx, render_rx) = unbounded::<FrameData>();
		let (shutdown_tx, shutdown_rx) = bounded(1);
		let render_worker = RenderWorker::start(render_tx);

		let mut app = Self {
			project,
			current_model: Model::default(),
			timeline: TimelinePanelState::default(),
			curve_editor: CurveEditorState::default(),
			project_pane,
			asset_browser,
			panel_system,
			render_worker,
			render_rx: Arc::new(std::sync::Mutex::new(render_rx)),
			render_shutdown_tx: Some(shutdown_tx),
			render_shutdown_rx: Arc::new(std::sync::Mutex::new(shutdown_rx)),
			frame_count: 0,
			fps_update_time: Instant::now(),
			render_fps: 0.0,
			tool: Tool::default(),
			tool_pill: pill_animation(Tool::default().index()),
			workspace: Workspace::default(),
			loop_playback: true,
			toast: None,
			now: Instant::now(),
			started: Instant::now(),
			window_size,
			viewer: ViewerSettings::default(),
			viewer_size: Size::new(960.0, 540.0),
			inspector_ui: panels::inspector::InspectorUi::default(),
			#[cfg(debug_assertions)]
			capture: crate::devtools::Capture::from_env(),
		};

		app.rebuild_timeline_for_composition(main_comp);
		app.sync_preview_size_from_composition(main_comp);

		// 初期フレームを描画
		app.apply_core_msg(Msg::SetTime(0.0));

		// 実際の内寸でパネルレイアウトを合わせる（透明タイトルバー分を含む）
		let initial_size = iced::window::latest()
			.and_then(iced::window::size)
			.map(Message::WindowResized);

		#[cfg(debug_assertions)]
		let initial_size = initial_size.chain(app.dev_scene());

		(app, initial_size)
	}

	/// `NADE_SCREENSHOT_SCENE` で指定された UI 状態を再現する
	#[cfg(debug_assertions)]
	fn dev_scene(&self) -> Task<Message> {
		let project = &self.project;
		let scene = crate::devtools::scene_messages(|name| {
			project.compositions().into_iter().find_map(|composition| {
				let instance = project
					.composition(&composition)?
					.all_objects()
					.find(|instance| instance.name() == name)?
					.id();
				Some(AppPanelMessage::Project(ProjectPaneMessage::Select(
					ObjectId::Instance { composition, instance },
				)))
			})
		});
		Task::batch(scene.into_iter().map(Task::done))
	}

	fn create_sample_project() -> (Project, CompositionId, AssetId) {
		let mut project = Project::default();

		let assets = project.create_folder("Assets", None);
		project.create_asset_in(Some(assets), "Background.png", AssetType::Image);
		project.create_asset_in(Some(assets), "0001-0004.mov", AssetType::Video);
		project.create_asset_in(Some(assets), "BGM.mp3", AssetType::Audio);

		let main = project.create_composition("Main Composition", 1920, 1080, 60.0);
		let scene = project.create_composition("Scene 1", 1280, 720, 30.0);

		let node_rect = project.create_node();
		let node_title = project.create_node();
		let node_badge = project.create_node();
		if let Some(comp) = project.composition_mut(&main) {
			let rect = comp.add_instance_named(node_rect, "Rectangle");
			rect.transform.position = [-120.0, 40.0, 0.0];

			let title = comp.add_instance_named(node_title, "Title");
			title.transform.position = [0.0, -60.0, 0.0];
			title.transform.scale = [1.2, 1.2, 1.0];
			title.set_content(InstanceContent::text("Title"));

			let badge = comp.add_instance_named(node_badge, "Badge");
			badge.transform.position = [160.0, 80.0, 0.0];
			badge.transform.opacity = 0.85;
			badge.set_content(InstanceContent::text("Badge"));
		}
		Self::animate_sample(&mut project, main);

		let node_bg = project.create_node();
		if let Some(comp) = project.composition_mut(&scene) {
			comp.add_instance_named(node_bg, "Background");
		}

		(project, main, assets)
	}

	/// サンプルにモーショングラフィックスらしい動きを付ける
	/// （タイトルのスライドイン + フェード、バッジのポップ）
	fn animate_sample(project: &mut Project, composition: CompositionId) {
		let Some(comp) = project.composition_mut(&composition) else {
			return;
		};
		/// (レイヤー名, チャンネル, [(時刻, 値)])
		type SampleKeys = (&'static str, TransformProperty, &'static [(f32, f32)]);
		let keys: [SampleKeys; 6] = [
			("Title", TransformProperty::PositionY, &[(0.0, 40.0), (1.0, -60.0)]),
			("Title", TransformProperty::Opacity, &[(0.0, 0.0), (0.6, 1.0)]),
			("Badge", TransformProperty::ScaleX, &[(0.8, 0.4), (1.4, 1.0)]),
			("Badge", TransformProperty::ScaleY, &[(0.8, 0.4), (1.4, 1.0)]),
			("Badge", TransformProperty::Opacity, &[(0.8, 0.0), (1.1, 0.85)]),
			("Badge", TransformProperty::RotationZ, &[(0.8, -12.0), (1.6, 0.0)]),
		];
		let ids: Vec<(String, InstanceId)> = comp
			.all_objects()
			.map(|instance| (instance.name().to_string(), instance.id()))
			.collect();
		for (name, property, points) in keys {
			let Some(id) = ids.iter().find(|(candidate, _)| candidate == name).map(|(_, id)| *id) else {
				continue;
			};
			if let Some(instance) = comp.get_mut(id) {
				let curve = instance.animation.curve_mut(property);
				for &(time, value) in points {
					curve.add_key(time, value);
				}
			}
		}
	}

	/// ワークスペースのパネルレイアウトを構築
	fn create_panel_layout(workspace: Workspace, window_size: Size) -> PanelSystem<PanelContent> {
		let (layout, next_area_id) = workspace.layout();
		let mut system = PanelSystem::new().with_layout(layout).with_ids(next_area_id);
		system.update(PanelSystemMessage::<PanelContent, AppPanelMessage>::WindowResized(
			panel_area(window_size),
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
					if !self.spawn_render(time, width, height) {
						log::trace!("Skipping frame render - render worker busy");
					}
				}
			}
		}
	}

	/// レンダリング要求をワーカーへ送る。キューが満杯なら `false`。
	fn spawn_render(&self, time: f32, width: u32, height: u32) -> bool {
		let track_hidden: Vec<bool> = (0..self.timeline.model.tracks.len())
			.map(|index| !self.timeline.model.is_track_rendered(index))
			.collect();

		let objects: Vec<TextObject> = self
			.target_composition()
			.and_then(|id| self.project.composition(&id))
			.map(|comp| {
				comp.objects_in_draw_order()
					.filter(|instance| {
						track_hidden
							.get(instance.track_index())
							.map(|hidden| !*hidden)
							.unwrap_or(true)
					})
					.filter_map(|instance| instance_to_text_object(instance, time))
					.collect()
			})
			.unwrap_or_default();

		// 解像度を落とすときはレイヤー座標とスケールも同じ倍率で縮める
		let factor = self.viewer.resolution.factor();
		let (width, height) = self.viewer.resolution.scale_size(width, height);
		let objects = if (factor - 1.0).abs() > f32::EPSILON {
			objects
				.into_iter()
				.map(|mut object| {
					for axis in 0..2 {
						object.transform.position[axis] *= factor;
						object.transform.scale[axis] *= factor;
					}
					object
				})
				.collect()
		} else {
			objects
		};

		self.render_worker.try_submit(RenderJob {
			objects,
			time,
			width,
			height,
		})
	}

	/// バックグラウンドレンダリング完了を処理する
	fn handle_render_completed(&mut self, frame: FrameData) {
		self.apply_core_msg(Msg::FrameRendered(frame));

		self.frame_count += 1;
		let now = Instant::now();
		let elapsed = now.duration_since(self.fps_update_time).as_secs_f32();
		if elapsed >= 0.5 {
			self.render_fps = self.frame_count as f32 / elapsed;
			self.frame_count = 0;
			self.fps_update_time = now;
		}
	}

	pub(super) fn update(&mut self, message: Message) -> Task<Message> {
		crate::platform::sync_scroll_phase(&mut self.timeline.state);
		let task = self.update_inner(message);
		crate::platform::sync_scroll_phase(&mut self.timeline.state);
		task
	}

	fn update_inner(&mut self, message: Message) -> Task<Message> {
		self.now = Instant::now();

		match message {
			Message::RenderCompleted(frame) => {
				self.handle_render_completed(frame);
				Task::none()
			}

			Message::Transport(transport) => {
				self.apply_transport(transport);
				Task::none()
			}

			Message::Shell(shell_message) => self.apply_shell_message(shell_message),

			Message::DeleteSelection => {
				self.delete_selected_layer();
				Task::none()
			}

			Message::WindowResized(size) => {
				self.window_size = size;
				self.panel_system
					.update(PanelSystemMessage::<PanelContent, AppPanelMessage>::WindowResized(
						panel_area(size),
					));
				Task::none()
			}

			Message::Tick => {
				self.timeline.state.tick_scroll_spring(self.now);
				if self.current_model.preview.is_playing {
					self.apply_core_msg(Msg::Tick);
					self.wrap_playback();
				}
				if self.toast.as_ref().is_some_and(|toast| !toast.is_active(self.now)) {
					self.toast = None;
				}
				#[cfg(debug_assertions)]
				if let Some(task) = self.capture.as_mut().and_then(|capture| capture.poll(self.now)) {
					return task;
				}
				if self.panel_system.is_animating() {
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
					PanelSystemMessage::AppMessage(AppPanelMessage::Timeline(message)) => {
						self.apply_timeline_message(message.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::CurveEditor(message)) => {
						self.apply_curve_editor_message(message.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::Project(project_msg)) => {
						self.apply_project_message(project_msg.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::Assets(assets_msg)) => {
						task = self.apply_assets_message(assets_msg.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(inspector_msg)) => {
						self.apply_inspector_message(inspector_msg.clone());
					}
					PanelSystemMessage::AppMessage(AppPanelMessage::Preview(preview_msg)) => {
						self.apply_preview_message(preview_msg.clone());
					}
					_ => {}
				}
				self.panel_system.update(msg);
				task
			}

			#[cfg(debug_assertions)]
			Message::DevScreenshot(screenshot) => {
				if let Some(capture) = &self.capture {
					capture.save(&screenshot);
				}
				self.update(Message::CloseRequested(iced::window::Id::unique()))
			}

			Message::MediaImported(path) => {
				if let Some(path) = path {
					self.import_media_path(path);
				}
				Task::none()
			}

			Message::CloseRequested(_id) => {
				if let Some(tx) = self.render_shutdown_tx.take()
					&& let Err(e) = tx.send(())
				{
					log::error!("App: Failed to send render shutdown: {:?}", e);
				}
				iced::exit()
			}
		}
	}

	// -------------------------------------------------------------------------
	// シェル・トランスポート
	// -------------------------------------------------------------------------

	fn apply_shell_message(&mut self, message: ShellMessage) -> Task<Message> {
		match message {
			ShellMessage::SelectTool(tool) => {
				self.tool = tool;
				self.tool_pill.go_mut(tool.index() as f32, self.now);
				Task::none()
			}
			ShellMessage::SelectWorkspace(workspace) => {
				if workspace != self.workspace {
					self.workspace = workspace;
					self.panel_system = Self::create_panel_layout(workspace, self.window_size);
				}
				Task::none()
			}
			ShellMessage::NewComposition => {
				self.create_composition();
				if let Some(name) = self.target_composition_name() {
					self.show_toast(Icon::Comp, format!("Created “{name}”"));
				}
				Task::none()
			}
			ShellMessage::NewTextLayer => {
				self.add_object_under_selection();
				Task::none()
			}
			ShellMessage::ImportMedia => Self::pick_media_file(),
			ShellMessage::DragWindow => iced::window::latest().and_then(iced::window::drag),
			ShellMessage::ToggleMaximize => iced::window::latest().and_then(iced::window::toggle_maximize),
		}
	}

	fn apply_transport(&mut self, transport: TransportMessage) {
		let fps = self.composition_fps();
		let duration = self.composition_duration();
		let time = self.current_model.preview.time;
		let last_frame = core::timecode::frame_at(duration, fps).saturating_sub(1);

		match transport {
			TransportMessage::TogglePlay => {
				// 終端で再生したら先頭から
				let at_end = core::timecode::frame_at(time, fps) >= last_frame;
				if !self.current_model.preview.is_playing && at_end {
					self.apply_core_msg(Msg::SetTime(0.0));
				}
				self.apply_core_msg(Msg::TogglePlay);
				log::debug!("Toggling play/pause: {}", self.current_model.preview.is_playing);
			}
			TransportMessage::GoToStart => self.apply_core_msg(Msg::SetTime(0.0)),
			TransportMessage::GoToEnd => {
				self.apply_core_msg(Msg::SetTime(core::timecode::time_of_frame(last_frame, fps)));
			}
			TransportMessage::StepBackward | TransportMessage::StepForward => {
				let frame = core::timecode::frame_at(time, fps);
				let target = if matches!(transport, TransportMessage::StepForward) {
					(frame + 1).min(last_frame)
				} else {
					frame.saturating_sub(1)
				};
				self.apply_core_msg(Msg::SetTime(core::timecode::time_of_frame(target, fps)));
			}
			TransportMessage::ToggleLoop => {
				self.loop_playback = !self.loop_playback;
				let label = if self.loop_playback {
					"Loop playback on"
				} else {
					"Loop playback off"
				};
				self.show_toast(Icon::Loop, label);
			}
		}
	}

	/// 再生がコンポジションの終端を越えたらループ or 停止
	fn wrap_playback(&mut self) {
		let duration = self.composition_duration();
		let time = self.current_model.preview.time;
		if time < duration {
			return;
		}
		if self.loop_playback {
			self.apply_core_msg(Msg::SetTime(time % duration.max(f32::EPSILON)));
		} else {
			self.apply_core_msg(Msg::TogglePlay);
			let fps = self.composition_fps();
			let last_frame = core::timecode::frame_at(duration, fps).saturating_sub(1);
			self.apply_core_msg(Msg::SetTime(core::timecode::time_of_frame(last_frame, fps)));
		}
	}

	fn composition_fps(&self) -> f32 {
		self.target_composition()
			.and_then(|id| self.project.composition(&id))
			.map_or(30.0, |comp| comp.fps())
	}

	fn composition_duration(&self) -> f32 {
		self.target_composition()
			.and_then(|id| self.project.composition(&id))
			.map_or(10.0, |comp| comp.duration())
			.max(0.1)
	}

	fn target_composition_name(&self) -> Option<String> {
		self.target_composition()
			.and_then(|id| self.project.composition(&id))
			.map(|comp| comp.name().to_string())
	}

	fn show_toast(&mut self, icon: Icon, text: impl Into<String>) {
		self.toast = Some(Toast::new(icon, text));
	}

	/// UI アニメーション（パネル・ピル・トースト）が進行中か
	fn is_ui_animating(&self) -> bool {
		self.timeline.state.is_scroll_animating()
			|| self.panel_system.is_animating()
			|| self.inspector_ui.is_animating(self.now)
			|| self.tool_pill.is_animating(self.now)
			|| self.toast.as_ref().is_some_and(|toast| toast.is_active(self.now))
	}

	/// 選択中のレイヤーを削除（カーブエディタにフォーカスがある時はキー削除を優先）
	fn delete_selected_layer(&mut self) {
		if matches!(self.panel_system.focused_content(), Some(PanelContent::CurveEditor)) {
			return;
		}
		let Some(ObjectId::Instance { composition, instance }) = self.project_pane.selected_id else {
			return;
		};
		let Some(comp) = self.project.composition_mut(&composition) else {
			return;
		};
		let name = comp.get(instance).map(|obj| obj.name().to_string()).unwrap_or_default();
		comp.remove(instance);

		self.rebuild_timeline_for_composition(composition);
		self.project_pane.selected_id = Some(ObjectId::Composition(composition));
		self.request_preview_refresh();
		self.show_toast(Icon::Trash, format!("Deleted “{name}”"));
	}

	fn apply_timeline_message(&mut self, msg: TimelineMessage) {
		let previous_selection = self.timeline.state.primary_selection();
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
			self.sync_instance_tracks_from_timeline();
			self.request_preview_refresh();
		}

		if timeline_update.clip_modified {
			self.sync_instances_from_timeline();
			self.request_preview_refresh();
		}

		if timeline_update.visibility_changed {
			self.request_preview_refresh();
		}

		if self.timeline.state.primary_selection() != previous_selection {
			self.sync_object_selection_from_timeline();
		}
	}

	fn apply_project_message(&mut self, msg: ProjectPaneMessage) {
		match msg {
			ProjectPaneMessage::ToggleExpand(id) => {
				self.toggle_expand(id);
			}
			ProjectPaneMessage::Select(id) => {
				self.select_object(id);
			}
			ProjectPaneMessage::ClearSelection => {
				self.project_pane.selected_id = None;
				self.timeline.state.clear_selection();
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
		self.select_object(id);
		if let ObjectId::Composition(composition_id) = id {
			self.project_pane.expanded_ids.insert(composition_id);
		}
	}

	fn select_object(&mut self, id: ObjectId) {
		self.project_pane.selected_id.replace(id);
		match id {
			ObjectId::Composition(composition_id) => {
				self.rebuild_timeline_for_composition(composition_id);
				self.sync_preview_size_from_composition(composition_id);
				self.request_preview_refresh();
			}
			ObjectId::Instance { composition, instance } => {
				if !self.timeline_has_instance(composition, instance) {
					self.rebuild_timeline_for_composition(composition);
				}
				self.sync_preview_size_from_composition(composition);
				self.sync_timeline_selection_from_object(id);
				self.focus_animated_channel(composition, instance);
			}
		}
	}

	/// カーブエディタの表示チャンネルが空なら、アニメーションのある最初のチャンネルへ切り替える
	fn focus_animated_channel(&mut self, composition: CompositionId, instance: InstanceId) {
		let Some(obj) = self
			.project
			.composition(&composition)
			.and_then(|comp| comp.get(instance))
		else {
			return;
		};
		if obj.is_animated(self.curve_editor.selected_property) {
			return;
		}
		if let Some(property) = TransformProperty::ALL
			.into_iter()
			.find(|property| obj.is_animated(*property))
		{
			self.curve_editor.selected_property = property;
			self.curve_editor.selected_key = None;
		}
	}

	fn create_composition(&mut self) {
		let name = unique_composition_name(&self.project);
		let id = self.project.create_composition(name, 1920, 1080, 60.0);
		self.project_pane.expanded_ids.insert(id);
		self.select_object(ObjectId::Composition(id));
	}

	fn add_object_under_selection(&mut self) {
		self.add_text_layer_at(None);
	}

	/// テキストレイヤーを追加（位置指定はコンポ中心基準）
	fn add_text_layer_at(&mut self, position: Option<[f32; 2]>) {
		let Some(composition_id) = self.target_composition() else {
			log::debug!("Add object ignored: no composition selected");
			return;
		};

		let name = unique_object_name(&self.project, composition_id);
		let node_id = self.project.create_node();
		let (instance_id, start_time, duration, track_index) = {
			let Some(comp) = self.project.composition_mut(&composition_id) else {
				return;
			};
			let instance = comp.add_instance_named(node_id, name.clone());
			instance.set_content(InstanceContent::text_default());
			if let Some([x, y]) = position {
				instance.transform_mut().position = [x, y, 0.0];
			}
			(
				instance.id(),
				instance.start_time(),
				instance.duration(),
				instance.track_index(),
			)
		};

		self.add_instance_clip(composition_id, instance_id, &name, start_time, duration, track_index);
		self.project_pane.expanded_ids.insert(composition_id);
		self.select_object(ObjectId::Instance {
			composition: composition_id,
			instance: instance_id,
		});
		self.request_preview_refresh();
		self.show_toast(Icon::LayerText, format!("Added text layer “{name}”"));
	}

	/// プレビュー解像度をコンポジションに合わせる（低解像度アップスケールの格子を防ぐ）
	fn sync_preview_size_from_composition(&mut self, composition_id: CompositionId) {
		let Some(comp) = self.project.composition(&composition_id) else {
			return;
		};
		self.current_model.preview.width = comp.width();
		self.current_model.preview.height = comp.height();
		self.current_model.preview.fps = comp.fps();
	}

	fn rebuild_timeline_for_composition(&mut self, composition_id: CompositionId) {
		self.timeline.state.clear_selection();

		let clips: Vec<_> = self
			.project
			.composition(&composition_id)
			.map(|comp| {
				comp.all_objects()
					.map(|obj| {
						(
							obj.id(),
							obj.name().to_string(),
							obj.start_time(),
							obj.duration(),
							obj.track_index(),
						)
					})
					.collect()
			})
			.unwrap_or_default();

		let track_count = clips
			.iter()
			.map(|(_, _, _, _, track_index)| *track_index + 1)
			.max()
			.unwrap_or(1)
			.max(1);

		self.timeline.model.tracks.clear();
		self.timeline.model.ensure_track_count(track_count);

		for (instance_id, name, start_time, duration, track_index) in clips {
			self.add_instance_clip(composition_id, instance_id, &name, start_time, duration, track_index);
		}
	}

	fn add_instance_clip(
		&mut self,
		composition_id: CompositionId,
		instance_id: InstanceId,
		name: &str,
		start_time: f32,
		duration: f32,
		track_index: usize,
	) {
		self.timeline.model.ensure_track_count(track_index + 1);

		let clip_id = self.timeline.model.alloc_clip_id();
		if let Some(track) = self.timeline.model.tracks.get_mut(track_index) {
			track.add_clip(TimelineClip::from_instance(
				clip_id,
				name,
				start_time,
				duration,
				composition_id,
				instance_id,
			));
		}
	}

	fn sync_instances_from_timeline(&mut self) {
		for (track_index, track) in self.timeline.model.tracks.iter().enumerate() {
			for clip in &track.clips {
				let (Some(composition_id), Some(instance_id)) = (clip.composition_id, clip.instance_id) else {
					continue;
				};
				let Some(instance) = self
					.project
					.composition_mut(&composition_id)
					.and_then(|comp| comp.get_mut(instance_id))
				else {
					continue;
				};
				instance.set_start_time(clip.start_time);
				instance.set_duration(clip.duration);
				instance.set_track_index(track_index);
			}
		}
	}

	fn sync_instance_tracks_from_timeline(&mut self) {
		for (track_index, track) in self.timeline.model.tracks.iter().enumerate() {
			for clip in &track.clips {
				let (Some(composition_id), Some(instance_id)) = (clip.composition_id, clip.instance_id) else {
					continue;
				};
				if let Some(instance) = self
					.project
					.composition_mut(&composition_id)
					.and_then(|comp| comp.get_mut(instance_id))
				{
					instance.set_track_index(track_index);
				}
			}
		}
	}

	fn sync_object_selection_from_timeline(&mut self) {
		let Some((track_index, clip_id)) = self.timeline.state.primary_selection() else {
			if matches!(self.project_pane.selected_id, Some(ObjectId::Instance { .. })) {
				self.project_pane.selected_id = None;
			}
			return;
		};

		let Some(clip) = self
			.timeline
			.model
			.tracks
			.get(track_index)
			.and_then(|track| track.clips.iter().find(|clip| clip.id == clip_id))
		else {
			return;
		};

		let (Some(composition), Some(instance)) = (clip.composition_id, clip.instance_id) else {
			return;
		};

		self.project_pane.expanded_ids.insert(composition);
		self.project_pane.selected_id = Some(ObjectId::Instance { composition, instance });
	}

	fn sync_timeline_selection_from_object(&mut self, id: ObjectId) {
		let ObjectId::Instance { composition, instance } = id else {
			self.timeline.state.clear_selection();
			return;
		};

		for (track_index, track) in self.timeline.model.tracks.iter().enumerate() {
			if let Some(clip) = track
				.clips
				.iter()
				.find(|clip| clip.composition_id == Some(composition) && clip.instance_id == Some(instance))
			{
				self.timeline.state.select_clip(track_index, clip.id);
				return;
			}
		}

		self.timeline.state.clear_selection();
	}

	fn timeline_has_instance(&self, composition: CompositionId, instance: InstanceId) -> bool {
		self.timeline.model.tracks.iter().any(|track| {
			track
				.clips
				.iter()
				.any(|clip| clip.composition_id == Some(composition) && clip.instance_id == Some(instance))
		})
	}

	fn target_composition(&self) -> Option<CompositionId> {
		match self.project_pane.selected_id? {
			ObjectId::Composition(id) => Some(id),
			ObjectId::Instance { composition, .. } => Some(composition),
		}
	}

	fn selected_transform_animation(&self) -> Option<&TransformAnimation> {
		let ObjectId::Instance { composition, instance } = self.project_pane.selected_id? else {
			return None;
		};
		self.project
			.composition(&composition)?
			.get(instance)
			.map(|obj| &obj.animation)
	}

	fn request_preview_refresh(&mut self) {
		let time = self.current_model.preview.time;
		self.apply_core_msg(Msg::SetTime(time));
	}

	fn apply_curve_editor_message(&mut self, msg: CurveEditorMessage) {
		// キャンバスはフォーカスに関係なくキー入力を受け取るため、Delete は
		// カーブエディタにフォーカスがある時だけキー削除として扱う
		if matches!(
			msg,
			CurveEditorMessage::CanvasEvent(curve_editor_panel::CurveCanvasEvent::DeletePressed)
		) && !matches!(self.panel_system.focused_content(), Some(PanelContent::CurveEditor))
		{
			return;
		}
		let Some(ObjectId::Instance { composition, instance }) = self.project_pane.selected_id else {
			return;
		};
		let current_time = self.current_model.preview.time;
		let Some(obj) = self
			.project
			.composition_mut(&composition)
			.and_then(|composition| composition.get_mut(instance))
		else {
			return;
		};
		let base = *obj.transform();
		let update = self
			.curve_editor
			.apply_message(&mut obj.animation, &base, msg, current_time);

		if let Some(time) = update.playhead_time {
			self.apply_core_msg(Msg::SetTime(time));
		} else if update.curve_changed {
			self.request_preview_refresh();
		}
	}

	fn apply_preview_message(&mut self, msg: PreviewMessage) {
		match msg {
			PreviewMessage::ViewChanged { zoom, offset } => {
				self.current_model.preview.view_zoom = zoom;
				self.current_model.preview.view_offset = offset;
			}
			PreviewMessage::ViewportSized(size) => self.viewer_size = size,
			PreviewMessage::FitView => {
				self.current_model.preview.view_zoom = 1.0;
				self.current_model.preview.view_offset = [0.0, 0.0];
			}
			PreviewMessage::ZoomBy(factor) => {
				let preview = &mut self.current_model.preview;
				let zoom = (preview.view_zoom * factor)
					.clamp(crate::widgets::viewport::MIN_ZOOM, crate::widgets::viewport::MAX_ZOOM);
				let applied = zoom / preview.view_zoom;
				preview.view_zoom = zoom;
				preview.view_offset = [preview.view_offset[0] * applied, preview.view_offset[1] * applied];
			}
			PreviewMessage::ActualPixels => {
				let (width, height) = (self.current_model.preview.width, self.current_model.preview.height);
				let preview = &mut self.current_model.preview;
				let viewport = Viewport::new(
					self.viewer_size,
					Size::new(width as f32, height as f32),
					preview.view_zoom,
					iced::Vector::new(preview.view_offset[0], preview.view_offset[1]),
				);
				let (zoom, offset) = viewport.with_magnification(1.0);
				preview.view_zoom = zoom;
				preview.view_offset = [offset.x, offset.y];
			}
			PreviewMessage::ToggleGrid => self.viewer.show_grid = !self.viewer.show_grid,
			PreviewMessage::ToggleSafeAreas => self.viewer.show_safe_areas = !self.viewer.show_safe_areas,
			PreviewMessage::SetResolution(resolution) => {
				self.viewer.resolution = resolution;
				self.request_preview_refresh();
			}
			PreviewMessage::Transport(transport) => self.apply_transport(transport),
			PreviewMessage::SelectLayer(Some(instance)) => {
				if let Some(composition) = self.target_composition() {
					self.select_object(ObjectId::Instance { composition, instance });
				}
			}
			PreviewMessage::SelectLayer(None) => {
				if let Some(composition) = self.target_composition() {
					self.project_pane.selected_id = Some(ObjectId::Composition(composition));
					self.timeline.state.clear_selection();
				}
			}
			PreviewMessage::MoveLayer { delta } => {
				self.edit_selected_property(TransformProperty::PositionX, |value| value + delta[0]);
				self.edit_selected_property(TransformProperty::PositionY, |value| value + delta[1]);
				self.request_preview_refresh();
			}
			PreviewMessage::RotateLayer { degrees } => {
				self.edit_selected_property(TransformProperty::RotationZ, |value| value + degrees);
				self.request_preview_refresh();
			}
			PreviewMessage::CreateText { position } => {
				self.add_text_layer_at(Some(position));
				self.tool = Tool::Select;
				self.tool_pill.go_mut(Tool::Select.index() as f32, self.now);
			}
		}
	}

	/// 選択インスタンスのチャンネルを現在時刻で編集（アニメーション中ならキーを打つ）
	fn edit_selected_property(&mut self, property: TransformProperty, edit: impl Fn(f32) -> f32) {
		let Some(ObjectId::Instance { composition, instance }) = self.project_pane.selected_id else {
			return;
		};
		let time = self.current_model.preview.time;
		if let Some(obj) = self
			.project
			.composition_mut(&composition)
			.and_then(|comp| comp.get_mut(instance))
		{
			let value = edit(obj.property_at(property, time));
			obj.set_property_at(property, time, value);
		}
	}

	/// タイムラインのクリップごとのキーフレーム時刻（全チャンネルを統合）
	fn clip_keyframes(&self) -> std::collections::HashMap<usize, Vec<f32>> {
		self.timeline
			.model
			.tracks
			.iter()
			.flat_map(|track| track.clips.iter())
			.filter_map(|clip| {
				let instance = self
					.project
					.composition(&clip.composition_id?)?
					.get(clip.instance_id?)?;
				let mut times: Vec<f32> = TransformProperty::ALL
					.into_iter()
					.flat_map(|property| instance.animation.curve(property).keys().iter().map(|key| key.time))
					.collect();
				if times.is_empty() {
					return None;
				}
				times.sort_by(f32::total_cmp);
				times.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
				Some((clip.id, times))
			})
			.collect()
	}

	/// 現在時刻で表示中のレイヤーのビューア上の境界（背面 → 前面）
	fn viewer_layers(&self) -> Vec<LayerBox> {
		let time = self.current_model.preview.time;
		let Some(comp) = self.target_composition().and_then(|id| self.project.composition(&id)) else {
			return Vec::new();
		};

		comp.objects_in_draw_order()
			.filter(|instance| time >= instance.start_time() && time < instance.end_time())
			.filter(|instance| self.timeline.model.is_track_rendered(instance.track_index()))
			.map(|instance| {
				let transform = instance.transform_at(time);
				let size = instance
					.content()
					.as_text()
					.and_then(|(text, font_path, font_size, spacing, _)| {
						renderer::measure_text(font_path, text, font_size, spacing)
					})
					.unwrap_or([0.0, 0.0]);
				LayerBox {
					id: instance.id(),
					center: [transform.position[0], transform.position[1]],
					half_size: [
						size[0] * transform.scale[0].abs() * 0.5,
						size[1] * transform.scale[1].abs() * 0.5,
					],
					rotation: transform.rotation[2],
				}
			})
			.collect()
	}

	fn viewer_props(&self) -> panels::preview::ViewerProps<'_> {
		let composition = self.target_composition().and_then(|id| self.project.composition(&id));
		let selected = match self.project_pane.selected_id {
			Some(ObjectId::Instance { instance, .. }) => Some(instance),
			_ => None,
		};
		panels::preview::ViewerProps {
			preview: &self.current_model.preview,
			composition: composition.map(|comp| crate::viewer::CompositionSummary {
				name: comp.name(),
				width: comp.width(),
				height: comp.height(),
				fps: comp.fps(),
				duration: comp.duration(),
			}),
			tool: self.tool,
			layers: self.viewer_layers(),
			selected,
			settings: self.viewer,
			viewport_size: self.viewer_size,
			loop_playback: self.loop_playback,
		}
	}

	fn apply_inspector_message(&mut self, msg: InspectorMessage) {
		let now = self.now;
		match msg {
			InspectorMessage::ToggleSection(section) => {
				self.inspector_ui.toggle(section, now);
				return;
			}
			InspectorMessage::ToggleScaleLink => {
				self.inspector_ui.scale_linked = !self.inspector_ui.scale_linked;
				return;
			}
			InspectorMessage::HexEdited(draft) => {
				self.inspector_ui.hex_draft = Some(draft);
				return;
			}
			_ => {}
		}

		let Some(ObjectId::Instance { composition, instance }) = self.project_pane.selected_id else {
			return;
		};
		let time = self.current_model.preview.time;
		let fps = self.composition_fps();
		let scale_linked = self.inspector_ui.scale_linked;
		let hex_draft = self.inspector_ui.hex_draft.take();

		let Some(obj) = self
			.project
			.composition_mut(&composition)
			.and_then(|comp| comp.get_mut(instance))
		else {
			return;
		};

		let mut toast = None;
		let mut jump = None;
		let mut renamed = None;

		match msg {
			InspectorMessage::SetChannel { property, value } => {
				let partner = match property {
					TransformProperty::ScaleX => Some(TransformProperty::ScaleY),
					TransformProperty::ScaleY => Some(TransformProperty::ScaleX),
					_ => None,
				};
				// 縦横比を固定: 変化率をもう一方の軸にも掛ける
				if let (true, Some(partner)) = (scale_linked, partner) {
					let previous = obj.property_at(property, time);
					let other = obj.property_at(partner, time);
					let other_next = if previous.abs() > f32::EPSILON {
						other * value / previous
					} else {
						value
					};
					obj.set_property_at(partner, time, other_next);
				}
				obj.set_property_at(property, time, value);
			}
			InspectorMessage::ToggleKeyframe(group) => {
				let channels = group.channels();
				let on_key = channels
					.iter()
					.any(|property| obj.animation.curve(*property).key_at(time).is_some());
				for property in channels {
					let has_key = obj.animation.curve(*property).key_at(time).is_some();
					if has_key == on_key {
						obj.toggle_key_at(*property, time);
					}
				}
				let timecode = core::timecode::format(time, fps);
				toast = Some(if on_key {
					(Icon::Keyframe, format!("Removed {} key at {timecode}", group.label()))
				} else {
					(Icon::KeyframeFilled, format!("Keyed {} at {timecode}", group.label()))
				});
			}
			InspectorMessage::JumpToKey { group, forward } => {
				let times = group.channels().iter().filter_map(|property| {
					let curve = obj.animation.curve(*property);
					if forward {
						curve.next_key_time(time)
					} else {
						curve.previous_key_time(time)
					}
				});
				jump = if forward {
					times.reduce(f32::min)
				} else {
					times.reduce(f32::max)
				};
			}
			InspectorMessage::ResetTransform => {
				for group in PropertyGroup::ALL {
					for property in group.channels() {
						obj.set_property_at(*property, time, PropertyGroup::default_value(*property));
					}
				}
				toast = Some((Icon::Reset, "Transform reset".to_string()));
			}
			InspectorMessage::Rename(name) => {
				obj.name = name.clone();
				renamed = Some(name);
			}
			InspectorMessage::SetText(text) => {
				if let Some(slot) = obj.content_mut().text_mut() {
					*slot = text;
				}
			}
			InspectorMessage::SetFontSize(size) => {
				if let Some(slot) = obj.content_mut().font_size_mut() {
					*slot = size.max(1.0);
				}
			}
			InspectorMessage::SetTracking(spacing) => {
				if let Some(slot) = obj.content_mut().spacing_mut() {
					*slot = spacing;
				}
			}
			InspectorMessage::SetFillChannel { index, value } => {
				if let Some(color) = obj.content_mut().fill_color_mut()
					&& let Some(slot) = color.get_mut(index)
				{
					*slot = value.clamp(0.0, 1.0);
				}
			}
			InspectorMessage::SetFillRgb(rgb) => {
				if let Some(color) = obj.content_mut().fill_color_mut() {
					color[..3].copy_from_slice(&rgb);
				}
			}
			InspectorMessage::HexSubmitted => {
				if let Some(rgb) = hex_draft.as_deref().and_then(parse_hex_color)
					&& let Some(color) = obj.content_mut().fill_color_mut()
				{
					color[..3].copy_from_slice(&rgb);
				}
			}
			InspectorMessage::ToggleSection(_) | InspectorMessage::ToggleScaleLink | InspectorMessage::HexEdited(_) => {
			}
		}

		if let Some(name) = renamed {
			for clip in self
				.timeline
				.model
				.tracks
				.iter_mut()
				.flat_map(|track| track.clips.iter_mut())
			{
				if clip.instance_id == Some(instance) && clip.composition_id == Some(composition) {
					clip.name = name.clone();
				}
			}
		}
		if let Some((icon, text)) = toast {
			self.show_toast(icon, text);
		}
		match jump {
			Some(time) => self.apply_core_msg(Msg::SetTime(time)),
			None => self.request_preview_refresh(),
		}
	}

	fn inspector_target(&self) -> panels::inspector::InspectorTarget<'_> {
		use panels::inspector::{InspectorTarget, LayerTarget};

		match self.project_pane.selected_id {
			Some(ObjectId::Instance { composition, instance }) => {
				let Some(obj) = self
					.project
					.composition(&composition)
					.and_then(|comp| comp.get(instance))
				else {
					return InspectorTarget::Empty;
				};
				InspectorTarget::Layer(LayerTarget {
					instance: obj,
					time: self.current_model.preview.time,
					label: constants::style::label_color(obj.track_index()),
				})
			}
			Some(ObjectId::Composition(id)) => match self.project.composition(&id) {
				Some(comp) => InspectorTarget::Composition {
					summary: crate::viewer::CompositionSummary {
						name: comp.name(),
						width: comp.width(),
						height: comp.height(),
						fps: comp.fps(),
						duration: comp.duration(),
					},
					layers: comp.len(),
				},
				None => InspectorTarget::Empty,
			},
			None => InspectorTarget::Empty,
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
			AssetBrowserMessage::FilterChanged(filter) => {
				self.asset_browser.filter = filter;
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
				let name = path
					.file_name()
					.map(|name| name.to_string_lossy().into_owned())
					.unwrap_or_default();
				self.show_toast(Icon::Import, format!("Imported “{name}”"));
			}
			None => {
				log::warn!("Unsupported media type: {}", path.display());
				self.show_toast(Icon::Close, "Unsupported media type");
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
		let track_index = if asset.kind() == AssetType::Audio { 1 } else { 0 };
		self.timeline.model.ensure_track_count(track_index + 1);
		let clip_id = self.timeline.model.alloc_clip_id();

		if let Some(track) = self.timeline.model.tracks.get_mut(track_index) {
			track.add_clip(TimelineClip::new(clip_id, &name, start_time, duration));
			log::info!("Added `{name}` to timeline at {start_time:.2}s");
			self.show_toast(Icon::Insert, format!("Added “{name}” to the timeline"));
		}
	}

	pub(super) fn view(&self) -> Element<'_, Message> {
		let panel_view = self.panel_system.view(|_, content| self.view_panel_content(content));

		let composition = self.target_composition().and_then(|id| self.project.composition(&id));

		let mut chrome = column![];
		if TITLE_BAR_HEIGHT > 0.0 {
			chrome = chrome.push(shell::title_bar::view(
				self.project.name(),
				composition.map(|comp| comp.name()),
			));
		}

		let breathing = self.now.saturating_duration_since(self.started).as_secs_f32() * std::f32::consts::TAU * 0.6;
		let pulse = breathing.sin() * 0.5 + 0.5;

		let chrome = chrome
			.push(shell::toolbar::view(shell::toolbar::ToolbarState {
				tool: self.tool,
				tool_pill: self.tool_pill.interpolate_with(|value| value, self.now),
			}))
			.push(
				container(panel_view.map(Message::PanelSystem))
					.padding([0.0, SHELL_GUTTER])
					.width(Length::Fill)
					.height(Length::Fill),
			)
			.push(shell::status_bar::view(shell::status_bar::StatusInfo {
				playing: self.current_model.preview.is_playing,
				pulse,
				render_fps: self.render_fps,
				composition: composition.map(|comp| crate::viewer::CompositionSummary {
					name: comp.name(),
					width: comp.width(),
					height: comp.height(),
					fps: comp.fps(),
					duration: comp.duration(),
				}),
			}));

		let base = container(chrome)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(constants::widgets::window);

		match self.toast.as_ref().filter(|toast| toast.is_active(self.now)) {
			Some(toast) => stack![base, shell::toast::view(toast, self.now, STATUS_BAR_HEIGHT + 18.0)]
				.width(Length::Fill)
				.height(Length::Fill)
				.into(),
			None => base.into(),
		}
	}

	fn view_panel_content<'a>(
		&'a self,
		content: &PanelContent,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		match content {
			PanelContent::Timeline => panels::timeline::view(panels::timeline::TimelineProps {
				state: &self.timeline.state,
				model: &self.timeline.model,
				current_time: self.current_model.preview.time,
				fps: self.composition_fps(),
				duration: self.composition_duration(),
				keyframes: self.clip_keyframes(),
			}),
			PanelContent::CurveEditor => panels::curve_editor::view(
				&self.curve_editor,
				self.selected_transform_animation(),
				self.current_model.preview.time,
			),
			PanelContent::MainPreview => panels::preview::view(self.viewer_props()),
			PanelContent::Inspector => panels::inspector::view(self.inspector_target(), &self.inspector_ui, self.now),
			PanelContent::Project => panels::browser::view(&self.project, &self.project_pane),
			PanelContent::Assets => panels::assets::view(&self.project, &self.asset_browser),
		}
	}

	pub(super) fn subscription(&self) -> Subscription<Message> {
		let render_subscription = Subscription::run_with(
			RenderConnection(self.render_rx.clone(), self.render_shutdown_rx.clone()),
			build_render_stream,
		);

		let events = iced::event::listen_with(on_window_event);

		// Only subscribe while something needs time-based updates; idle has no timer.
		let needs_tick = self.current_model.preview.is_playing || self.is_ui_animating();
		#[cfg(debug_assertions)]
		let needs_tick = needs_tick || self.capture.as_ref().is_some_and(|capture| capture.is_pending());
		let tick = if needs_tick {
			iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
		} else {
			Subscription::none()
		};
		let close = iced::window::close_requests().map(Message::CloseRequested);

		Subscription::batch([render_subscription, events, tick, close])
	}
}

fn instance_to_text_object(instance: &Instance, time: f32) -> Option<TextObject> {
	let (text, font_path, font_size, spacing, fill_color) = instance.content().as_text()?;
	Some(TextObject {
		id: SceneObjectId(instance.id().value() as u64),
		name: instance.name().to_string(),
		transform: instance.transform_at(time),
		start_time: instance.start_time(),
		duration: instance.duration(),
		text: text.to_string(),
		font_path: font_path.to_string(),
		font_size,
		spacing,
		fill_color,
	})
}

/// `#RRGGBB` / `RRGGBB` / `#RGB` → linear 0..1 RGB.
fn parse_hex_color(input: &str) -> Option<[f32; 3]> {
	let hex = input.trim().trim_start_matches('#');
	let expanded: String = match hex.len() {
		3 => hex.chars().flat_map(|c| [c, c]).collect(),
		6 => hex.to_string(),
		_ => return None,
	};
	let channel = |index: usize| u8::from_str_radix(&expanded[index..index + 2], 16).ok();
	Some([
		channel(0)? as f32 / 255.0,
		channel(2)? as f32 / 255.0,
		channel(4)? as f32 / 255.0,
	])
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

	unique_numbered_name("Text", &existing)
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

#[cfg(test)]
mod tests {
	use super::parse_hex_color;

	#[test]
	fn parses_hex_colors() {
		assert_eq!(parse_hex_color("#FF0000"), Some([1.0, 0.0, 0.0]));
		assert_eq!(parse_hex_color("0f0"), Some([0.0, 1.0, 0.0]));
		assert_eq!(parse_hex_color("#12345"), None);
		assert_eq!(parse_hex_color("#GG0000"), None);
	}
}
