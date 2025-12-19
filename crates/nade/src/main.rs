//! # Nade - Entry Point
//!
//! Nadeアプリケーションのエントリーポイント。
//! パネルシステムを使用したマルチペインUIを提供します。

mod composition;
mod encoder;
mod renderer;

use iced::widget::{Image, column, container, image, row, text};
use iced::{Color, Element, Length, Subscription, Task, Theme};
use panel_system::{LayoutBuilder, PanelSystem, PanelSystemMessage};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use timeline_panel::{TimelineMessage, TimelineWidget};

use crossbeam_channel::{Receiver, Sender, unbounded};
use nade_core::{CoreEffect, FrameData, Model, Msg, PreviewModel, update};

// =============================================================================
// テーマ設定
// =============================================================================

/// アプリケーションのテーマを返す
fn theme(_state: &NadeApp) -> Theme {
	Theme::Dark
}

// =============================================================================
// パネルコンテンツ定義
// =============================================================================

/// パネルに表示するコンテンツの種類
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelContent {
	/// メインプレビュー（ピクセルビューア）
	MainPreview,
	/// タイムライン
	Timeline,
	/// プロパティ（パラメータ編集）
	Properties,
	/// コンポジション（レイヤー/エフェクトツリー）
	Composition,
	/// コンソール（ログ出力）
	Console,
}

impl PanelContent {
	/// パネルの色を返す（識別用）
	pub fn color(&self) -> Color {
		match self {
			PanelContent::MainPreview => Color::from_rgb(0.2, 0.3, 0.4),
			PanelContent::Timeline => Color::from_rgb(0.3, 0.2, 0.4),
			PanelContent::Properties => Color::from_rgb(0.2, 0.4, 0.3),
			PanelContent::Composition => Color::from_rgb(0.4, 0.3, 0.2),
			PanelContent::Console => Color::from_rgb(0.15, 0.15, 0.15),
		}
	}
}

// =============================================================================
// メッセージ定義
// =============================================================================

/// アプリケーションパネルメッセージ（ラッパー）
#[derive(Debug, Clone)]
pub enum AppPanelMessage {
	Timeline(TimelineMessage),
}

/// アプリケーションメッセージ
#[derive(Debug, Clone)]
pub enum Message {
	/// タイムスライダー変更
	TimeChanged(f32),
	/// パネルシステムメッセージ
	PanelSystem(PanelSystemMessage<PanelContent, AppPanelMessage>),
	/// Coreからのモデル更新
	CoreUpdated(Model),
}

// =============================================================================
// アプリケーション状態
// =============================================================================

/// Coreへの接続状態（Subscription用）
#[derive(Clone)]
struct CoreConnection(Arc<std::sync::Mutex<Receiver<Model>>>);

impl std::hash::Hash for CoreConnection {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(Arc::as_ptr(&self.0) as usize).hash(state);
	}
}

// run_with only requires Hash, but usually Eq is good practice or required by Hash derivation rules
impl PartialEq for CoreConnection {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

impl Eq for CoreConnection {}

/// Coreストリームの構築
fn build_core_stream(conn: &CoreConnection) -> iced::futures::stream::BoxStream<'static, Message> {
	use iced::futures::{SinkExt, StreamExt};
	use std::time::Duration;

	let rx_mutex = conn.0.clone();

	iced::stream::channel(
		100,
		move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
			let rx = rx_mutex;
			loop {
				let result = tokio::task::spawn_blocking({
					let rx = rx.clone();
					move || {
						if let Ok(guard) = rx.lock() {
							// タイムアウト付きで受信することで、iced終了時にループを抜けられる
							match guard.recv_timeout(Duration::from_millis(100)) {
								Ok(model) => Some(Some(model)),
								Err(crossbeam_channel::RecvTimeoutError::Timeout) => Some(None),
								Err(crossbeam_channel::RecvTimeoutError::Disconnected) => None,
							}
						} else {
							None
						}
					}
				})
				.await
				.ok()
				.flatten();

				match result {
					Some(Some(model)) => {
						// Model受信成功
						if output.send(Message::CoreUpdated(model)).await.is_err() {
							// iced側がチャンネルを閉じた
							break;
						}
					}
					Some(None) => {
						// タイムアウト - 継続（ただしicedがシャットダウン中なら自然に終了する）
						continue;
					}
					None => {
						// チャンネル切断またはロック失敗
						break;
					}
				}
			}
		},
	)
	.boxed()
}

/// Nadeアプリケーション
struct NadeApp {
	/// パネルシステム
	panel_system: PanelSystem<PanelContent>,
	/// Coreへの送信チャンネル
	core_tx: Sender<Msg>,
	/// Coreからの受信チャンネル (Subscriptionで使用)
	core_rx: Arc<std::sync::Mutex<Receiver<Model>>>,
	/// 現在のモデル（表示用キャッシュ）
	current_model: Model,

	/// タイムラインウィジェット (UI State)
	timeline: TimelineWidget,
	/// ステータスバー (UI State)
	status_bar: status_bar::StatusBar,
}

impl NadeApp {
	/// アプリケーションの初期化
	fn new(core_tx: Sender<Msg>, core_rx: Receiver<Model>) -> (Self, Task<Message>) {
		let panel_system = Self::create_panel_layout();

		let app = Self {
			panel_system,
			core_tx,
			core_rx: Arc::new(std::sync::Mutex::new(core_rx)),
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
	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::CoreUpdated(model) => {
				self.current_model = model;
				Task::none()
			}
			Message::TimeChanged(value) => {
				self.core_tx.send(Msg::SetTime(value)).ok();
				Task::none()
			}
			Message::PanelSystem(msg) => {
				// アプリケーションメッセージのルーティング
				if let PanelSystemMessage::AppMessage(app_msg) = &msg {
					match app_msg {
						AppPanelMessage::Timeline(timeline_msg) => {
							self.timeline.update(timeline_msg.clone());
							// Timelineの変更をCoreに通知する場合
							// self.core_tx.send(Msg::...);
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
	fn view(&self) -> Element<'_, Message> {
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
					background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
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
			PanelContent::MainPreview => self.view_preview(),
			PanelContent::Timeline => self.view_timeline(),
			PanelContent::Properties => self.view_properties(),
			PanelContent::Composition => self.view_composition(),
			PanelContent::Console => self.view_console(),
		}
	}

	/// プレビューパネルのビュー
	fn view_preview<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		let preview = &self.current_model.preview;

		let content = if let Some(frame) = &preview.frame {
			// bytes::Bytes を使用してコピーを回避
			let handle = image::Handle::from_rgba(frame.width, frame.height, frame.pixels.clone());

			let img = Image::new(handle)
				.content_fit(iced::ContentFit::Contain)
				.width(Length::Fill)
				.height(Length::Fill);

			container(img).width(Length::Fill).height(Length::Fill)
		} else {
			container(text("No Signal").color(Color::WHITE))
				.width(Length::Fill)
				.height(Length::Fill)
				.center_x(Length::Fill)
				.center_y(Length::Fill)
		};

		let fps_text = text(format!("{:.1} FPS", preview.fps))
			.size(12)
			.color(Color::from_rgb(0.4, 0.8, 1.0));

		let time_text = text(format!("Time: {:.2}s", preview.time))
			.size(12)
			.color(Color::from_rgb(0.8, 0.8, 0.8));

		column![
			row![fps_text, text(" | ").size(12), time_text].spacing(5),
			content,
		]
		.spacing(5)
		.padding(5)
		.into()
	}

	/// タイムラインパネルのビュー
	fn view_timeline<'a>(
		&'a self,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		self.timeline
			.view()
			.map(AppPanelMessage::Timeline)
			.map(PanelSystemMessage::AppMessage)
	}

	/// プロパティパネルのビュー
	fn view_properties<'a>(
		&self,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		let content = column![
			text("Properties").size(14).color(Color::WHITE),
			text("No selection")
				.size(12)
				.color(Color::from_rgb(0.6, 0.6, 0.6)),
		]
		.spacing(10)
		.padding(10);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(PanelContent::Properties.color().into()),
				..Default::default()
			})
			.into()
	}

	/// コンポジションパネルのビュー
	fn view_composition<'a>(
		&self,
	) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		let content = column![
			text("Composition").size(14).color(Color::WHITE),
			text("└─ Layer 1")
				.size(12)
				.color(Color::from_rgb(0.8, 0.8, 0.8)),
			text("   └─ Effect: Sine Wave")
				.size(11)
				.color(Color::from_rgb(0.6, 0.6, 0.6)),
		]
		.spacing(5)
		.padding(10);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(PanelContent::Composition.color().into()),
				..Default::default()
			})
			.into()
	}

	/// コンソールパネルのビュー
	fn view_console<'a>(&self) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
		let content = column![
			text("Console").size(14).color(Color::WHITE),
			text("[INFO] Application started")
				.size(11)
				.color(Color::from_rgb(0.7, 0.7, 0.7)),
		]
		.spacing(5)
		.padding(10);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(PanelContent::Console.color().into()),
				..Default::default()
			})
			.into()
	}

	/// サブスクリプション
	fn subscription(&self) -> Subscription<Message> {
		let core_rx = self.core_rx.clone();
		Subscription::run_with(CoreConnection(core_rx), build_core_stream)
	}
}

// =============================================================================
// Core Loop
// =============================================================================

fn core_loop(rx: Receiver<Msg>, tx: Sender<Model>) {
	let mut model = Model::default();
	// 初期状態送信
	tx.send(model.clone()).ok();

	loop {
		// メッセージ待機
		let msg = match rx.recv() {
			Ok(m) => m,
			Err(_) => break, // Sender drops
		};

		if let Msg::Shutdown = msg {
			break;
		}

		// ロジック更新 (Pure)
		let (next_model, effects) = update(model, msg);
		model = next_model;

		// Effect 実行 (Impure)
		for effect in effects {
			match effect {
				CoreEffect::RenderFrame {
					time,
					width,
					height,
				} => {
					// レンダリング実行
					// ここで renderer::render_frame を呼ぶ
					// フレーム番号換算
					let frame_num = (time * 60.0) as u32;
					let img_buffer = renderer::render_frame(frame_num, width, height);
					let raw = img_buffer.into_raw();
					// Arc化してFrameData作成
					let frame_data = FrameData {
						width,
						height,
						pixels: bytes::Bytes::from(raw),
					};

					// レンダリング結果をメッセージとして自分自身(Core Logic)に戻すか、
					// 直接 Model に反映して送るか。
					// update関数は FrameRendered メッセージを受け付けるので、
					// ここで再帰的に update を呼ぶか、次のループで処理するか。
					// メッセージとして投げ直すのが本来の Elm Architecture だが、
					// チャンネル経由だと非同期になる。
					// synchronous に反映したいならここで update を呼ぶ。

					// Simple approach: Apply directly to model for now
					model.preview.frame = Some(frame_data);
				}
			}
		}

		// 更新された状態をUIへ送信
		tx.send(model.clone()).ok();
	}
}

// =============================================================================
// Entry Point
// =============================================================================

fn run_ui(ui_tx: Sender<Msg>, ui_rx: Receiver<Model>) -> iced::Result {
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

/// Entry point of the application (desktop)
pub fn main() -> iced::Result {
	#[allow(unsafe_code)]
	unsafe {
		//FIXME: RUST_LOGをアプリケーションから分離する
		std::env::set_var("RUST_LOG", "debug");
		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}

	// TODO: debug_assertions でログレベル分岐（優先度：中）

	// TODO: bounded channel 検討（優先度：低・将来）
	// 現状は問題ないが、Coreのレンダリングが重くなってUIが追いつかない場合、
	// unboundedだとメモリを消費し続ける。将来的にはboundedにしてbackpressureをかけるか、
	// 最新のModelだけ保持する仕組みを検討する。
	env_logger::init();

	// Create channels for communication between UI and core logic
	let (ui_tx, core_rx) = unbounded::<Msg>();
	let (core_tx, ui_rx) = unbounded::<Model>();

	// Shutdown用にSenderを保持
	// NOTE: NadeApp内のSenderはicedがdropするタイミングが不定のため、
	// チャネルcloseに依存せず明示的にMsg::Shutdownを送る方式を採用
	let shutdown_tx = ui_tx.clone();

	// Start core logic in a separate thread
	let core_handle = thread::spawn(move || {
		core_loop(core_rx, core_tx);
	});

	// Start UI in the main thread
	let ui_result = run_ui(ui_tx, ui_rx);

	// TODO: Graceful Shutdown改善（優先度：低）
	// 現在Subscription内でrecv_timeout(100ms)を使用してシャットダウンを検知している。
	// これは実質ポーリングであり、以下の改善案がある:
	// - shutdown専用channelを追加し、futures::select!で両方を監視する
	// - tokio::sync::watch等のbroadcast channelを使用する
	// ただし現状で実用上問題ないため、複雑化を避けて保留。
	log::debug!("UI finished, sending Shutdown...");
	shutdown_tx.send(Msg::Shutdown).ok();

	// Wait for the core thread to finish
	log::debug!("Waiting for core thread to finish...");
	if let Err(e) = core_handle.join() {
		log::error!("Core thread joined with error: {:?}", e);
	} else {
		log::info!("Core thread shutdown successfully.");
	}

	log::debug!("Shutdown completed.");
	ui_result
}
