//! Video Editor Core Prototype

use std::f32::consts::PI;
use uuid::Uuid;

// -----------------------------------------------------------------
// 1. Domain Layer: ID Definition (堅牢なID)
// -----------------------------------------------------------------
mod id {
	use super::*;

	// TODO: Replace with macro
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct ClipId(Uuid);

	impl ClipId {
		pub fn new() -> Self {
			Self(Uuid::new_v4())
		}
	}
}

// -----------------------------------------------------------------
// 2. Domain Layer: Property System (値モディファイア)
// -----------------------------------------------------------------
mod property {
	use super::*;

	pub type Ticks = u64;

	// 値を加工する計算ロジック
	#[derive(Debug, Clone)]
	pub enum ValueModifier {
		// サイン波で値を揺らす
		SineWave { amplitude: f32, frequency: f32 },
		// 値に加算する（オフセット）
		Add { value: f32 },
	}

	impl ValueModifier {
		// 与えられた基本値(base)を、時間(time)に応じて加工する
		fn apply(&self, base: f32, time: Ticks) -> f32 {
			match self {
				ValueModifier::SineWave { amplitude, frequency } => {
					let t_sec = time as f32 / 60.0; // 60fpsと仮定
					base + (t_sec * frequency * 2.0 * PI).sin() * amplitude
				}
				ValueModifier::Add { value } => base + value,
			}
		}
	}

	// アニメーション可能なプロパティ
	#[derive(Debug, Clone)]
	pub struct Animated<T> {
		pub base: T, // 本来はここにキーフレームが入るが、今回は固定値
		pub modifiers: Vec<ValueModifier>,
	}

	impl Animated<f32> {
		pub fn new(base: f32) -> Self {
			Self {
				base,
				modifiers: vec![],
			}
		}

		pub fn add_modifier(&mut self, modifier: ValueModifier) {
			self.modifiers.push(modifier);
		}

		// ■■■ 評価フェーズ (Evaluation) ■■■
		// ここがエンジンの肝。時間を渡して「今の値」を確定させる。
		pub fn evaluate(&self, time: Ticks) -> f32 {
			let mut value = self.base;
			for modifier in &self.modifiers {
				value = modifier.apply(value, time);
			}
			value
		}
	}
}

// -----------------------------------------------------------------
// 3. Domain Layer: Model (映像エフェクト & クリップ)
// -----------------------------------------------------------------
mod model {
	use super::id::ClipId;
	use super::property::{Animated, Ticks};

	// 映像エフェクトの種類
	#[derive(Debug, Clone)]
	pub enum VideoEffectKind {
		Blur { radius: Animated<f32> }, // パラメータ自体がAnimated
		ColorGrade { brightness: Animated<f32> },
	}

	#[derive(Debug, Clone)]
	pub struct Clip {
		id: ClipId,
		pub name: String,
		pub opacity: Animated<f32>,        // クリップ自体の透明度
		pub effects: Vec<VideoEffectKind>, // エフェクトチェーン
	}

	impl Clip {
		pub fn new(name: &str) -> Self {
			Self {
				id: ClipId::new(),
				name: name.to_string(),
				opacity: Animated::new(1.0), // デフォルト不透明
				effects: vec![],
			}
		}

		// 現在の状態を「レンダリング命令」としてダンプする（Engineへの指示）
		pub fn evaluate_state(&self, time: Ticks) -> String {
			// 1. 透明度の計算
			let current_opacity = self.opacity.evaluate(time);

			// 2. エフェクト情報の構築
			let mut effects_info = String::new();
			for effect in &self.effects {
				match effect {
					VideoEffectKind::Blur { radius } => {
						let r = radius.evaluate(time);
						effects_info.push_str(&format!("[Blur: r={:.2}] ", r));
					}
					VideoEffectKind::ColorGrade { brightness } => {
						let b = brightness.evaluate(time);
						effects_info.push_str(&format!("[Color: b={:.2}] ", b));
					}
				}
			}

			format!(
				"Clip {:?} '{}' | Time: {:>3} | Opacity: {:.2} | Effects: {}",
				self.id, self.name, time, current_opacity, effects_info
			)
		}
	}
}

// -----------------------------------------------------------------
// 4. App Layer: CLI Implementation (シミュレーション)
// -----------------------------------------------------------------
use model::{Clip, VideoEffectKind};
use property::{Animated, ValueModifier};

fn main() {
	println!("--- Nade ---");

	// 1. クリップ作成 (ドメインロジック)
	let mut clip = Clip::new("Test Footage");
	println!("Created Clip: {}", clip.name);

	// 2. クリップの透明度を「サイン波」で点滅させる (値モディファイア)
	println!("Adding SineWave modifier to Opacity...");
	clip.opacity.add_modifier(ValueModifier::SineWave {
		amplitude: 0.5, // 0.5〜1.5の間で振れる(後でClamp必要だが今回は簡易化)
		frequency: 0.5, // 2秒で1周
	});

	// 3. ブラーエフェクトを追加 (映像エフェクト)
	// さらに、ブラーの強さ(Radius)を「時間とともに増やす」
	println!("Adding Blur effect with animated radius...");
	let mut blur_radius = Animated::new(0.0);
	blur_radius.add_modifier(ValueModifier::Add { value: 0.1 }); // 毎フレーム固定値を足すわけではない。評価関数の見直しが必要だが、デモ用にベースAdd

	blur_radius.add_modifier(ValueModifier::SineWave {
		amplitude: 10.0,
		frequency: 0.2,
	});

	clip.effects.push(VideoEffectKind::Blur { radius: blur_radius });

	// 3.5 カラーグレーディングも追加
	let mut brightness = Animated::new(1.0);
	brightness.add_modifier(ValueModifier::SineWave {
		amplitude: 0.2,
		frequency: 0.1,
	});
	clip.effects.push(VideoEffectKind::ColorGrade { brightness });

	// 4. タイムライン再生シミュレーション (Engine Loop)
	println!("\n--- Rendering Simulation (0 to 120 frames) ---");

	for time in (0..=120).step_by(10) {
		// ここで `evaluate` が走り、その瞬間の値が確定する
		let render_instruction = clip.evaluate_state(time);

		// 実際にはここで wgpu::Queue::submit などが行われる
		println!("{}", render_instruction);
	}
}
