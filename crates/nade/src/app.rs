//! # Nade アプリケーション
//!
//! NadeのメインUIアプリケーション実装です。

use iced::keyboard;
use iced::time;
use iced::widget::container;
use iced::{Element, Length, Subscription, Task, Theme};
use panel_system::{DockNode, LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;

use timeline_panel::TimelineInteraction;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use inspector_panel::InspectorUiState;
use core::{
	CoreEffect, FrameData, Model, Msg, RectangleObject, TimelineClip, TimelineModel, TimelineTrack,
};
use domain::{AssetType, Composition, InstanceId, Project};

use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;
use crate::panels;
use crate::services::render_service::{RenderConnection, build_render_stream};
use project_panel::ProjectPaneState;

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

#[derive(Debug)]
struct TimelinePanelState {
	state: TimelineInteraction,
	model: TimelineModel,
	clip_bindings: HashMap<usize, InstanceId>,
}

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
	global_state: Model,
	/// バックグラウンドレンダリング中かどうか
	is_rendering: Arc<AtomicBool>,

	/// FPS計算用カウンタ
	frame_count: u32,

	/// FPS更新の基準時刻
	fps_update_time: Instant,

	/// タイムラインパネルごとのUI状態
	timeline_panels: HashMap<usize, TimelinePanelState>,

	/// プロジェクトUI状態
	project_ui: ProjectPaneState,

	project: Project,

	/// インスペクターUI状態
	inspector_ui: InspectorUiState,

	/// コンポジション（シーンオブジェクト管理）
	composition: Arc<std::sync::Mutex<Composition>>,
}

impl NadeApp {
	/// アプリケーションの初期化
	pub fn new() -> (Self, Task<Message>) {
		let panel_system = Self::create_panel_layout();
		let (render_tx, render_rx) = unbounded::<FrameData>();
		let (shutdown_tx, shutdown_rx) = bounded(1);
		let now = Instant::now();
		let composition = Arc::new(std::sync::Mutex::new(Self::create_sample_composition()));
		let timeline_panels = {
			let comp = composition
				.lock()
				.expect("composition lock should be available");
			Self::build_timeline_panel_instances(panel_system.root(), &comp)
		};

		let mut project = Project::default();
		project.add_asset("hogehoge image".into(), AssetType::Image);
		project.add_asset("0001-0004.mov".into(), AssetType::Video);
		project.add_asset("16mm Burn 6.mov".into(), AssetType::Video);

		let mut app = Self {
			panel_system,
			render_tx,
			render_rx: Arc::new(std::sync::Mutex::new(render_rx)),
			render_shutdown_tx: Some(shutdown_tx),
			render_shutdown_rx: Arc::new(std::sync::Mutex::new(shutdown_rx)),
			global_state: Model::default(),
			is_rendering: Arc::new(AtomicBool::new(false)),
			frame_count: 0,
			fps_update_time: now,
			timeline_panels,
			project,
			project_ui: ProjectPaneState::default(),
			inspector_ui: InspectorUiState::new(),
			composition,
		};

		// 初期フレームを描画するためのトリガー
		app.apply_core_msg(Msg::SetTime(0.0));

		(app, Task::none())
	}

	/// デフォルトのパネルレイアウトを作成
	fn create_panel_layout() -> PanelSystem<PanelContent> {
		let mut builder = LayoutBuilder::new();
		let layout = builder.panel("Inspector", PanelContent::Inspector);

		PanelSystem::new().with_layout(layout)
	}

	fn build_timeline_panel_instances(
		root: &DockNode<PanelContent>,
		composition: &Composition,
	) -> HashMap<usize, TimelinePanelState> {
		let mut timeline_ids = Vec::new();
		Self::collect_timeline_panel_ids(root, &mut timeline_ids);

		timeline_ids
			.into_iter()
			.map(|panel_id| (panel_id, Self::build_timeline_panel_state(composition)))
			.collect()
	}

	fn build_timeline_panel_state(composition: &Composition) -> TimelinePanelState {
		let (model, clip_bindings) = Self::build_timeline_model_from_composition(composition);
		TimelinePanelState {
			state: TimelineInteraction::new(),
			model,
			clip_bindings,
		}
	}

	fn build_timeline_model_from_composition(
		composition: &Composition,
	) -> (TimelineModel, HashMap<usize, InstanceId>) {
		let mut model = TimelineModel::new();
		let mut clip_bindings = HashMap::new();
		let mut next_clip_id = 0usize;

		for object in composition.all_objects() {
			let mut track = TimelineTrack::new(object.name());
			let clip_id = next_clip_id;
			next_clip_id += 1;
			let clip = TimelineClip::new(
				clip_id,
				object.name(),
				object.start_time(),
				object.duration(),
			);
			clip_bindings.insert(clip_id, object.id());
			track.add_clip(clip);
			model.add_track(track);
		}

		(model, clip_bindings)
	}

	fn apply_timeline_model_to_composition(
		model: &TimelineModel,
		clip_bindings: &HashMap<usize, InstanceId>,
		composition: &mut Composition,
	) {
		for track in &model.tracks {
			for clip in &track.clips {
				if let Some(instance_id) = clip_bindings.get(&clip.id).copied()
					&& let Some(object) = composition.get_mut(instance_id)
				{
					object.set_start_time(clip.start_time);
					object.set_duration(clip.duration);
				}
			}
		}
	}

	fn collect_timeline_panel_ids(node: &DockNode<PanelContent>, ids: &mut Vec<usize>) {
		match node {
			DockNode::Empty => {}
			DockNode::Leaf(container) => {
				for panel in &container.panels {
					if panel.content == PanelContent::Timeline {
						ids.push(panel.id);
					}
				}
			}
			DockNode::Split { first, second, .. } => {
				Self::collect_timeline_panel_ids(first, ids);
				Self::collect_timeline_panel_ids(second, ids);
			}
		}
	}

	fn ensure_timeline_panel_state(&mut self, panel_id: usize) -> &mut TimelinePanelState {
		if !self.timeline_panels.contains_key(&panel_id) {
			let comp = self
				.composition
				.lock()
				.expect("composition lock should be available");
			self.timeline_panels
				.insert(panel_id, Self::build_timeline_panel_state(&comp));
		}

		self.timeline_panels
			.get_mut(&panel_id)
			.expect("timeline panel should exist")
	}

	fn reconcile_timeline_panels(&mut self) {
		let mut timeline_ids = Vec::new();
		Self::collect_timeline_panel_ids(self.panel_system.root(), &mut timeline_ids);
		let timeline_set: HashSet<usize> = timeline_ids.iter().copied().collect();

		self.timeline_panels
			.retain(|panel_id, _| timeline_set.contains(panel_id));

		if timeline_ids
			.iter()
			.any(|panel_id| !self.timeline_panels.contains_key(panel_id))
		{
			let comp = self
				.composition
				.lock()
				.expect("composition lock should be available");
			for panel_id in timeline_ids {
				if self.timeline_panels.contains_key(&panel_id) {
					continue;
				}
				self.timeline_panels
					.insert(panel_id, Self::build_timeline_panel_state(&comp));
			}
		}
	}

	/// サンプルのコンポジションを作成
	fn create_sample_composition() -> Composition {
		let mut composition = Composition::new();

		let rect = RectangleObject::new("Blue Rectangle")
			.with_start_time(0.0)
			.with_duration(5.0)
			.with_size(200.0, 150.0)
			.with_fill_color(0.2, 0.6, 1.0, 1.0);
		let instance = composition.add_instance(core::NodeId::new(composition.len()));
		instance.name = rect.name;
		instance.start_time = rect.start_time;
		instance.duration = rect.duration;

		let rect = RectangleObject::new("Red Rectangle")
			.with_start_time(3.0)
			.with_duration(5.0)
			.with_size(100.0, 100.0)
			.with_fill_color(1.0, 0.3, 0.3, 0.8)
			.with_transform(core::Transform {
				position: [100.0, 50.0, 0.0],
				rotation: [0.0, 0.0, 30.0],
				scale: [1.0, 1.0, 1.0],
				opacity: 1.0,
			});
		let instance = composition.add_instance(core::NodeId::new(composition.len()));
		instance.name = rect.name;
		instance.start_time = rect.start_time;
		instance.duration = rect.duration;

		composition
	}

	/// CoreロジックをUIスレッドで適用し、副作用のみを非同期実行する
	fn apply_core_msg(&mut self, msg: Msg) {
		let (next_model, effects) = Self::reduce_core(self.global_state.clone(), msg);
		self.global_state = next_model;
		self.handle_effects(effects);
	}

	/// Coreメッセージをモデルへ反映し、副作用を生成する
	fn reduce_core(mut model: Model, msg: Msg) -> (Model, Vec<CoreEffect>) {
		let mut effects = Vec::new();

		match msg {
			Msg::Tick => {
				if model.preview.is_playing {
					model.preview.time += 1.0 / 60.0;
					effects.push(CoreEffect::RenderFrame {
						time: model.preview.time,
						width: model.preview.width,
						height: model.preview.height,
					});
				}
			}
			Msg::SetTime(time) => {
				model.preview.time = time;
				effects.push(CoreEffect::RenderFrame {
					time: model.preview.time,
					width: model.preview.width,
					height: model.preview.height,
				});
			}
			Msg::TogglePlay => {
				model.preview.is_playing = !model.preview.is_playing;
			}
			Msg::Shutdown => {}
			Msg::FrameRendered(frame) => {
				model.preview.frame = Some(frame);
			}
			Msg::UpdateTransform(transform) => {
				model.preview.selection = Some(transform);
			}
		}

		(model, effects)
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
		let composition = Arc::clone(&self.composition);

		thread::spawn(move || {
			// Compositionをロックしてレンダリング
			let comp = composition.lock().unwrap();
			let img_buffer = renderer::render_frame_with_composition(&comp, time, width, height);
			drop(comp); // ロックを早期解放

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
			self.global_state.preview.fps = self.frame_count as f32 / elapsed;
			self.frame_count = 0;
			self.fps_update_time = now;
		}
	}

	fn sync_timeline_changes_to_composition(&mut self, source_panel_id: usize) {
		{
			let Some(source_panel) = self.timeline_panels.get(&source_panel_id) else {
				return;
			};

			let mut comp = self.composition.lock().unwrap();
			Self::apply_timeline_model_to_composition(
				&source_panel.model,
				&source_panel.clip_bindings,
				&mut comp,
			);
		}

		{
			let comp = self.composition.lock().unwrap();
			for (panel_id, panel) in &mut self.timeline_panels {
				if *panel_id == source_panel_id {
					continue;
				}
				let (model, clip_bindings) = Self::build_timeline_model_from_composition(&comp);
				panel.model = model;
				panel.clip_bindings = clip_bindings;
				panel.state.clear_selection();
			}
		}
	}

	fn handle_timeline_message(
		&mut self,
		panel_id: usize,
		message: timeline_panel::TimelineMessage,
	) {
		let timeline_update = {
			let current_time = self.global_state.preview.time;
			let panel = self.ensure_timeline_panel_state(panel_id);
			panel
				.state
				.apply_message(&mut panel.model, message, current_time)
		};

		if let Some(time) = timeline_update.playhead_time {
			self.apply_core_msg(Msg::SetTime(time));
		}

		if timeline_update.clip_modified {
			self.sync_timeline_changes_to_composition(panel_id);
			self.apply_core_msg(Msg::SetTime(self.global_state.preview.time));
		}
	}

	/// メッセージを処理
	pub fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::RenderCompleted(frame) => {
				self.handle_render_completed(frame);
				Task::none()
			}
			Message::TogglePlay => {
				self.apply_core_msg(Msg::TogglePlay);
				Task::none()
			}
			Message::Tick => {
				// 再生中のみフレームを進める
				if self.global_state.preview.is_playing {
					self.apply_core_msg(Msg::Tick);
					self.inspector_ui
						.evaluate(self.global_state.preview.time as f64);
				}
				Task::none()
			}
			Message::PanelSystem(msg) => {
				// アプリケーションメッセージのルーティング
				if let PanelSystemMessage::AppMessage(app_msg) = &msg {
					match app_msg {
						AppPanelMessage::Timeline { panel_id, message } => {
							self.handle_timeline_message(*panel_id, message.clone());
						}
						AppPanelMessage::Inspector(inspector_msg) => {
							self.inspector_ui.update(inspector_msg.clone());
						}
						AppPanelMessage::Project(proj_msg) => match proj_msg {
							crate::message::ProjectPaneMessage::ToggleExpand(id) => {
								if self.project_ui.expanded_ids.contains(id) {
									self.project_ui.expanded_ids.remove(id);
								} else {
									self.project_ui.expanded_ids.insert(*id);
								}
							}
							crate::message::ProjectPaneMessage::Select(id) => {
								self.project_ui.selected_id = Some(*id);
							}
							crate::message::ProjectPaneMessage::ClearSelection => {
								self.project_ui.selected_id = None;
							}
							crate::message::ProjectPaneMessage::OpenItem(_id) => {}
						},
					}
				}
				// システムメッセージはパネルシステムへ
				self.panel_system.update(msg);
				self.reconcile_timeline_panels();
				Task::none()
			}
			Message::WindowClosed(id) => {
				log::info!("App: WindowClosed event received. ID: {:?}", id);
				// レンダリングServiceへシャットダウンを通知
				if let Some(tx) = self.render_shutdown_tx.take()
					&& let Err(e) = tx.send(())
				{
					log::error!("App: Failed to send render shutdown: {:?}", e);
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
			.view(|panel_id, content| self.view_panel_content(panel_id, content));

		container(panel_view.map(Message::PanelSystem))
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	/// パネルコンテンツをレンダリング
	fn view_panel_content<'a>(
		&'a self,
		panel_id: usize,
		content: &PanelContent,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		match content {
			PanelContent::MainPreview => panels::preview::view(&self.global_state.preview),
			PanelContent::Timeline => {
				let timeline_state = self
					.timeline_panels
					.get(&panel_id)
					.expect("timeline panel state should be initialized");
				panels::timeline::view(
					panel_id,
					&timeline_state.state,
					&timeline_state.model,
					self.global_state.preview.time,
				)
			}
			PanelContent::Inspector => inspector_panel::view(&self.inspector_ui)
				.map(|msg| PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(msg))),
			PanelContent::Project => panels::project::view(&self.project, &self.project_ui),
		}
	}

	/// サブスクリプション
	pub fn subscription(&self) -> Subscription<Message> {
		let render_rx = self.render_rx.clone();
		let shutdown_rx = self.render_shutdown_rx.clone();
		let render_subscription = Subscription::run_with(
			RenderConnection(render_rx, shutdown_rx),
			build_render_stream,
		);

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
		let tick_subscription: Subscription<Message> = if self.global_state.preview.is_playing {
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
