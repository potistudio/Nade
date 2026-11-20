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
use inspector_panel::{EffectEdit, InspectorMessage, PropertyGroup};
use panel_system::{PanelSystem, PanelSystemMessage};
use std::path::PathBuf;
use std::sync::Arc;
use timeline_panel::{TimelineClip, TimelineInteraction, TimelineMessage, TimelineModel, TimelineTrack};

use core::{AssetId, CompositionId, CoreEffect, FrameData, InstanceId, MediaInfo, Model, Msg, update};
use domain::{
	AssetType, Instance, InstanceContent, MediaKind, MediaSource, Project, TransformAnimation, TransformProperty,
};
use renderer::Scene;

use crate::history::History;
use crate::message::{
	AppPanelMessage, EditCommand, MenuMessage, Message, PreviewMessage, ShellMessage, TransportMessage,
};
use crate::panel_content::PanelContent;
use crate::panels;
use crate::services::export::ExportTask;
use crate::services::render_service::{RenderConnection, build_render_stream};
use crate::services::render_worker::{RenderJob, RenderWorker};
use crate::shell::menu::{self, Entry, MenuKind, OpenMenu};
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
/// 最後に見たカーソル位置（右クリックメニューの表示位置）。f32 2 つを詰めて持つ
static CURSOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn store_cursor(position: iced::Point) {
	let bits = (u64::from(position.x.to_bits()) << 32) | u64::from(position.y.to_bits());
	CURSOR.store(bits, std::sync::atomic::Ordering::Relaxed);
}

fn last_cursor() -> iced::Point {
	let bits = CURSOR.load(std::sync::atomic::Ordering::Relaxed);
	iced::Point::new(f32::from_bits((bits >> 32) as u32), f32::from_bits(bits as u32))
}

fn on_window_event(event: iced::Event, status: iced::event::Status, _id: iced::window::Id) -> Option<Message> {
	match event {
		iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
			store_cursor(position);
			None
		}
		// 右クリック: 押した対象を選択したうえでコンテキストメニューを開く
		iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
			Some(Message::Menu(MenuMessage::OpenContext(last_cursor())))
		}
		iced::Event::Window(iced::window::Event::Resized(size)) => Some(Message::WindowResized(size)),
		iced::Event::Window(iced::window::Event::FileDropped(path)) => Some(Message::FileDropped(path)),
		// 押下（キャプチャ済みを含む）でポインタ下のパネルへフォーカス
		iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_)) => {
			Some(Message::PanelSystem(PanelSystemMessage::FocusPointer))
		}
		// パネル外での解放でもドラッグを終える（パネル内の解放はドラッグオーバーレイが処理する）
		iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
			Some(Message::PanelSystem(PanelSystemMessage::PointerReleased))
		}
		iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => Some(Message::PointerReleased),
		// Esc は、ドラッグ中のパネル操作を取り消せるよう処理済みでも通知する
		iced::Event::Keyboard(keyboard::Event::KeyPressed {
			key: keyboard::Key::Named(keyboard::key::Named::Escape),
			modifiers,
			..
		}) if !modifiers.command() && !modifiers.alt() && !modifiers.control() => {
			Some(Message::EscapePressed(status == iced::event::Status::Ignored))
		}
		iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
			if status == iced::event::Status::Ignored =>
		{
			shortcut(&key, modifiers)
		}
		_ => None,
	}
}

/// After Effects / Premiere 風のショートカット
fn shortcut(key: &keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
	use keyboard::key::Named;

	if modifiers.command() {
		let keyboard::Key::Character(c) = key.as_ref() else {
			return None;
		};
		let edit = |command| Some(Message::Edit(command));
		return match (c.to_ascii_lowercase().as_str(), modifiers.shift()) {
			("z", false) => edit(EditCommand::Undo),
			("z", true) => edit(EditCommand::Redo),
			("k", false) | ("d", true) => edit(EditCommand::Split),
			("d", false) => edit(EditCommand::Duplicate),
			("c", false) => edit(EditCommand::Copy),
			("c", true) => edit(EditCommand::Precompose),
			("x", false) => edit(EditCommand::Cut),
			("v", false) => edit(EditCommand::Paste),
			("y", false) => Some(Message::Shell(ShellMessage::NewSolid)),
			("i", false) => Some(Message::Shell(ShellMessage::ImportMedia)),
			("m", false) | ("e", false) => Some(Message::Shell(ShellMessage::Export)),
			("s", false) => Some(Message::Shell(ShellMessage::SaveProject)),
			("s", true) => Some(Message::Shell(ShellMessage::SaveProjectAs)),
			("o", false) => Some(Message::Shell(ShellMessage::OpenProject)),
			_ => None,
		};
	}
	if modifiers.alt() && !modifiers.control() {
		return match key.as_ref() {
			keyboard::Key::Character("[") => Some(Message::Edit(EditCommand::TrimInPoint)),
			keyboard::Key::Character("]") => Some(Message::Edit(EditCommand::TrimOutPoint)),
			_ => None,
		};
	}
	if modifiers.control() {
		return None;
	}

	match key.as_ref() {
		keyboard::Key::Named(Named::Space) => Some(Message::Transport(TransportMessage::TogglePlay)),
		keyboard::Key::Named(Named::Home) => Some(Message::Transport(TransportMessage::GoToStart)),
		keyboard::Key::Named(Named::End) => Some(Message::Transport(TransportMessage::GoToEnd)),
		keyboard::Key::Named(Named::ArrowLeft) => Some(Message::Transport(TransportMessage::StepBackward)),
		keyboard::Key::Named(Named::ArrowRight) => Some(Message::Transport(TransportMessage::StepForward)),
		keyboard::Key::Named(Named::PageUp) => Some(Message::Transport(TransportMessage::StepBackward)),
		keyboard::Key::Named(Named::PageDown) => Some(Message::Transport(TransportMessage::StepForward)),
		keyboard::Key::Named(Named::F9) => Some(Message::Edit(EditCommand::EasyEase)),
		keyboard::Key::Character(c) if !modifiers.shift() => match c {
			"b" => Some(Message::Edit(EditCommand::WorkAreaStart)),
			"n" => Some(Message::Edit(EditCommand::WorkAreaEnd)),
			"[" => Some(Message::Edit(EditCommand::MoveInPoint)),
			"]" => Some(Message::Edit(EditCommand::MoveOutPoint)),
			"i" => Some(Message::Transport(TransportMessage::GoToLayerIn)),
			"o" => Some(Message::Transport(TransportMessage::GoToLayerOut)),
			"j" => Some(Message::Transport(TransportMessage::PreviousEditPoint)),
			"k" => Some(Message::Transport(TransportMessage::NextEditPoint)),
			_ => Tool::from_key(c).map(|tool| Message::Shell(ShellMessage::SelectTool(tool))),
		},
		keyboard::Key::Named(Named::Delete | Named::Backspace) => Some(Message::Edit(EditCommand::Delete)),
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

/// Keys being dragged on the timeline, pinned when the drag starts.
struct KeyDragSession {
	clip_id: usize,
	origin: f32,
	keys: Vec<(TransformProperty, u64)>,
	/// `(effect id, slot, key id)` of effect keys on `origin`.
	effect_keys: Vec<(u32, usize, u64)>,
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

	/// ワーカーが埋まっていて捨てたレンダリング要求があるか（完了時に描き直す）
	render_dirty: bool,
	/// 直近の再生ティック時刻（実時間で再生を進めるため）
	last_play_tick: Option<Instant>,
	/// タイムライン上でドラッグ中のキー
	key_drag: Option<KeyDragSession>,

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
	/// 開いているメニュー（メニューバー or コンテキスト）
	menu: Option<OpenMenu>,
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
	inspector_ui: inspector_panel::InspectorUi,
	/// Undo / Redo 履歴
	history: History,
	/// コピーしたレイヤー（貼り付け時はクリップ群の先頭を再生ヘッドに合わせる）
	clipboard: Vec<Instance>,
	/// マウスボタン押下中（ドラッグ中は履歴を確定しない）
	pointer_down: bool,
	/// 実行中の書き出し
	export: Option<ExportTask>,
	/// 再生中の音声
	audio: audio::AudioPreview,
	/// 今回の再生で音声の開始を試みた時刻と再生位置（シーク検出に使う）
	audio_started: Option<(Instant, f32)>,
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

		let project_for_history = project.snapshot();
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
			render_dirty: false,
			last_play_tick: None,
			key_drag: None,
			frame_count: 0,
			fps_update_time: Instant::now(),
			render_fps: 0.0,
			tool: Tool::default(),
			tool_pill: pill_animation(Tool::default().index()),
			workspace: Workspace::default(),
			loop_playback: true,
			toast: None,
			menu: None,
			now: Instant::now(),
			started: Instant::now(),
			window_size,
			viewer: ViewerSettings::default(),
			viewer_size: Size::new(960.0, 540.0),
			inspector_ui: inspector_panel::InspectorUi::default(),
			history: History::with_snapshot(project_for_history),
			clipboard: Vec::new(),
			pointer_down: false,
			export: None,
			audio: audio::AudioPreview::new(),
			audio_started: None,
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
			// コンポ名ならコンポを選択
			if let Some(composition) = project
				.compositions()
				.into_iter()
				.find(|id| project.composition(id).is_some_and(|comp| comp.name() == name))
			{
				return Some(AppPanelMessage::Project(ProjectPaneMessage::Select(
					ObjectId::Composition(composition),
				)));
			}
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
			rect.set_content(InstanceContent::solid(320.0, 180.0, [0.29, 0.47, 0.96, 1.0]));

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

	/// レンダリング要求をワーカーへ送る。キューが満杯なら `false` を返し、
	/// 現在のフレームが描き終わった時点で描き直す。
	fn spawn_render(&mut self, time: f32, _width: u32, _height: u32) -> bool {
		let Some(comp) = self.target_composition().and_then(|id| self.project.composition(&id)) else {
			return true;
		};
		let model = &self.timeline.model;
		let scene = Scene::from_composition(
			comp,
			time,
			self.viewer.resolution.factor(),
			|track| {
				// タイムライン未作成のトラックは描画する
				track >= model.tracks.len() || model.is_track_rendered(track)
			},
			&self.project,
		);

		let submitted = self.render_worker.try_submit(RenderJob { scene });
		self.render_dirty = !submitted;
		submitted
	}

	/// バックグラウンドレンダリング完了を処理する
	fn handle_render_completed(&mut self, frame: FrameData) {
		self.apply_core_msg(Msg::FrameRendered(frame));
		if self.render_dirty && !self.current_model.preview.is_playing {
			self.request_preview_refresh();
		}

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
		match &message {
			// 左ボタン解放はパネルドラッグ終了メッセージで届くため、ここでも共通の解放処置を行う
			Message::PanelSystem(PanelSystemMessage::FocusPointer) => self.pointer_down = true,
			Message::PointerReleased | Message::PanelSystem(PanelSystemMessage::PointerReleased) => {
				self.pointer_down = false;
				self.key_drag = None;
			}
			_ => {}
		}
		let records_history = !matches!(
			message,
			Message::Tick | Message::RenderCompleted(_) | Message::Edit(EditCommand::Undo | EditCommand::Redo)
		);
		#[cfg(target_os = "macos")]
		let sync_menu = !matches!(message, Message::Tick | Message::RenderCompleted(_));
		let task = self.update_inner(message);
		self.sync_audio();
		// システムメニューバーは起動完了後（最初の更新）に設置し、以降は有効状態だけ追従
		#[cfg(target_os = "macos")]
		if sync_menu || !shell::native_menu::is_installed() {
			shell::native_menu::sync(|kind| self.menu_entries(kind));
		}
		// ドラッグ中の連続した変更は、ボタンを離した時点で 1 ステップにまとめる
		if records_history && !self.pointer_down {
			self.history.checkpoint(&self.project);
		}
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

			Message::Edit(command) => {
				self.apply_edit(command);
				Task::none()
			}

			Message::PointerReleased => Task::none(),

			Message::EscapePressed(unhandled) => {
				if self.panel_system.is_move_dragging() {
					self.panel_system
						.update(PanelSystemMessage::<_, AppPanelMessage>::CancelDrag);
					Task::none()
				} else if unhandled {
					self.update_inner(Message::Shell(ShellMessage::CancelExport))
				} else {
					Task::none()
				}
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
					// タイマーの遅れに左右されないよう実時間で進める（大きな停止は 1/10 秒まで）
					let wall = self
						.last_play_tick
						.map_or(1.0 / 60.0, |last| self.now.duration_since(last).as_secs_f32())
						.min(0.1);
					self.last_play_tick = Some(self.now);
					// 音声があれば音声の再生位置に映像を合わせる
					let delta = match self.audio_clock() {
						Some(clock) => (clock - self.current_model.preview.time).max(0.0),
						None => wall,
					};
					if delta > 0.0 {
						self.apply_core_msg(Msg::Advance(delta));
					}
					self.wrap_playback();
				} else {
					self.last_play_tick = None;
				}
				self.poll_export();
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

			Message::ProjectChosen(path) => {
				if let Some(path) = path {
					self.open_project(path);
				}
				Task::none()
			}

			Message::SaveProjectTo(path) => {
				if let Some(path) = path {
					let path = if path.extension().is_none() {
						path.with_extension("nade")
					} else {
						path
					};
					match self.project.save_as(&path) {
						Ok(()) => self.show_toast(Icon::Save, format!("Saved “{}”", self.project.name())),
						Err(error) => {
							log::error!("Save failed: {error}");
							self.show_toast(Icon::Close, "Save failed — see log");
						}
					}
				}
				Task::none()
			}

			Message::ExportTo(path) => {
				if let Some(path) = path {
					self.start_export(path);
				}
				Task::none()
			}

			Message::MediaImported(files) => {
				for (path, info) in files {
					self.import_media_path(path, info);
				}
				Task::none()
			}

			Message::Menu(message) => self.apply_menu_message(message),

			Message::FileDropped(path) => {
				if path
					.extension()
					.is_some_and(|extension| extension.eq_ignore_ascii_case("nade"))
				{
					self.open_project(path);
					return Task::none();
				}
				if domain::AssetType::from_path(&path).is_none() {
					self.show_toast(Icon::Close, "Unsupported file type");
					return Task::none();
				}
				// ffprobe はブロッキングなので UI スレッドの外で回す
				Task::perform(
					async move {
						let info = media::probe(&path).map_err(|error| format!("{error:#}"));
						vec![(path, info)]
					},
					Message::MediaImported,
				)
			}

			Message::CloseRequested(_id) => {
				if let Some(export) = self.export.take() {
					export.cancel();
				}
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
			ShellMessage::NewSolid => {
				self.add_solid_layer();
				Task::none()
			}
			ShellMessage::ImportMedia => Self::pick_media_file(),
			ShellMessage::OpenProject => Task::perform(
				async {
					rfd::AsyncFileDialog::new()
						.set_title("Open Project")
						.add_filter("Nade Project", &["nade"])
						.pick_file()
						.await
						.map(|handle| handle.path().to_path_buf())
				},
				Message::ProjectChosen,
			),
			ShellMessage::SaveProject if self.project.path().is_some() => {
				match self.project.save() {
					Ok(()) => self.show_toast(Icon::Save, format!("Saved “{}”", self.project.name())),
					Err(error) => {
						log::error!("Save failed: {error}");
						self.show_toast(Icon::Close, "Save failed — see log");
					}
				}
				Task::none()
			}
			ShellMessage::SaveProject | ShellMessage::SaveProjectAs => {
				let name = self.project.name().to_string();
				Task::perform(
					async move {
						rfd::AsyncFileDialog::new()
							.set_title("Save Project")
							.set_file_name(format!("{name}.nade"))
							.add_filter("Nade Project", &["nade"])
							.save_file()
							.await
							.map(|handle| handle.path().to_path_buf())
					},
					Message::SaveProjectTo,
				)
			}
			ShellMessage::Export => {
				if self.export.is_some() {
					self.show_toast(Icon::Export, "An export is already running");
					return Task::none();
				}
				let Some(name) = self.target_composition_name() else {
					self.show_toast(Icon::Close, "Select a composition to export");
					return Task::none();
				};
				Task::perform(
					async move {
						rfd::AsyncFileDialog::new()
							.set_title("Export Composition")
							.set_file_name(format!("{name}.mp4"))
							.add_filter("MPEG-4 Movie", &["mp4"])
							.add_filter("QuickTime Movie", &["mov"])
							.save_file()
							.await
							.map(|handle| handle.path().to_path_buf())
					},
					Message::ExportTo,
				)
			}
			ShellMessage::CancelExport => {
				// Esc はまず開いているメニューを閉じる
				if self.menu.take().is_some() {
					return Task::none();
				}
				if let Some(export) = &self.export {
					export.cancel();
				}
				Task::none()
			}
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
				// ワークエリアの外や終端で再生したらワークエリアの先頭から
				let [start, end] = self.work_area();
				let last = core::timecode::frame_at(end, fps).saturating_sub(1);
				let outside = time < start - 1e-4 || core::timecode::frame_at(time, fps) >= last;
				if !self.current_model.preview.is_playing && outside {
					self.apply_core_msg(Msg::SetTime(start));
				}
				self.apply_core_msg(Msg::TogglePlay);
				self.last_play_tick = self.current_model.preview.is_playing.then_some(self.now);
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
			TransportMessage::GoToLayerIn | TransportMessage::GoToLayerOut => {
				let Some(layer) = self.focused_layer() else {
					return;
				};
				let target = if matches!(transport, TransportMessage::GoToLayerIn) {
					layer.start_time()
				} else {
					// アウトポイントはレイヤー最後のフレーム
					(layer.end_time() - 1.0 / fps.max(1.0)).max(layer.start_time())
				};
				self.apply_core_msg(Msg::SetTime(target.clamp(0.0, duration)));
			}
			TransportMessage::PreviousEditPoint | TransportMessage::NextEditPoint => {
				let points = self.edit_points();
				let target = if matches!(transport, TransportMessage::NextEditPoint) {
					points.into_iter().filter(|point| *point > time + 1e-4).reduce(f32::min)
				} else {
					points.into_iter().filter(|point| *point < time - 1e-4).reduce(f32::max)
				};
				if let Some(target) = target {
					let frame = core::timecode::frame_at(target.clamp(0.0, duration), fps).min(last_frame);
					self.apply_core_msg(Msg::SetTime(core::timecode::time_of_frame(frame, fps)));
				}
			}
		}
	}

	/// The layer I / O and F9 act on: the first selected layer of the shown comp.
	fn focused_layer(&self) -> Option<&domain::Instance> {
		let composition = self.target_composition()?;
		let (_, instance) = self
			.selected_instances()
			.into_iter()
			.find(|(comp, _)| *comp == composition)?;
		self.project.composition(&composition)?.get(instance)
	}

	/// J / K stops: comp and work area bounds, layer in / out points of shown
	/// tracks and every keyframe.
	fn edit_points(&self) -> Vec<f32> {
		let Some(comp) = self.target_composition().and_then(|id| self.project.composition(&id)) else {
			return Vec::new();
		};
		let model = &self.timeline.model;
		let mut points = vec![0.0, comp.duration()];
		points.extend(comp.work_area());
		for layer in comp
			.all_objects()
			.filter(|layer| layer.track_index() >= model.tracks.len() || model.is_track_rendered(layer.track_index()))
		{
			points.extend([layer.start_time(), layer.end_time()]);
			points.extend(layer.key_times());
		}
		points
	}

	/// 再生がワークエリアの終端を越えたらループ or 停止
	fn wrap_playback(&mut self) {
		let [start, end] = self.work_area();
		let time = self.current_model.preview.time;
		if time < end {
			return;
		}
		if self.loop_playback {
			let length = (end - start).max(f32::EPSILON);
			self.apply_core_msg(Msg::SetTime(start + (time - start) % length));
		} else {
			self.apply_core_msg(Msg::TogglePlay);
			let fps = self.composition_fps();
			let last_frame = core::timecode::frame_at(end, fps).saturating_sub(1);
			self.apply_core_msg(Msg::SetTime(core::timecode::time_of_frame(last_frame, fps)));
		}
	}

	/// プロジェクトを読み込み、UI 状態を作り直す
	fn open_project(&mut self, path: PathBuf) {
		let project = match Project::open(&path) {
			Ok(project) => project,
			Err(error) => {
				log::error!("Failed to open {}: {error}", path.display());
				self.show_toast(Icon::Close, "Couldn’t open the project");
				return;
			}
		};
		if self.current_model.preview.is_playing {
			self.apply_core_msg(Msg::TogglePlay);
		}
		self.project = project;
		self.history.reset(&self.project);
		self.clipboard.clear();
		self.project_pane = ProjectPaneState::default();
		self.asset_browser = AssetBrowserState::default();
		for folder in self.project.root_assets() {
			self.asset_browser.expanded_ids.insert(folder);
		}
		self.timeline.state.clear_selection();
		self.timeline.model.tracks.clear();
		self.current_model.preview.time = 0.0;

		match self.project.compositions().into_iter().min() {
			Some(composition) => {
				self.project_pane.expanded_ids.insert(composition);
				self.select_object(ObjectId::Composition(composition));
			}
			None => self.request_preview_refresh(),
		}
		self.show_toast(Icon::Open, format!("Opened “{}”", self.project.name()));
	}

	fn start_export(&mut self, path: PathBuf) {
		let Some(composition) = self
			.target_composition()
			.and_then(|id| self.project.composition(&id))
			.cloned()
		else {
			return;
		};
		let track_rendered = (0..self.timeline.model.tracks.len())
			.map(|track| self.timeline.model.is_track_rendered(track))
			.collect();
		log::info!("Exporting “{}” to {}", composition.name(), path.display());
		// ネストしたコンポも描けるよう、全コンポのコピーを渡す
		let library = self
			.project
			.compositions()
			.iter()
			.filter_map(|id| self.project.composition(id).cloned())
			.collect();
		self.export = Some(ExportTask::start(composition, library, track_rendered, path));
		self.show_toast(Icon::Export, "Export started  ·  Esc to cancel");
	}

	/// 書き出しの完了を確認して通知する
	fn poll_export(&mut self) {
		let Some(result) = self.export.as_ref().and_then(ExportTask::take_result) else {
			return;
		};
		let Some(export) = self.export.take() else {
			return;
		};
		let name = export
			.path()
			.file_name()
			.map(|name| name.to_string_lossy().into_owned())
			.unwrap_or_default();
		match result {
			Ok(()) => {
				log::info!("Exported {}", export.path().display());
				self.show_toast(Icon::Check, format!("Exported “{name}”"));
			}
			Err(_) if export.is_cancelled() => self.show_toast(Icon::Close, "Export cancelled"),
			Err(error) => {
				log::error!("Export failed: {error}");
				self.show_toast(Icon::Close, "Export failed — see log");
			}
		}
	}

	/// 音声の再生位置。音声の起動待ち（0.5 秒まで）は再生ヘッドを止めておく。
	fn audio_clock(&self) -> Option<f32> {
		if !self.audio.is_playing() {
			return None;
		}
		match self.audio.clock() {
			Some(clock) => Some(clock),
			None if self
				.audio_started
				.is_some_and(|(started, _)| self.now.duration_since(started).as_secs_f32() < 0.5) =>
			{
				Some(self.current_model.preview.time)
			}
			None => None,
		}
	}

	/// 再生状態に合わせて音声を開始・停止し、シークされたら鳴らし直す
	fn sync_audio(&mut self) {
		if !self.current_model.preview.is_playing {
			if self.audio.is_playing() {
				self.audio.stop();
			}
			self.audio_started = None;
			return;
		}
		let time = self.current_model.preview.time;
		if let Some((started, from)) = self.audio_started {
			// 音声が鳴っていればその位置、無ければ開始からの経過時間と比べる
			let expected = self
				.audio
				.clock()
				.unwrap_or_else(|| from + self.now.duration_since(started).as_secs_f32());
			let waiting = self.audio.is_playing() && self.audio.clock().is_none();
			if waiting || (expected - time).abs() <= 0.3 {
				return;
			}
		}
		let Some(comp) = self.target_composition().and_then(|id| self.project.composition(&id)) else {
			return;
		};
		let model = &self.timeline.model;
		let clips = renderer::export_settings(
			comp,
			std::path::Path::new(""),
			[0.0, comp.duration()],
			|track| track >= model.tracks.len() || model.is_track_rendered(track),
			&self.project,
		)
		.audio;
		let until = comp.work_area()[1];
		self.audio_started = Some((self.now, time));
		self.audio.play(&clips, time, until);
	}

	fn composition_fps(&self) -> f32 {
		self.target_composition()
			.and_then(|id| self.project.composition(&id))
			.map_or(30.0, |comp| comp.fps())
	}

	/// Work area of the shown composition (whole timeline without one).
	fn work_area(&self) -> [f32; 2] {
		self.target_composition()
			.and_then(|id| self.project.composition(&id))
			.map_or([0.0, self.composition_duration()], |comp| comp.work_area())
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

	// -------------------------------------------------------------------------
	// 編集コマンド
	// -------------------------------------------------------------------------

	fn apply_edit(&mut self, command: EditCommand) {
		match command {
			EditCommand::Undo => {
				if self.history.undo(&mut self.project) {
					self.after_history_change();
					self.show_toast(Icon::Undo, "Undo");
				} else {
					self.show_toast(Icon::Undo, "Nothing to undo");
				}
			}
			EditCommand::Redo => {
				if self.history.redo(&mut self.project) {
					self.after_history_change();
					self.show_toast(Icon::Redo, "Redo");
				} else {
					self.show_toast(Icon::Redo, "Nothing to redo");
				}
			}
			EditCommand::Split => self.split_at_playhead(),
			EditCommand::Duplicate => self.duplicate_selection(),
			EditCommand::Copy => {
				self.copy_selection();
			}
			EditCommand::Cut => {
				if self.copy_selection() {
					self.delete_selected_layers();
				}
			}
			EditCommand::Paste => self.paste_clipboard(),
			EditCommand::Delete => self.delete_selected_layers(),
			EditCommand::Precompose => self.precompose_selection(),
			EditCommand::WorkAreaStart | EditCommand::WorkAreaEnd => self.set_work_area_edge(command),
			EditCommand::MoveInPoint
			| EditCommand::MoveOutPoint
			| EditCommand::TrimInPoint
			| EditCommand::TrimOutPoint => self.retime_selection(command),
			EditCommand::EasyEase => self.easy_ease_selection(),
			EditCommand::ClearWorkArea => {
				if let Some(comp) = self
					.target_composition()
					.and_then(|id| self.project.composition_mut(&id))
				{
					comp.clear_work_area();
				}
			}
		}
	}

	fn apply_menu_message(&mut self, message: MenuMessage) -> Task<Message> {
		match message {
			MenuMessage::ToggleBar(kind) => {
				self.menu = match self.menu {
					Some(open) if open.kind == kind => None,
					_ => Some(OpenMenu {
						kind,
						position: menu::bar_menu_position(kind, TITLE_BAR_HEIGHT),
					}),
				};
			}
			MenuMessage::HoverBar(kind) => {
				if let Some(open) = self.menu
					&& open.kind != MenuKind::Context
					&& open.kind != kind
				{
					self.menu = Some(OpenMenu {
						kind,
						position: menu::bar_menu_position(kind, TITLE_BAR_HEIGHT),
					});
				}
			}
			MenuMessage::OpenContext(position) => {
				// メニューの外での右クリックは閉じるだけ
				if self.menu.take().is_none() {
					self.menu = Some(OpenMenu {
						kind: MenuKind::Context,
						position,
					});
				}
			}
			MenuMessage::Close => self.menu = None,
			MenuMessage::Run(message) => {
				self.menu = None;
				return self.update_inner(*message);
			}
			#[cfg(target_os = "macos")]
			MenuMessage::Native(id) => {
				self.menu = None;
				if let Some(action) = shell::native_menu::action(&id, |kind| self.menu_entries(kind)) {
					return self.update_inner(action);
				}
			}
		}
		Task::none()
	}

	/// Items of a menu, enabled for the current selection.
	fn menu_entries(&self, kind: MenuKind) -> Vec<Entry> {
		let edit = Message::Edit;
		let shell = Message::Shell;
		let transport = Message::Transport;
		let preview = |message| Message::PanelSystem(PanelSystemMessage::AppMessage(AppPanelMessage::Preview(message)));
		let composition = self.target_composition().and_then(|id| self.project.composition(&id));
		let has_comp = composition.is_some();
		let layer = self.focused_layer();
		let selected = layer.is_some();
		let nested = layer.is_some_and(|layer| layer.content().as_composition().is_some());
		let parented = layer.is_some_and(|layer| layer.parent.is_some());
		let can_paste = has_comp && !self.clipboard.is_empty();
		let work_area = composition.is_some_and(|comp| comp.has_work_area());
		let remove_parent = Message::PanelSystem(PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(
			inspector_panel::InspectorMessage::SetParent(None),
		)));
		let open_nested = Message::PanelSystem(PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(
			inspector_panel::InspectorMessage::OpenNestedComposition,
		)));

		match kind {
			MenuKind::File => vec![
				Entry::item("New Composition", "", shell(ShellMessage::NewComposition)),
				Entry::Separator,
				Entry::item("Open Project…", "⌘O", shell(ShellMessage::OpenProject)),
				Entry::item("Save", "⌘S", shell(ShellMessage::SaveProject)),
				Entry::item("Save As…", "⇧⌘S", shell(ShellMessage::SaveProjectAs)),
				Entry::Separator,
				Entry::item("Import Media…", "⌘I", shell(ShellMessage::ImportMedia)),
				Entry::when(has_comp, "Export…", "⌘M", shell(ShellMessage::Export)),
			],
			MenuKind::Edit => vec![
				Entry::item("Undo", "⌘Z", edit(EditCommand::Undo)),
				Entry::item("Redo", "⇧⌘Z", edit(EditCommand::Redo)),
				Entry::Separator,
				Entry::when(selected, "Cut", "⌘X", edit(EditCommand::Cut)),
				Entry::when(selected, "Copy", "⌘C", edit(EditCommand::Copy)),
				Entry::when(can_paste, "Paste", "⌘V", edit(EditCommand::Paste)),
				Entry::when(selected, "Duplicate", "⌘D", edit(EditCommand::Duplicate)),
				Entry::when(has_comp, "Split Layer", "⇧⌘D", edit(EditCommand::Split)),
				Entry::when(selected, "Delete", "⌫", edit(EditCommand::Delete)),
			],
			MenuKind::Composition => vec![
				Entry::item("New Composition", "", shell(ShellMessage::NewComposition)),
				Entry::Separator,
				Entry::when(has_comp, "Set Work Area Start", "B", edit(EditCommand::WorkAreaStart)),
				Entry::when(has_comp, "Set Work Area End", "N", edit(EditCommand::WorkAreaEnd)),
				Entry::when(work_area, "Clear Work Area", "", edit(EditCommand::ClearWorkArea)),
				Entry::Separator,
				Entry::item("Go to Start", "Home", transport(TransportMessage::GoToStart)),
				Entry::item("Go to End", "End", transport(TransportMessage::GoToEnd)),
				Entry::item("Loop Playback", "", transport(TransportMessage::ToggleLoop)),
				Entry::Separator,
				Entry::when(has_comp, "Export…", "⌘M", shell(ShellMessage::Export)),
			],
			MenuKind::Layer => vec![
				Entry::when(has_comp, "New Text Layer", "", shell(ShellMessage::NewTextLayer)),
				Entry::when(has_comp, "New Solid", "⌘Y", shell(ShellMessage::NewSolid)),
				Entry::Separator,
				Entry::when(selected, "Pre-compose", "⇧⌘C", edit(EditCommand::Precompose)),
				Entry::when(nested, "Open Composition", "", open_nested),
				Entry::when(parented, "Remove Parent", "", remove_parent),
				Entry::Separator,
				Entry::when(selected, "In Point to Playhead", "[", edit(EditCommand::MoveInPoint)),
				Entry::when(selected, "Out Point to Playhead", "]", edit(EditCommand::MoveOutPoint)),
				Entry::when(selected, "Trim In Point", "⌥[", edit(EditCommand::TrimInPoint)),
				Entry::when(selected, "Trim Out Point", "⌥]", edit(EditCommand::TrimOutPoint)),
				Entry::Separator,
				Entry::when(
					selected,
					"Go to In Point",
					"I",
					transport(TransportMessage::GoToLayerIn),
				),
				Entry::when(
					selected,
					"Go to Out Point",
					"O",
					transport(TransportMessage::GoToLayerOut),
				),
				Entry::when(selected, "Easy Ease", "F9", edit(EditCommand::EasyEase)),
			],
			MenuKind::Effect => {
				let inspector =
					|message| Message::PanelSystem(PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(message)));
				let has_effects = layer.is_some_and(|layer| !layer.effects.is_empty());
				let mut entries = vec![Entry::when(
					has_effects,
					"Remove All",
					"",
					inspector(InspectorMessage::RemoveAllEffects),
				)];
				// カテゴリごとに区切って並べる（AE の Effect メニューと同じ構成）
				for category in domain::EffectCategory::ALL {
					entries.push(Entry::Separator);
					entries.extend(category.effects().map(|kind| {
						Entry::when(selected, kind.name(), "", inspector(InspectorMessage::AddEffect(kind)))
					}));
				}
				entries
			}
			MenuKind::View => vec![
				Entry::item("Zoom In", "", preview(PreviewMessage::ZoomBy(1.25))),
				Entry::item("Zoom Out", "", preview(PreviewMessage::ZoomBy(1.0 / 1.25))),
				Entry::item("Fit in Viewer", "", preview(PreviewMessage::FitView)),
				Entry::item("Actual Pixels", "", preview(PreviewMessage::ActualPixels)),
				Entry::Separator,
				Entry::item("Grid", "", preview(PreviewMessage::ToggleGrid)),
				Entry::item("Safe Areas", "", preview(PreviewMessage::ToggleSafeAreas)),
				Entry::Separator,
				Entry::item("Previous Keyframe", "J", transport(TransportMessage::PreviousEditPoint)),
				Entry::item("Next Keyframe", "K", transport(TransportMessage::NextEditPoint)),
			],
			MenuKind::Context if selected => {
				let mut entries = vec![
					Entry::item("Cut", "⌘X", edit(EditCommand::Cut)),
					Entry::item("Copy", "⌘C", edit(EditCommand::Copy)),
					Entry::when(can_paste, "Paste", "⌘V", edit(EditCommand::Paste)),
					Entry::item("Duplicate", "⌘D", edit(EditCommand::Duplicate)),
					Entry::item("Split Layer", "⇧⌘D", edit(EditCommand::Split)),
					Entry::item("Delete", "⌫", edit(EditCommand::Delete)),
					Entry::Separator,
					Entry::item("Pre-compose", "⇧⌘C", edit(EditCommand::Precompose)),
				];
				if nested {
					entries.push(Entry::item("Open Composition", "", open_nested));
				}
				if parented {
					entries.push(Entry::item("Remove Parent", "", remove_parent));
				}
				entries.extend([
					Entry::Separator,
					Entry::item("In Point to Playhead", "[", edit(EditCommand::MoveInPoint)),
					Entry::item("Out Point to Playhead", "]", edit(EditCommand::MoveOutPoint)),
					Entry::item("Easy Ease", "F9", edit(EditCommand::EasyEase)),
				]);
				entries
			}
			MenuKind::Context => vec![
				Entry::when(has_comp, "New Text Layer", "", shell(ShellMessage::NewTextLayer)),
				Entry::when(has_comp, "New Solid", "⌘Y", shell(ShellMessage::NewSolid)),
				Entry::item("Import Media…", "⌘I", shell(ShellMessage::ImportMedia)),
				Entry::when(can_paste, "Paste", "⌘V", edit(EditCommand::Paste)),
				Entry::Separator,
				Entry::when(has_comp, "Set Work Area Start", "B", edit(EditCommand::WorkAreaStart)),
				Entry::when(has_comp, "Set Work Area End", "N", edit(EditCommand::WorkAreaEnd)),
			],
		}
	}

	/// B / N
	fn set_work_area_edge(&mut self, command: EditCommand) {
		let time = self.current_model.preview.time;
		let fps = self.composition_fps();
		let Some(comp) = self
			.target_composition()
			.and_then(|id| self.project.composition_mut(&id))
		else {
			return;
		};
		if command == EditCommand::WorkAreaStart {
			comp.set_work_area_start(time);
		} else {
			// 終点は再生ヘッドのフレームを含める
			comp.set_work_area_end(time + 1.0 / fps.max(1.0));
		}
		let [start, end] = comp.work_area();
		self.show_toast(
			Icon::Loop,
			format!(
				"Work area {} – {}",
				core::timecode::format(start, fps),
				core::timecode::format(end, fps)
			),
		);
	}

	/// [ / ] / Alt+[ / Alt+]
	fn retime_selection(&mut self, command: EditCommand) {
		let Some(composition) = self.target_composition() else {
			return;
		};
		let layers: Vec<InstanceId> = self
			.selected_instances()
			.into_iter()
			.filter(|(comp, _)| *comp == composition)
			.map(|(_, instance)| instance)
			.collect();
		if layers.is_empty() {
			self.show_toast(Icon::Close, "Select a layer first");
			return;
		}
		let time = self.current_model.preview.time;
		// アウトポイントは再生ヘッドのフレームの終わり
		let frame_end = time + 1.0 / self.composition_fps().max(1.0);
		let Some(comp) = self.project.composition_mut(&composition) else {
			return;
		};
		for id in &layers {
			let Some(layer) = comp.get_mut(*id) else {
				continue;
			};
			match command {
				EditCommand::MoveInPoint => layer.move_to(time),
				EditCommand::MoveOutPoint => layer.move_to(frame_end - layer.duration()),
				EditCommand::TrimInPoint => {
					layer.trim_in(time);
				}
				EditCommand::TrimOutPoint => {
					layer.trim_out(frame_end);
				}
				_ => {}
			}
		}
		self.finish_layer_edit(composition, layers.first().copied());
	}

	/// F9: the curve editor's selected key, else keys under the playhead of
	/// the selected layers, else all their keys.
	fn easy_ease_selection(&mut self) {
		let Some(composition) = self.target_composition() else {
			return;
		};
		let layers: Vec<InstanceId> = self
			.selected_instances()
			.into_iter()
			.filter(|(comp, _)| *comp == composition)
			.map(|(_, instance)| instance)
			.collect();
		let time = self.current_model.preview.time;
		let (property, key) = (self.curve_editor.selected_property, self.curve_editor.selected_key);
		let Some(comp) = self.project.composition_mut(&composition) else {
			return;
		};
		let mut count = 0;
		if let (Some(key), [layer]) = (key, layers.as_slice())
			&& let Some(layer) = comp.get_mut(*layer)
			&& layer.animation.curve_mut(property).easy_ease(key)
		{
			count = 1;
		} else {
			for id in &layers {
				if let Some(layer) = comp.get_mut(*id) {
					count += layer.easy_ease(Some(time));
				}
			}
			if count == 0 {
				for id in &layers {
					if let Some(layer) = comp.get_mut(*id) {
						count += layer.easy_ease(None);
					}
				}
			}
		}
		if count == 0 {
			self.show_toast(Icon::Close, "No keyframes to ease");
			return;
		}
		self.request_preview_refresh();
		self.show_toast(
			Icon::InterpBezier,
			format!("Easy Ease on {count} key{}", if count == 1 { "" } else { "s" }),
		);
	}

	/// 選択レイヤーをプリコンポーズし、置き換わったコンポレイヤーを選択する
	fn precompose_selection(&mut self) {
		let Some(composition) = self.target_composition() else {
			return;
		};
		let layers: Vec<InstanceId> = self
			.selected_instances()
			.into_iter()
			.filter(|(comp, _)| *comp == composition)
			.map(|(_, instance)| instance)
			.collect();
		if layers.is_empty() {
			self.show_toast(Icon::Close, "Select layers to pre-compose");
			return;
		}
		let existing: Vec<String> = self
			.project
			.compositions()
			.iter()
			.filter_map(|id| self.project.composition(id).map(|comp| comp.name().to_string()))
			.collect();
		let name = unique_numbered_name("Pre-comp", &existing);
		let time = self.current_model.preview.time;
		let Some((nested, layer)) = self.project.precompose(composition, &layers, name.clone(), time) else {
			return;
		};
		self.project_pane.expanded_ids.insert(nested);
		self.timeline.state.clear_selection();
		self.finish_layer_edit(composition, Some(layer));
		let count = layers.len();
		self.show_toast(
			Icon::Comp,
			format!(
				"Pre-composed {count} layer{} into “{name}”",
				if count == 1 { "" } else { "s" }
			),
		);
	}

	/// Undo / Redo 後に UI 側の派生状態を作り直す
	fn after_history_change(&mut self) {
		let Some(composition) = self.target_composition() else {
			return;
		};
		if self.project.composition(&composition).is_none() {
			return;
		}
		// 消えたレイヤーを選択したままにしない
		if let Some(ObjectId::Instance { instance, .. }) = self.project_pane.selected_id
			&& self
				.project
				.composition(&composition)
				.and_then(|comp| comp.get(instance))
				.is_none()
		{
			self.project_pane.selected_id = Some(ObjectId::Composition(composition));
		}
		let selected = self.project_pane.selected_id;
		self.rebuild_timeline_for_composition(composition);
		if let Some(id) = selected {
			self.sync_timeline_selection_from_object(id);
		}
		self.request_preview_refresh();
	}

	/// 選択中のレイヤー（タイムラインの複数選択を優先）
	fn selected_instances(&self) -> Vec<(CompositionId, InstanceId)> {
		let from_timeline: Vec<_> = self
			.timeline
			.state
			.selected_clip_ids()
			.into_iter()
			.filter_map(|(track_index, clip_id)| {
				let clip = self
					.timeline
					.model
					.tracks
					.get(track_index)?
					.clips
					.iter()
					.find(|clip| clip.id == clip_id)?;
				Some((clip.composition_id?, clip.instance_id?))
			})
			.collect();
		if !from_timeline.is_empty() {
			return from_timeline;
		}
		match self.project_pane.selected_id {
			Some(ObjectId::Instance { composition, instance }) => vec![(composition, instance)],
			_ => Vec::new(),
		}
	}

	/// 編集後にタイムラインを作り直し、`instance` を選択する
	fn finish_layer_edit(&mut self, composition: CompositionId, select: Option<InstanceId>) {
		self.rebuild_timeline_for_composition(composition);
		self.project_pane.expanded_ids.insert(composition);
		match select {
			Some(instance) => self.select_object(ObjectId::Instance { composition, instance }),
			None => self.project_pane.selected_id = Some(ObjectId::Composition(composition)),
		}
		self.request_preview_refresh();
	}

	fn split_at_playhead(&mut self) {
		let Some(composition) = self.target_composition() else {
			return;
		};
		let time = self.current_model.preview.time;
		let crossing = self
			.project
			.composition(&composition)
			.map(|comp| comp.instances_at(time))
			.unwrap_or_default();
		// 選択レイヤーのうち再生ヘッドを跨ぐもの。無ければ再生ヘッド下の全レイヤー
		let mut targets: Vec<InstanceId> = self
			.selected_instances()
			.into_iter()
			.filter(|(comp, instance)| *comp == composition && crossing.contains(instance))
			.map(|(_, instance)| instance)
			.collect();
		if targets.is_empty() {
			targets = crossing;
		}
		let Some(comp) = self.project.composition_mut(&composition) else {
			return;
		};
		let parts: Vec<InstanceId> = targets
			.into_iter()
			.filter_map(|instance| comp.split_instance(instance, time))
			.collect();
		if parts.is_empty() {
			self.show_toast(Icon::Scissors, "Nothing to split at the playhead");
			return;
		}
		let count = parts.len();
		self.finish_layer_edit(composition, parts.first().copied());
		let timecode = core::timecode::format(time, self.composition_fps());
		self.show_toast(
			Icon::Scissors,
			if count == 1 {
				format!("Split at {timecode}")
			} else {
				format!("Split {count} layers at {timecode}")
			},
		);
	}

	fn duplicate_selection(&mut self) {
		let selected = self.selected_instances();
		let Some(&(composition, _)) = selected.first() else {
			return;
		};
		let Some(comp) = self.project.composition_mut(&composition) else {
			return;
		};
		let copies: Vec<InstanceId> = selected
			.iter()
			.filter(|(comp_id, _)| *comp_id == composition)
			.filter_map(|(_, instance)| comp.duplicate_instance(*instance))
			.collect();
		let count = copies.len();
		self.finish_layer_edit(composition, copies.last().copied());
		self.show_toast(
			Icon::Insert,
			if count == 1 {
				"Duplicated layer".to_string()
			} else {
				format!("Duplicated {count} layers")
			},
		);
	}

	/// 選択レイヤーをクリップボードへ。コピーしたら `true`。
	fn copy_selection(&mut self) -> bool {
		let copies: Vec<Instance> = self
			.selected_instances()
			.into_iter()
			.filter_map(|(composition, instance)| self.project.composition(&composition)?.get(instance).cloned())
			.collect();
		if copies.is_empty() {
			return false;
		}
		let count = copies.len();
		self.clipboard = copies;
		self.show_toast(
			Icon::Insert,
			if count == 1 {
				"Copied layer".to_string()
			} else {
				format!("Copied {count} layers")
			},
		);
		true
	}

	/// クリップボードのレイヤーを再生ヘッド位置へ貼り付ける
	fn paste_clipboard(&mut self) {
		let Some(composition) = self.target_composition() else {
			return;
		};
		let Some(base) = self.clipboard.iter().map(Instance::start_time).reduce(f32::min) else {
			return;
		};
		let time = self.current_model.preview.time;
		let copies = self.clipboard.clone();
		let Some(comp) = self.project.composition_mut(&composition) else {
			return;
		};
		let mut last = None;
		for mut copy in copies {
			let track = copy.track_index();
			copy.move_to(time + copy.start_time() - base);
			last = Some(comp.insert_instance(copy, Some(track)));
		}
		let end = comp.content_end();
		if end > comp.duration() {
			comp.set_duration(end);
		}
		self.finish_layer_edit(composition, last);
		self.show_toast(Icon::Insert, "Pasted");
	}

	/// 選択中のレイヤーを削除（カーブエディタにフォーカスがある時はキー削除を優先）
	fn delete_selected_layers(&mut self) {
		if matches!(self.panel_system.focused_content(), Some(PanelContent::CurveEditor)) {
			return;
		}
		let selected = self.selected_instances();
		let Some(&(composition, _)) = selected.first() else {
			return;
		};
		let mut names = Vec::new();
		if let Some(comp) = self.project.composition_mut(&composition) {
			for (_, instance) in selected.iter().filter(|(comp_id, _)| *comp_id == composition) {
				if let Some(obj) = comp.get(*instance) {
					names.push(obj.name().to_string());
				}
				comp.remove(*instance);
			}
		}

		self.finish_layer_edit(composition, None);
		self.timeline.state.clear_selection();
		match names.as_slice() {
			[] => {}
			[name] => self.show_toast(Icon::Trash, format!("Deleted “{name}”")),
			_ => self.show_toast(Icon::Trash, format!("Deleted {} layers", names.len())),
		}
	}

	fn apply_timeline_message(&mut self, msg: TimelineMessage) {
		if let TimelineMessage::KeyframeDragged { clip_id, origin, time } = msg {
			self.drag_clip_keys(clip_id, origin, time);
			return;
		}
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
			if !self.timeline.state.is_dragging_clip() {
				self.materialize_timeline_duplicates();
			}
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
		// コンポレイヤーのダブルクリックは中身のコンポを開く（AE と同じ）
		if let ObjectId::Instance { composition, instance } = id
			&& let Some(nested) = self
				.project
				.composition(&composition)
				.and_then(|comp| comp.get(instance))
				.and_then(|layer| layer.content().as_composition())
			&& self.project.composition(&nested).is_some()
		{
			self.open_item(ObjectId::Composition(nested));
			return;
		}
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

		// トラックの表示・ソロ状態は作り直しても残す
		let flags: Vec<(bool, bool)> = self
			.timeline
			.model
			.tracks
			.iter()
			.map(|track| (track.visible, track.solo))
			.collect();
		self.timeline.model.tracks.clear();
		self.timeline.model.ensure_track_count(track_count);
		for (track, (visible, solo)) in self.timeline.model.tracks.iter_mut().zip(flags) {
			track.visible = visible;
			track.solo = solo;
		}

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

	/// タイムラインのクリップ位置をインスタンスへ書き戻す。
	///
	/// 左端だけが動いた（終端が変わらない）クリップはトリムとみなしてフッテージの
	/// 開始位置もずらす。尺のある素材は素材の範囲を越えないようクリップ側を戻す。
	/// Shift+ドラッグの複製クリップ（同じインスタンスを指す 2 本目）は無視する。
	fn sync_instances_from_timeline(&mut self) {
		const EPSILON: f32 = 1e-4;
		let mut seen = std::collections::HashSet::new();
		for (track_index, track) in self.timeline.model.tracks.iter_mut().enumerate() {
			for clip in &mut track.clips {
				let (Some(composition_id), Some(instance_id)) = (clip.composition_id, clip.instance_id) else {
					continue;
				};
				if !seen.insert((composition_id, instance_id)) {
					continue;
				}
				let Some(instance) = self
					.project
					.composition_mut(&composition_id)
					.and_then(|comp| comp.get_mut(instance_id))
				else {
					continue;
				};

				let trimmed_left = (clip.start_time - instance.start_time()).abs() > EPSILON
					&& (clip.end_time() - instance.end_time()).abs() <= EPSILON;
				let shift = clip.start_time - instance.start_time();
				let moved = shift.abs() > EPSILON && (clip.end_time() - instance.end_time() - shift).abs() <= EPSILON;
				if moved {
					// レイヤーを動かしたらキーフレームも一緒に動く（AE と同じ）
					instance.move_to(clip.start_time);
				}
				if trimmed_left {
					let offset = instance.source_offset + clip.start_time - instance.start_time();
					instance.source_offset = offset;
				}

				let source_length = instance
					.content()
					.is_timed_media()
					.then(|| instance.content().as_media().and_then(|source| source.info.duration))
					.flatten();
				if let Some(length) = source_length {
					if instance.source_offset < 0.0 {
						// 素材の先頭より前には伸ばせない
						clip.start_time -= instance.source_offset;
						clip.duration += instance.source_offset;
						instance.source_offset = 0.0;
					}
					clip.duration = clip.duration.min((length - instance.source_offset).max(EPSILON));
				}

				instance.set_start_time(clip.start_time);
				instance.set_duration(clip.duration);
				instance.set_track_index(track_index);
			}
		}
	}

	/// Shift+ドラッグで作られた複製クリップに専用のインスタンスを割り当てる。
	/// 複製があれば `true`（タイムラインは作り直される）。
	fn materialize_timeline_duplicates(&mut self) -> bool {
		let mut seen = std::collections::HashSet::new();
		let mut duplicates = Vec::new();
		for (track_index, track) in self.timeline.model.tracks.iter().enumerate() {
			for clip in &track.clips {
				let (Some(composition), Some(instance)) = (clip.composition_id, clip.instance_id) else {
					continue;
				};
				if !seen.insert((composition, instance)) {
					duplicates.push((composition, instance, track_index, clip.start_time));
				}
			}
		}
		let Some(&(composition, ..)) = duplicates.first() else {
			return false;
		};

		let mut last = None;
		for (composition, instance, track_index, start_time) in duplicates {
			let Some(comp) = self.project.composition_mut(&composition) else {
				continue;
			};
			let Some(mut copy) = comp.get(instance).cloned() else {
				continue;
			};
			copy.move_to(start_time);
			last = Some(comp.insert_instance(copy, Some(track_index)));
		}
		self.rebuild_timeline_for_composition(composition);
		if let Some(instance) = last {
			self.select_object(ObjectId::Instance { composition, instance });
		}
		true
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
			PreviewMessage::ScaleLayer { scale } => {
				self.edit_selected_property(TransformProperty::ScaleX, |_| scale[0]);
				self.edit_selected_property(TransformProperty::ScaleY, |_| scale[1]);
				self.request_preview_refresh();
			}
			PreviewMessage::MoveAnchor { anchor, position } => {
				self.edit_selected_property(TransformProperty::AnchorX, |value| value + anchor[0]);
				self.edit_selected_property(TransformProperty::AnchorY, |value| value + anchor[1]);
				self.edit_selected_property(TransformProperty::PositionX, |value| value + position[0]);
				self.edit_selected_property(TransformProperty::PositionY, |value| value + position[1]);
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
				let mut times: Vec<f32> = instance.key_times().collect();
				if times.is_empty() {
					return None;
				}
				times.sort_by(f32::total_cmp);
				times.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
				Some((clip.id, times))
			})
			.collect()
	}

	/// Timeline keyframe drag: keys that sat at `origin` move to `time`. The
	/// key ids are pinned when the drag starts, so keys that get clamped by
	/// their neighbours keep following the pointer.
	fn drag_clip_keys(&mut self, clip_id: usize, origin: f32, time: f32) {
		let Some((composition, instance)) = self
			.timeline
			.model
			.tracks
			.iter()
			.flat_map(|track| track.clips.iter())
			.find(|clip| clip.id == clip_id)
			.and_then(|clip| Some((clip.composition_id?, clip.instance_id?)))
		else {
			return;
		};
		let Some(layer) = self
			.project
			.composition_mut(&composition)
			.and_then(|comp| comp.get_mut(instance))
		else {
			return;
		};
		let same_drag = self
			.key_drag
			.as_ref()
			.is_some_and(|drag| drag.clip_id == clip_id && drag.origin == origin);
		if !same_drag {
			let keys = TransformProperty::ALL
				.into_iter()
				.filter_map(|property| Some((property, layer.animation.curve(property).key_at(origin)?.id)))
				.collect();
			let effect_keys = layer
				.effects
				.iter()
				.flat_map(|effect| {
					effect
						.keys_at(origin)
						.into_iter()
						.map(move |(slot, key)| (effect.id, slot, key))
				})
				.collect();
			self.key_drag = Some(KeyDragSession {
				clip_id,
				origin,
				keys,
				effect_keys,
			});
		}
		let Some(KeyDragSession { keys, effect_keys, .. }) = &self.key_drag else {
			return;
		};
		for (property, id) in keys {
			let curve = layer.animation.curve_mut(*property);
			if let Some(value) = curve.key(*id).map(|key| key.value) {
				curve.set_key(*id, time, value);
			}
		}
		for (effect, slot, key) in effect_keys {
			if let Some(effect) = layer.effect_mut(*effect) {
				effect.move_key(*slot, *key, time);
			}
		}
		if !same_drag {
			self.select_object(ObjectId::Instance { composition, instance });
		}
		self.request_preview_refresh();
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
				let parent = comp.parent_matrix(instance.id(), time);
				let size = match instance.content() {
					InstanceContent::Text {
						text,
						font_path,
						font_size,
						spacing,
						..
					} => renderer::measure_text(font_path, text, *font_size, *spacing).unwrap_or([0.0, 0.0]),
					InstanceContent::Solid { width, height, .. } => [*width, *height],
					InstanceContent::Media(source) => source.size(),
					InstanceContent::Composition(id) => self
						.project
						.composition(id)
						.map_or([0.0, 0.0], |nested| [nested.width() as f32, nested.height() as f32]),
					InstanceContent::Empty => [0.0, 0.0],
				};
				LayerBox {
					id: instance.id(),
					matrix: parent.then_apply(transform.local_matrix()),
					parent,
					size,
					anchor: [transform.anchor[0], transform.anchor[1]],
					scale: [transform.scale[0], transform.scale[1]],
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
			composition: composition.map(|comp| core::CompositionSummary {
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
			InspectorMessage::SetCompositionDuration(_)
			| InspectorMessage::FitCompositionToLayers
			| InspectorMessage::SetCompositionBackground(_) => {
				self.apply_composition_setting(msg);
				return;
			}
			InspectorMessage::SetParent(parent) => {
				self.set_selected_parent(parent);
				return;
			}
			InspectorMessage::OpenNestedComposition => {
				self.open_nested_composition();
				return;
			}
			InspectorMessage::NestInto(outer) => {
				self.nest_shown_composition(outer);
				return;
			}
			InspectorMessage::Effect {
				id,
				edit: EffectEdit::ToggleExpanded,
			} => {
				if let Some(ObjectId::Instance { instance, .. }) = self.project_pane.selected_id {
					self.inspector_ui.toggle_effect(instance, id);
				}
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
		let frame = self
			.project
			.composition(&composition)
			.map_or([1920.0, 1080.0], |comp| [comp.width() as f32, comp.height() as f32]);

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
		let mut added_effect = None;

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
			InspectorMessage::SetSolidSize { width, height } => {
				if let Some((slot_width, slot_height)) = obj.content_mut().solid_size_mut() {
					*slot_width = width.max(1.0);
					*slot_height = height.max(1.0);
				}
			}
			InspectorMessage::FitToComposition => {
				let [width, height] = match obj.content() {
					InstanceContent::Media(source) => source.size(),
					InstanceContent::Solid { width, height, .. } => [*width, *height],
					_ => [0.0, 0.0],
				};
				if width > 0.0 && height > 0.0 {
					let fit = (frame[0] / width).min(frame[1] / height);
					obj.set_property_at(TransformProperty::ScaleX, time, fit);
					obj.set_property_at(TransformProperty::ScaleY, time, fit);
					obj.set_property_at(TransformProperty::PositionX, time, 0.0);
					obj.set_property_at(TransformProperty::PositionY, time, 0.0);
					toast = Some((Icon::Fit, "Fit to composition".to_string()));
				}
			}
			InspectorMessage::SetBlendMode(mode) => obj.blend_mode = mode,
			InspectorMessage::AddEffect(kind) => {
				added_effect = Some(obj.add_effect(kind));
				toast = Some((Icon::Sparkle, format!("Added {}", kind.name())));
			}
			InspectorMessage::RemoveAllEffects => {
				let count = obj.effects.len();
				obj.effects.clear();
				toast = Some((
					Icon::Trash,
					format!("Removed {count} effect{}", if count == 1 { "" } else { "s" }),
				));
			}
			InspectorMessage::Effect { id, edit } => {
				let (result, effect_jump) = edit_effect(obj, id, edit, time, fps);
				toast = result;
				jump = effect_jump;
			}
			InspectorMessage::ToggleSection(_)
			| InspectorMessage::ToggleScaleLink
			| InspectorMessage::HexEdited(_)
			| InspectorMessage::SetCompositionDuration(_)
			| InspectorMessage::FitCompositionToLayers
			| InspectorMessage::SetCompositionBackground(_)
			| InspectorMessage::SetParent(_)
			| InspectorMessage::OpenNestedComposition
			| InspectorMessage::NestInto(_) => {}
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
		if let Some(effect) = added_effect {
			self.inspector_ui.expand_effect(instance, effect);
			if !self.inspector_ui.is_open(inspector_panel::Section::Effects) {
				self.inspector_ui.toggle(inspector_panel::Section::Effects, now);
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

	/// Parent picker: re-parent the selected layer keeping it in place.
	fn set_selected_parent(&mut self, parent: Option<InstanceId>) {
		let Some(ObjectId::Instance { composition, instance }) = self.project_pane.selected_id else {
			return;
		};
		let time = self.current_model.preview.time;
		let Some(comp) = self.project.composition_mut(&composition) else {
			return;
		};
		if !comp.set_parent(instance, parent, time) {
			self.show_toast(Icon::Close, "That parent would create a loop");
			return;
		}
		let name = parent.and_then(|id| comp.get(id)).map(|layer| layer.name().to_string());
		self.show_toast(
			Icon::Link,
			match name {
				Some(name) => format!("Parented to “{name}”"),
				None => "Parent removed".to_string(),
			},
		);
		self.request_preview_refresh();
	}

	/// Add the shown composition as the top layer of `outer`, then open `outer`
	/// with the new layer selected.
	fn nest_shown_composition(&mut self, outer: CompositionId) {
		let Some(inner) = self.target_composition() else {
			return;
		};
		if !self.project.can_nest(outer, inner) {
			self.show_toast(Icon::Close, "That would nest the composition in itself");
			return;
		}
		let Some((name, duration)) = self
			.project
			.composition(&inner)
			.map(|comp| (comp.name().to_string(), comp.duration()))
		else {
			return;
		};
		let node = self.project.create_node();
		let Some(comp) = self.project.composition_mut(&outer) else {
			return;
		};
		comp.insert_track(0);
		let mut layer = domain::Instance::new(InstanceId::new(0), node, &name, 0.0, duration);
		layer.set_content(InstanceContent::Composition(inner));
		let layer = comp.insert_instance(layer, Some(0));
		comp.compact_tracks();
		let outer_name = comp.name().to_string();
		self.finish_layer_edit(outer, Some(layer));
		self.show_toast(Icon::Comp, format!("Added “{name}” to “{outer_name}”"));
	}

	/// Show the composition the selected comp layer plays (AE: double-click).
	fn open_nested_composition(&mut self) {
		if let Some(id @ ObjectId::Instance { .. }) = self.project_pane.selected_id {
			self.open_item(id);
		}
	}

	fn apply_composition_setting(&mut self, msg: InspectorMessage) {
		let Some(comp) = self
			.target_composition()
			.and_then(|id| self.project.composition_mut(&id))
		else {
			return;
		};
		match msg {
			InspectorMessage::SetCompositionDuration(seconds) => comp.set_duration(seconds),
			InspectorMessage::FitCompositionToLayers => {
				let end = comp.content_end();
				if end > 0.0 {
					comp.set_duration(end);
				}
			}
			InspectorMessage::SetCompositionBackground([r, g, b]) => comp.set_background([r, g, b, 1.0]),
			_ => return,
		}
		// 尺を縮めたら再生ヘッドを範囲内に戻す
		let duration = self.composition_duration();
		if self.current_model.preview.time > duration {
			self.apply_core_msg(Msg::SetTime(duration));
		} else {
			self.request_preview_refresh();
		}
	}

	fn inspector_target(&self) -> inspector_panel::InspectorTarget<'_> {
		use inspector_panel::{InspectorTarget, LayerTarget};

		match self.project_pane.selected_id {
			Some(ObjectId::Instance { composition, instance }) => {
				let Some(obj) = self
					.project
					.composition(&composition)
					.and_then(|comp| comp.get(instance))
				else {
					return InspectorTarget::Empty;
				};
				let comp = self.project.composition(&composition).expect("layer found above");
				let mut candidates: Vec<&domain::Instance> = comp
					.all_objects()
					.filter(|other| comp.can_parent(instance, other.id()))
					.collect();
				candidates.sort_by_key(|other| (other.track_index(), other.id()));
				let parents = std::iter::once(inspector_panel::ParentChoice::none())
					.chain(candidates.into_iter().map(|other| inspector_panel::ParentChoice {
						id: Some(other.id()),
						name: format!("{}. {}", other.track_index() + 1, other.name()),
					}))
					.collect();
				let nested = obj
					.content()
					.as_composition()
					.and_then(|id| self.project.composition(&id))
					.map(|nested| nested.name());
				InspectorTarget::Layer(LayerTarget {
					instance: obj,
					time: self.current_model.preview.time,
					label: constants::style::label_color(obj.track_index()),
					fps: self.composition_fps(),
					parents,
					nested,
				})
			}
			Some(ObjectId::Composition(id)) => match self.project.composition(&id) {
				Some(comp) => InspectorTarget::Composition {
					summary: core::CompositionSummary {
						name: comp.name(),
						width: comp.width(),
						height: comp.height(),
						fps: comp.fps(),
						duration: comp.duration(),
					},
					layers: comp.len(),
					background: comp.background(),
					nest_targets: {
						let mut targets: Vec<_> = self
							.project
							.compositions()
							.into_iter()
							.filter(|outer| *outer != id && self.project.can_nest(*outer, id))
							.filter_map(|outer| {
								let name = self.project.composition(&outer)?.name().to_string();
								Some(inspector_panel::CompositionChoice { id: outer, name })
							})
							.collect();
						targets.sort_by_key(|choice| choice.id.value());
						targets
					},
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
				let Some(handles) = rfd::AsyncFileDialog::new()
					.set_title("Import Media")
					.add_filter(
						"Media",
						&[
							"png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "mov", "mp4", "mkv", "avi",
							"webm", "m4v", "mp3", "wav", "flac", "aac", "ogg", "m4a",
						],
					)
					.pick_files()
					.await
				else {
					return Vec::new();
				};
				// ffprobe はブロッキングなので UI スレッドの外で回す
				handles
					.into_iter()
					.map(|handle| {
						let path = handle.path().to_path_buf();
						let info = media::probe(&path).map_err(|error| format!("{error:#}"));
						(path, info)
					})
					.collect()
			},
			Message::MediaImported,
		)
	}

	fn import_media_path(&mut self, path: PathBuf, info: Result<MediaInfo, String>) {
		let name = path
			.file_name()
			.map(|name| name.to_string_lossy().into_owned())
			.unwrap_or_default();
		let info = match info {
			Ok(info) => info,
			Err(error) => {
				log::warn!("Failed to read media {}: {error}", path.display());
				self.show_toast(Icon::Close, format!("Couldn’t read “{name}”"));
				return;
			}
		};
		let parent = self.asset_import_parent();
		match self.project.import_media(&path, parent) {
			Some(id) => {
				if let Some(asset) = self.project.asset_mut(id) {
					asset.media_info = Some(info);
				}
				if let Some(parent_id) = parent {
					self.asset_browser.expanded_ids.insert(parent_id);
				}
				self.asset_browser.selected_id = Some(id);
				log::info!("Imported media: {} ({info:?})", path.display());
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
		let kind = match asset.kind() {
			AssetType::Image => MediaKind::Image,
			AssetType::Video => MediaKind::Video,
			AssetType::Audio => MediaKind::Audio,
			_ => {
				log::debug!("Add to timeline ignored for non-media asset");
				return;
			}
		};
		let name = asset.name.clone();
		let (Some(path), Some(info)) = (asset.source_path.clone(), asset.media_info) else {
			self.show_toast(Icon::Close, format!("“{name}” has no media file"));
			return;
		};
		let Some(composition_id) = self.target_composition() else {
			self.show_toast(Icon::Close, "Select a composition first");
			return;
		};

		// 動画の音声のみ・映像のみのファイルにも対応する
		let kind = match kind {
			MediaKind::Video if !info.has_video => MediaKind::Audio,
			kind => kind,
		};
		let start_time = self.current_model.preview.time;
		let duration = match kind {
			MediaKind::Image => 5.0,
			MediaKind::Video | MediaKind::Audio => info.duration.unwrap_or(5.0),
		}
		.max(1.0 / self.composition_fps().max(1.0));
		let node_id = self.project.create_node();
		let Some(comp) = self.project.composition_mut(&composition_id) else {
			return;
		};
		let (comp_width, comp_height) = (comp.width() as f32, comp.height() as f32);
		let instance = comp.add_instance_at(node_id, name.clone(), start_time, duration);
		let source = MediaSource {
			asset: id,
			kind,
			path,
			info,
		};
		// コンポより大きい素材はフレームに収まるよう縮小する
		let [width, height] = source.size();
		if width > 0.0 && height > 0.0 {
			let fit = (comp_width / width).min(comp_height / height).min(1.0);
			instance.transform.scale = [fit, fit, 1.0];
		}
		instance.set_content(InstanceContent::Media(source));
		let instance_id = instance.id();
		// 素材がコンポの尺を越えるなら尺を伸ばす
		let end = start_time + duration;
		if end > comp.duration() {
			comp.set_duration(end);
		}

		self.rebuild_timeline_for_composition(composition_id);
		self.project_pane.expanded_ids.insert(composition_id);
		self.select_object(ObjectId::Instance {
			composition: composition_id,
			instance: instance_id,
		});
		self.request_preview_refresh();
		log::info!("Added `{name}` to timeline at {start_time:.2}s");
		self.show_toast(Icon::Insert, format!("Added “{name}” to the timeline"));
	}

	/// コンポジションと同じ大きさの平面レイヤーを追加する
	fn add_solid_layer(&mut self) {
		let Some(composition_id) = self.target_composition() else {
			self.show_toast(Icon::Close, "Select a composition first");
			return;
		};
		let existing: Vec<String> = self
			.project
			.composition(&composition_id)
			.map(|comp| comp.all_objects().map(|obj| obj.name().to_string()).collect())
			.unwrap_or_default();
		let name = unique_numbered_name("Solid", &existing);
		let node_id = self.project.create_node();
		let Some(comp) = self.project.composition_mut(&composition_id) else {
			return;
		};
		let (width, height, duration) = (comp.width() as f32, comp.height() as f32, comp.duration());
		let color = constants::style::label_color(comp.len());
		let instance = comp.add_instance_at(node_id, name.clone(), 0.0, duration);
		instance.set_content(InstanceContent::solid(width, height, [color.r, color.g, color.b, 1.0]));
		let instance_id = instance.id();

		self.rebuild_timeline_for_composition(composition_id);
		self.project_pane.expanded_ids.insert(composition_id);
		self.select_object(ObjectId::Instance {
			composition: composition_id,
			instance: instance_id,
		});
		self.request_preview_refresh();
		self.show_toast(Icon::LayerSolid, format!("Added solid “{name}”"));
	}

	pub(super) fn view(&self) -> Element<'_, Message> {
		let panel_view = self.panel_system.view(|_, content| self.view_panel_content(content));

		let composition = self.target_composition().and_then(|id| self.project.composition(&id));

		let mut chrome = column![];
		if TITLE_BAR_HEIGHT > 0.0 {
			chrome = chrome.push(shell::title_bar::view(
				self.project.name(),
				composition.map(|comp| comp.name()),
				self.menu
					.map(|open| open.kind)
					.filter(|kind| *kind != MenuKind::Context),
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
				export: self.export.as_ref().map(ExportTask::progress),
				pulse,
				render_fps: self.render_fps,
				composition: composition.map(|comp| core::CompositionSummary {
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

		let mut layers = stack![base].width(Length::Fill).height(Length::Fill);
		if let Some(toast) = self.toast.as_ref().filter(|toast| toast.is_active(self.now)) {
			layers = layers.push(shell::toast::view(toast, self.now, STATUS_BAR_HEIGHT + 18.0));
		}
		if let Some(open) = self.menu {
			let entries = self.menu_entries(open.kind);
			let position = menu::clamp_position(open.position, menu::panel_size(&entries), self.window_size);
			layers = layers.push(menu::overlay(entries, position));
		}
		layers.into()
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
				work_area: self.work_area(),
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
		let needs_tick = self.current_model.preview.is_playing || self.is_ui_animating() || self.export.is_some();
		#[cfg(debug_assertions)]
		let needs_tick = needs_tick || self.capture.as_ref().is_some_and(|capture| capture.is_pending());
		let tick = if needs_tick {
			iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::Tick)
		} else {
			Subscription::none()
		};
		let close = iced::window::close_requests().map(Message::CloseRequested);
		#[cfg(target_os = "macos")]
		let native_menu = Subscription::run(shell::native_menu::events);
		#[cfg(not(target_os = "macos"))]
		let native_menu = Subscription::none();

		Subscription::batch([render_subscription, events, tick, close, native_menu])
	}
}

/// `#RRGGBB` / `RRGGBB` / `#RGB` → linear 0..1 RGB.
/// Apply an inspector edit to effect `id` of `layer`. Returns the toast to
/// show and the time to jump to, if any.
fn edit_effect(
	layer: &mut domain::Instance,
	id: u32,
	edit: EffectEdit,
	time: f32,
	fps: f32,
) -> (Option<(Icon, String)>, Option<f32>) {
	let Some(name) = layer.effect(id).map(domain::LayerEffect::name) else {
		return (None, None);
	};
	match edit {
		EffectEdit::Remove => {
			layer.remove_effect(id);
			return (Some((Icon::Trash, format!("Removed {name}"))), None);
		}
		EffectEdit::Move { up } => {
			layer.move_effect(id, up);
			return (None, None);
		}
		_ => {}
	}
	let Some(effect) = layer.effect_mut(id) else {
		return (None, None);
	};
	match edit {
		EffectEdit::ToggleEnabled => {
			effect.enabled = !effect.enabled;
			(None, None)
		}
		EffectEdit::Reset => {
			effect.reset();
			(Some((Icon::Reset, format!("{name} reset"))), None)
		}
		EffectEdit::SetValue { slot, value } => {
			effect.set_value_at(slot, time, value);
			(None, None)
		}
		EffectEdit::ToggleKey { param } => {
			let label = effect.kind.params().get(param).map_or("", |param| param.name);
			let timecode = core::timecode::format(time, fps);
			let toast = if effect.toggle_key_at(param, time) {
				(Icon::KeyframeFilled, format!("Keyed {label} at {timecode}"))
			} else {
				(Icon::Keyframe, format!("Removed {label} key at {timecode}"))
			};
			(Some(toast), None)
		}
		EffectEdit::JumpToKey { param, forward } => (None, effect.adjacent_key_time(param, time, forward)),
		EffectEdit::ToggleExpanded | EffectEdit::Remove | EffectEdit::Move { .. } => (None, None),
	}
}

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
	use super::{EditCommand, Message, TransportMessage, parse_hex_color, shortcut};
	use iced::keyboard::{Key, Modifiers};

	fn press(key: &str, modifiers: Modifiers) -> Option<Message> {
		shortcut(&Key::Character(key.into()), modifiers)
	}

	#[test]
	fn after_effects_shortcuts() {
		let edit = |key, modifiers| match press(key, modifiers) {
			Some(Message::Edit(command)) => Some(command),
			_ => None,
		};
		assert_eq!(edit("b", Modifiers::empty()), Some(EditCommand::WorkAreaStart));
		assert_eq!(edit("n", Modifiers::empty()), Some(EditCommand::WorkAreaEnd));
		assert_eq!(edit("[", Modifiers::empty()), Some(EditCommand::MoveInPoint));
		assert_eq!(edit("]", Modifiers::ALT), Some(EditCommand::TrimOutPoint));
		assert_eq!(
			edit("c", Modifiers::COMMAND | Modifiers::SHIFT),
			Some(EditCommand::Precompose)
		);
		assert!(matches!(
			press("k", Modifiers::empty()),
			Some(Message::Transport(TransportMessage::NextEditPoint))
		));
		assert!(matches!(
			press("y", Modifiers::empty()),
			Some(Message::Shell(crate::message::ShellMessage::SelectTool(
				crate::tool::Tool::Anchor
			)))
		));
	}

	#[test]
	fn parses_hex_colors() {
		assert_eq!(parse_hex_color("#FF0000"), Some([1.0, 0.0, 0.0]));
		assert_eq!(parse_hex_color("0f0"), Some([0.0, 1.0, 0.0]));
		assert_eq!(parse_hex_color("#12345"), None);
		assert_eq!(parse_hex_color("#GG0000"), None);
	}
}
