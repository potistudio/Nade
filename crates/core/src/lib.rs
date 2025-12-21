//! # Core - Shared Types and Traits
//!
//! Nadeアプリケーションで共有される基本型とトレイトを提供します。

// =============================================================================
// 基本型
// =============================================================================

/// フレームバッファ
///
/// レンダリング結果を格納するためのバッファ構造体です。
pub struct FrameBuffer {
	/// バッファの幅（ピクセル）
	pub width: u32,
	/// バッファの高さ（ピクセル）
	pub height: u32,
	/// RGBピクセルデータ
	pub data: Vec<u8>,
}

/// RGB色型
///
/// 0-255の範囲で各チャンネルを表現します。
#[derive(Debug, Clone, Copy, Default)]
pub struct RgbColor {
	/// 赤チャンネル (0-255)
	pub r: u8,
	/// 緑チャンネル (0-255)
	pub g: u8,
	/// 青チャンネル (0-255)
	pub b: u8,
}

impl RgbColor {
	/// 新しいRgbColorを作成
	pub const fn new(r: u8, g: u8, b: u8) -> Self {
		Self { r, g, b }
	}

	/// 黒色
	pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

	/// 白色
	pub const WHITE: Self = Self {
		r: 255,
		g: 255,
		b: 255,
	};
}

// =============================================================================
// レンダリングコンテキスト
// =============================================================================

/// レンダリングコンテキスト
///
/// エフェクト処理に必要な情報を提供します。
#[derive(Debug, Clone, Copy)]
pub struct RenderContext {
	/// 出力幅（ピクセル）
	pub width: u32,
	/// 出力高さ（ピクセル）
	pub height: u32,
	/// アニメーション時間（秒）
	pub time: f32,
	/// フレーム番号
	pub frame: u32,
}

// =============================================================================
// エフェクト
// =============================================================================

pub mod effects;

// =============================================================================
// エフェクトトレイト
// =============================================================================

/// エフェクトコンポーネントの共通インターフェース
///
/// すべてのエフェクトはこのトレイトを実装します。
/// `Send + Sync` によりマルチスレッド処理と外部プラグイン連携が可能です。
///
/// # Example
///
/// ```ignore
/// struct MyEffect;
///
/// impl Effect for MyEffect {
///     fn apply(&self, input: RgbColor, x: u32, y: u32, ctx: &RenderContext) -> RgbColor {
///         // エフェクト処理
///         input
///     }
///
///     fn name(&self) -> &str {
///         "My Effect"
///     }
/// }
/// ```
pub trait Effect: Send + Sync {
	/// エフェクトを初期化
	///
	/// エフェクトが使用される前に呼び出されます。
	/// 外部ファイルの読み込みやリソースの確保などを行います。
	///
	/// # Returns
	///
	/// 初期化に成功した場合は `Ok(())`、失敗した場合はエラーメッセージを含む `Err`
	///
	/// # Example
	///
	/// ```ignore
	/// fn init(&mut self) -> Result<(), String> {
	///     self.texture = load_texture("path/to/texture.png")?;
	///     Ok(())
	/// }
	/// ```
	fn init(&mut self) -> Result<(), String> {
		Ok(())
	}

	/// リソースを解放
	///
	/// エフェクトが不要になった際に呼び出されます。
	/// 読み込んだファイルやリソースの解放を行います。
	fn dispose(&mut self) {
		// デフォルトでは何もしない
	}

	/// 初期化済みかどうかを確認
	///
	/// # Returns
	///
	/// 初期化済みの場合は `true`
	fn is_initialized(&self) -> bool {
		true
	}

	/// ピクセルにエフェクトを適用
	///
	/// # Arguments
	///
	/// * `input` - 入力色（前のエフェクトの出力または初期値）
	/// * `x` - ピクセルのX座標
	/// * `y` - ピクセルのY座標
	/// * `ctx` - レンダリングコンテキスト
	///
	/// # Returns
	///
	/// エフェクト適用後の色
	fn apply(&self, input: RgbColor, x: u32, y: u32, ctx: &RenderContext) -> RgbColor;

	/// デバッグやUI表示に使用されます。
	fn name(&self) -> &str;
}

// =============================================================================
// アプリケーション状態 (Core/Model)
// =============================================================================

#[derive(Debug, Clone)]
pub struct Model {
	pub preview: PreviewModel,
}

impl Default for Model {
	fn default() -> Self {
		Self {
			preview: PreviewModel::default(),
		}
	}
}

/// プレビュー状態
#[derive(Debug, Clone)]
pub struct PreviewModel {
	/// 現在の時間（秒）
	pub time: f32,
	/// フレームレート
	pub fps: f32,
	/// 幅
	pub width: u32,
	/// 高さ
	pub height: u32,
	/// 再生中かどうか
	pub is_playing: bool,
	/// 現在のフレームデータ（レンダリング結果）
	///
	/// `Arc<[u8]>` を使用してスレッド間で効率的に共有します。
	pub frame: Option<FrameData>,
}

impl Default for PreviewModel {
	fn default() -> Self {
		Self {
			time: 0.0,
			fps: 0.0,
			width: 640,
			height: 360,
			is_playing: false,
			frame: None,
		}
	}
}

/// フレームデータ
#[derive(Debug, Clone)]
pub struct FrameData {
	pub width: u32,
	pub height: u32,
	pub pixels: bytes::Bytes,
}

// =============================================================================
// メッセージ (Msg / Events)
// =============================================================================

#[derive(Debug, Clone)]
pub enum Msg {
	/// 定期更新（ティック）
	Tick,
	/// 時間変更要求
	SetTime(f32),
	/// 再生トグル
	TogglePlay,
	/// シャットダウン
	Shutdown,
	/// フレームレンダリング完了
	FrameRendered(FrameData),
}

// =============================================================================
// エフェクト (Side Effects)
// =============================================================================

#[derive(Debug, Clone)]
pub enum CoreEffect {
	RenderFrame { time: f32, width: u32, height: u32 },
}

// =============================================================================
// Update Logic
// =============================================================================

pub fn update(mut model: Model, msg: Msg) -> (Model, Vec<CoreEffect>) {
	let mut effects = Vec::new();

	match msg {
		Msg::Tick => {
			if model.preview.is_playing {
				// シンプルな 60fps シミュレーション
				model.preview.time += 1.0 / 60.0;
				// Render request
				effects.push(CoreEffect::RenderFrame {
					time: model.preview.time,
					width: model.preview.width,
					height: model.preview.height,
				});
			}
		}
		Msg::SetTime(t) => {
			model.preview.time = t;
			// Seek したらレンダリング
			effects.push(CoreEffect::RenderFrame {
				time: model.preview.time,
				width: model.preview.width,
				height: model.preview.height,
			});
		}
		Msg::TogglePlay => {
			model.preview.is_playing = !model.preview.is_playing;
		}
		Msg::Shutdown => {
			// No-op for now
		}
		Msg::FrameRendered(frame) => {
			model.preview.frame = Some(frame);
		}
	}

	(model, effects)
}
