pub mod wave;
pub use wave::WaveEffect;

use core::{RenderContext, RgbColor};

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
