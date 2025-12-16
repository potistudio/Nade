//! SDK_Noise.aexを実行するエフェクト

use core::{Effect, RenderContext, RgbColor};

pub struct AexNoiseEffect {}

impl Effect for AexNoiseEffect {
	fn init(&mut self) -> Result<(), String> {
		Ok(())
	}

	fn dispose(&mut self) {
		// デフォルトでは何もしない
	}

	fn apply(&self, input: RgbColor, x: u32, y: u32, ctx: &RenderContext) -> RgbColor {
		input
	}

	fn name(&self) -> &str {
		"AEX Noise Effect"
	}
}
