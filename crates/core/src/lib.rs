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

	/// エフェクト名を取得
	///
	/// デバッグやUI表示に使用されます。
	fn name(&self) -> &str;
}
