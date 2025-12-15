//! # Nade - Entry Point
//!
//! Nadeアプリケーションのエントリーポイント。
//!
//! ## 概要
//!
//! Icedフレームワークを使用したリアルタイムピクセルバッファビューアのデモアプリケーションです。
//! 時間に依存しないアニメーションシステムと、サイン波パターンによるピクセル生成を示しています。
//!
//! ## アーキテクチャ
//!
//! このアプリケーションはElmアーキテクチャ（TEA: The Elm Architecture）に従っています：
//!
//! - **State**: [`PixelViewer`] - アプリケーションの状態を保持
//! - **Message**: [`Message`] - ユーザー入力やシステムイベントを表現
//! - **Update**: [`PixelViewer::update`] - メッセージに基づいて状態を更新
//! - **View**: [`PixelViewer::view`] - 状態からUIを生成
//! - **Subscription**: [`PixelViewer::subscription`] - 外部イベント（フレームティック）を購読
//!
//! ## フレームレート非依存アニメーション
//!
//! アニメーションはdelta time（前フレームからの経過時間）を使用して更新されるため、
//! どのフレームレートでも一定の速度でアニメーションが進行します。

mod composition;
mod encoder;
mod renderer;

use iced::widget::{Image, column, container, image, row, slider, text};
use iced::{Color, Element, Length, Subscription, Theme};
use std::time::Instant;

// =============================================================================
// テーマ設定
// =============================================================================

/// アプリケーションのテーマを返す
///
/// 現在はライトテーマを使用していますが、ダークテーマに切り替えることも可能です。
///
/// # 引数
///
/// * `_state` - アプリケーションの状態（現在は使用していない）
///
/// # 戻り値
///
/// アプリケーションに適用する [`Theme`]
fn theme(_state: &PixelViewer) -> Theme {
	// ダークテーマを使用する場合:
	// Theme::Dark
	Theme::Light
}

// =============================================================================
// アプリケーション状態
// =============================================================================

/// アプリケーションのメイン状態構造体
///
/// ピクセルバッファ、アニメーション時間、FPS計算用のデータを保持します。
///
/// # フィールドの説明
///
/// - `width`, `height`: ピクセルバッファのサイズ（固定値320x240）
/// - `pixels`: RGBAフォーマットのピクセルデータ（1ピクセル = 4バイト）
/// - `time`: アニメーション用の時間値（秒単位、無限に増加）
/// - `last_frame`: FPS計算用の前フレームのタイムスタンプ
/// - `fps`: 表示用のFPS値（指数移動平均でスムージング）
/// - `status_bar`: 画面下部のステータスバーコンポーネント
#[derive(Debug)]
struct PixelViewer {
	/// ピクセルバッファの幅（ピクセル単位）
	width: u32,
	/// ピクセルバッファの高さ（ピクセル単位）
	height: u32,
	/// RGBAフォーマットの生ピクセルデータ
	///
	/// サイズ: `width * height * 4` バイト
	/// フォーマット: [R, G, B, A, R, G, B, A, ...]
	pixels: Vec<u8>,
	/// アニメーション時間（秒単位）
	///
	/// delta timeで更新されるため、フレームレートに依存しない
	time: f32,
	/// 前フレームのタイムスタンプ
	///
	/// delta time計算とFPS計算に使用
	last_frame: Option<Instant>,
	/// 現在のFPS値（スムージング済み）
	///
	/// 指数移動平均（EMA）でスムージング: `fps = fps * 0.9 + current_fps * 0.1`
	fps: f32,
	/// ステータスバーコンポーネント
	status_bar: status_bar::StatusBar,
}

impl PixelViewer {
	/// 新しい [`PixelViewer`] インスタンスをデフォルト値で作成
	///
	/// # 初期化内容
	///
	/// - ピクセルバッファ: 320x240ピクセル（すべて黒で初期化）
	/// - アニメーション時間: 0.0秒
	/// - FPS: 0.0（最初のフレームで計算開始）
	///
	/// # 戻り値
	///
	/// 初期化された [`PixelViewer`] インスタンス
	fn default() -> Self {
		// ピクセルバッファのサイズ設定
		let width = 320;
		let height = 240;

		// RGBAフォーマット（4バイト/ピクセル）でバッファを確保
		// 初期値は0（黒色、透明度255は後で設定）
		let pixels = vec![0u8; (width * height * 4) as usize];

		Self {
			width,
			height,
			pixels,
			time: 0.0,
			last_frame: None,
			fps: 0.0,
			status_bar: status_bar::StatusBar::new(),
		}
	}
}

// =============================================================================
// メッセージ定義
// =============================================================================

/// アプリケーションで発生するイベント（メッセージ）
///
/// Elmアーキテクチャにおけるメッセージタイプです。
/// すべてのユーザー入力やシステムイベントはこの enum として表現されます。
#[derive(Debug, Clone)]
enum Message {
	/// フレームティックイベント
	///
	/// `window::frames()` から発火され、アニメーション更新のトリガーとなります。
	/// 引数の [`Instant`] はフレームのタイムスタンプで、delta time計算に使用されます。
	Tick(Instant),

	/// タイムスライダーの値変更イベント
	///
	/// ユーザーがスライダーを操作した際に発火されます。
	/// 引数の `f32` は新しい時間値（秒単位）です。
	TimeChanged(f32),
}

// =============================================================================
// アプリケーションロジック
// =============================================================================

impl PixelViewer {
	/// メッセージを処理してアプリケーション状態を更新
	///
	/// Elmアーキテクチャのupdate関数に相当します。
	/// 受信したメッセージに基づいて状態を変更し、必要に応じてピクセルバッファを再生成します。
	///
	/// # 引数
	///
	/// * `message` - 処理するメッセージ
	///
	/// # 処理フロー
	///
	/// 1. **Tick**: delta timeを計算 → FPSを更新 → 時間を進める → ピクセル再生成
	/// 2. **TimeChanged**: 時間を直接設定 → ピクセル再生成
	fn update(&mut self, message: Message) {
		match message {
			Message::Tick(now) => {
				// ========================================
				// Step 1: Delta timeを計算
				// ========================================
				// 前フレームからの経過時間を秒単位で計算
				// 初回フレームの場合は60FPS相当（16.67ms）を仮定
				let dt = if let Some(last) = self.last_frame {
					now.duration_since(last).as_secs_f32()
				} else {
					0.016 // ≈ 1/60秒（60FPS相当）
				};

				// 次回のdelta time計算用にタイムスタンプを保存
				self.last_frame = Some(now);

				// ========================================
				// Step 2: FPSを更新（指数移動平均）
				// ========================================
				// 急激な変動を避けるため、EMA（Exponential Moving Average）を使用
				// 新しい値の重みは10%、既存の値の重みは90%
				if dt > 0.0 {
					let current_fps = 1.0 / dt;
					self.fps = self.fps * 0.9 + current_fps * 0.1;
				}

				// ========================================
				// Step 3: アニメーション時間を更新
				// ========================================
				// delta timeを加算することで、フレームレートに依存しない
				// 一定速度のアニメーションを実現
				self.time += dt;

				// ========================================
				// Step 4: ピクセルバッファを再生成
				// ========================================
				self.update_pixels();
			}
			Message::TimeChanged(value) => {
				// スライダーからの直接入力で時間を設定
				self.time = value;
				self.update_pixels();
			}
		}
	}

	/// ピクセルバッファをアニメーションパターンで更新
	///
	/// サイン波を組み合わせたカラフルなパターンを生成します。
	/// 時間値（`self.time`）によってパターンがアニメーションします。
	///
	/// # パターンの説明
	///
	/// - **赤チャンネル (R)**: 水平方向のサイン波（左右に波打つ）
	/// - **緑チャンネル (G)**: 垂直方向のコサイン波（上下に波打つ）
	/// - **青チャンネル (B)**: 放射状のサイン波（中心から広がる波紋）
	///
	/// # 数学的表現
	///
	/// ```text
	/// R = (sin(x * 10 + t) * 0.5 + 0.5) * 255
	/// G = (cos(y * 10 - t * 1.3) * 0.5 + 0.5) * 255
	/// B = (sin(d * 20 - t * 0.7) * 0.5 + 0.5) * 255
	///   where d = sqrt((x - 0.5)² + (y - 0.5)²)  // 中心からの距離
	/// ```
	fn update_pixels(&mut self) {
		// ========================================
		// Step 1: 時間からフレーム番号を計算
		// ========================================
		// 60 FPS 相当のフレーム番号に変換
		let frame_num = (self.time * 60.0) as u32;

		// ========================================
		// Step 2: renderer モジュールでフレームを生成
		// ========================================
		// RGB フォーマットの ImageBuffer を取得
		let rgb_image = renderer::render_frame(frame_num, self.width, self.height);

		// ========================================
		// Step 3: RGB → RGBA 変換
		// ========================================
		// Iced の Image ウィジェットは RGBA フォーマットを期待するため変換が必要
		let rgb_data = rgb_image.into_raw();

		// RGB (3バイト/ピクセル) → RGBA (4バイト/ピクセル)
		let pixel_count = (self.width * self.height) as usize;
		for i in 0..pixel_count {
			let src_idx = i * 3; // RGB のインデックス
			let dst_idx = i * 4; // RGBA のインデックス

			self.pixels[dst_idx] = rgb_data[src_idx]; // R
			self.pixels[dst_idx + 1] = rgb_data[src_idx + 1]; // G
			self.pixels[dst_idx + 2] = rgb_data[src_idx + 2]; // B
			self.pixels[dst_idx + 3] = 255; // A（不透明）
		}
	}

	/// アプリケーションUIをレンダリング
	///
	/// 現在の状態からIcedのUI要素を生成します。
	/// Elmアーキテクチャのview関数に相当します。
	///
	/// # UIレイアウト構造
	///
	/// ```text
	/// ┌─────────────────────────────────────┐
	/// │ Raw Pixel Viewer (Iced)             │ ← ヘッダー
	/// │ Time: [========] 12.3               │ ← タイムスライダー
	/// │ 60.0 FPS                            │ ← FPS表示
	/// │ ┌─────────────────────────────────┐ │
	/// │ │                                 │ │
	/// │ │        (ピクセル画像)           │ │ ← メイン画像
	/// │ │                                 │ │
	/// │ └─────────────────────────────────┘ │
	/// │ ステータスバー                      │ ← ステータスバー
	/// └─────────────────────────────────────┘
	/// ```
	///
	/// # 戻り値
	///
	/// レンダリング可能なUI要素 [`Element`]
	fn view(&self) -> Element<'_, Message> {
		// ========================================
		// Step 1: ピクセルバッファから画像ハンドルを作成
		// ========================================
		// RGBAデータから直接Iced画像ハンドルを生成
		// 注意: pixels.clone() により毎フレームコピーが発生（最適化の余地あり）
		let handle = image::Handle::from_rgba(self.width, self.height, self.pixels.clone());

		// ========================================
		// Step 2: ヘッダーテキストを作成
		// ========================================
		let header = text("Raw Pixel Viewer (Iced)").size(20);

		// ========================================
		// Step 3: タイムスライダーコントロールを作成
		// ========================================
		// 構成: [ラベル] [スライダー] [現在値]
		let time_control = row![
			text("Time:").size(14),
			slider(0.0..=100.0, self.time, Message::TimeChanged).width(Length::Fixed(200.0)),
			text(format!("{:.1}", self.time)).size(14),
		]
		.spacing(10)
		.align_y(iced::Alignment::Center);

		// ========================================
		// Step 4: FPS表示テキストを作成
		// ========================================
		// シアン色でFPS値を表示
		let fps_text = text(format!("{:.1} FPS", self.fps))
			.size(14)
			.color(Color::from_rgb(0.4, 0.8, 1.0));

		// ========================================
		// Step 5: メイン画像ウィジェットを作成
		// ========================================
		// アスペクト比を保持しながら利用可能なスペースに収まるように表示
		let img = Image::new(handle)
			.content_fit(iced::ContentFit::Contain)
			.width(Length::Fill)
			.height(Length::Fill);

		// 画像をコンテナでラップ
		let image_container = container(img).width(Length::Fill).height(Length::Fill);

		// ========================================
		// Step 6: メインコンテンツを縦に配置
		// ========================================
		let content = column![header, time_control, fps_text, image_container]
			.spacing(10)
			.padding(10)
			.width(Length::Fill)
			.height(Length::Fill);

		// ========================================
		// Step 7: ステータスバーを追加
		// ========================================
		// メインコンテンツの下にダークカラーのステータスバーを配置
		let main_layout = column![
			container(content).width(Length::Fill).height(Length::Fill),
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

	/// 外部イベントを購読
	///
	/// ディスプレイのリフレッシュレートに同期したフレームイベントを購読します。
	/// これにより、VSyncに合わせた滑らかなアニメーションが実現されます。
	///
	/// # 動作
	///
	/// `window::frames()` はディスプレイのリフレッシュレート（通常60Hz）に
	/// 同期してイベントを発火します。各イベントには [`Instant`] タイムスタンプが
	/// 含まれ、これを使用してdelta timeを計算します。
	///
	/// # 戻り値
	///
	/// フレームイベントの [`Subscription`]
	fn subscription(&self) -> Subscription<Message> {
		// ディスプレイのリフレッシュレートに同期
		// 各フレームで Message::Tick(Instant) が発火される
		iced::window::frames().map(Message::Tick)
	}
}

// =============================================================================
// エントリーポイント
// =============================================================================

/// アプリケーションのエントリーポイント
///
/// 環境変数の設定、ロガーの初期化、Icedアプリケーションの起動を行います。
///
/// # 環境変数
///
/// - `RUST_LOG=debug`: デバッグログを有効化
/// - `ICED_PRESENT_MODE=immediate`: VSyncを無効化（最大フレームレートで描画）
///
/// # ウィンドウ設定
///
/// - タイトル: "Nade Prototype"
/// - 初期サイズ: 1280x720ピクセル
/// - テーマ: ライト
///
/// # 戻り値
///
/// アプリケーション終了時の [`iced::Result`]
pub fn main() -> iced::Result {
	#[allow(unsafe_code)]
	unsafe {
		std::env::set_var("RUST_LOG", "debug");
		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}

	env_logger::init();

	iced::application(PixelViewer::default, PixelViewer::update, PixelViewer::view)
		.subscription(PixelViewer::subscription)
		.theme(theme)
		.title("Nade Prototype")
		.window_size((1280.0, 720.0))
		.run()
}
