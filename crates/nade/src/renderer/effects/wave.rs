//! # Wave Effect
//!
//! アニメーション波形パターンを生成するエフェクトです。

use core::{Effect, RenderContext, RgbColor};

/// 波形エフェクト
///
/// X座標とY座標に基づくsin/cos波パターンを生成します。
/// 時間経過によりアニメーションします。
///
/// # Example
///
/// ```ignore
/// let effect = WaveEffect::default();
/// let ctx = RenderContext { width: 320, height: 240, time: 0.0, frame: 0 };
/// let color = effect.apply(RgbColor::BLACK, 100, 50, &ctx);
/// ```
#[derive(Debug, Clone)]
pub struct WaveEffect {
	/// 波の周波数（デフォルト: 1.0）
	pub frequency: f32,
	/// 青チャンネルの固定値（デフォルト: 128）
	pub blue_value: u8,
}

impl Default for WaveEffect {
	fn default() -> Self {
		Self {
			frequency: 1.0,
			blue_value: 128,
		}
	}
}

impl Effect for WaveEffect {
	fn apply(&self, _input: RgbColor, x: u32, y: u32, ctx: &RenderContext) -> RgbColor {
		// フレーム番号を時間パラメータに変換
		let t = ctx.frame as f32 / 100.0 * self.frequency;

		// x座標に基づく赤チャンネル（sin波でアニメーション）
		let r = ((x as f32 / ctx.width as f32 * 255.0) * t.sin().abs()) as u8;

		// y座標に基づく緑チャンネル（cos波でアニメーション）
		let g = ((y as f32 / ctx.height as f32 * 255.0) * t.cos().abs()) as u8;

		RgbColor {
			r,
			g,
			b: self.blue_value,
		}
	}

	fn name(&self) -> &str {
		"Wave Effect"
	}
}
