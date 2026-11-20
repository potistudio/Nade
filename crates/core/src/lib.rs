//! # Core - Shared Types and Traits
//!
//! Nadeアプリケーションで共有される基本型とトレイトを提供します。

pub use core::{
	AssetId, EvalContext, EvalError, Image, NodeId, NodeState, OperatorState, Output, OutputType, Value, ValueKind,
	ValueParam,
};
pub mod affine;
pub mod bitdepth;
pub mod media;
pub mod sample_rate;

pub use affine::{Affine, BlendMode};
pub use bitdepth::BitDepth;
pub use media::MediaInfo;
pub use sample_rate::SampleRate;

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
	pub const WHITE: Self = Self { r: 255, g: 255, b: 255 };
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

pub mod animation;
pub mod core;
pub mod object;
pub mod ops;
pub mod timecode;
pub mod timeline;

pub use animation::{AnimationCurve, HandleSide, Interpolation, Keyframe};
pub use core::*;
pub use object::{RectangleObject, SceneObject, SceneObjectData, SceneObjectId, TextObject};
pub use ops::{EvalAccess, Operator, ParamDescriptor, ParamKind, ParamValue, ResolvedHashes, ResolvedOp, ValueParamUi};
pub use timeline::{TimelineClip, TimelineModel, TimelineTrack, TrackVisibilityDisplay};

// =============================================================================
// アプリケーション状態 (Core/Model)
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct Model {
	pub preview: PreviewModel,
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
	/// 現在選択されているオブジェクトのトランスフォーム (テスト用)
	pub selection: Option<Transform>,
	/// プレビュービューポートのズーム（1.0 = フィット）
	pub view_zoom: f32,
	/// プレビュービューポートのパンオフセット（ピクセル）
	pub view_offset: [f32; 2],
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
			selection: Some(Transform::default()),
			view_zoom: 1.0,
			view_offset: [0.0, 0.0],
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
	/// 再生中に実時間で時間を進める（秒）
	Advance(f32),
	/// 時間変更要求
	SetTime(f32),
	/// 再生トグル
	TogglePlay,
	/// シャットダウン
	Shutdown,
	/// フレームレンダリング完了
	FrameRendered(FrameData),
	/// トランスフォーム更新
	UpdateTransform(Transform),
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
				model.preview.time += 1.0 / 60.0;
				effects.push(CoreEffect::RenderFrame {
					time: model.preview.time,
					width: model.preview.width,
					height: model.preview.height,
				});
			}
		}
		Msg::Advance(delta) => {
			if model.preview.is_playing {
				model.preview.time += delta.max(0.0);
				effects.push(CoreEffect::RenderFrame {
					time: model.preview.time,
					width: model.preview.width,
					height: model.preview.height,
				});
			}
		}
		Msg::SetTime(t) => {
			model.preview.time = t;
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

// =============================================================================
// トランスフォーム
// =============================================================================

/// 3次元トランスフォーム
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transform {
	/// Point of the layer (relative to its center, layer px) that sits at
	/// `position` and that rotation / scale pivot around.
	#[serde(default)]
	pub anchor: [f32; 3],
	pub position: [f32; 3],
	pub rotation: [f32; 3],
	pub scale: [f32; 3],
	pub opacity: f32,
}

impl Default for Transform {
	fn default() -> Self {
		Self {
			anchor: [0.0, 0.0, 0.0],
			position: [0.0, 0.0, 0.0],
			rotation: [0.0, 0.0, 0.0],
			scale: [1.0, 1.0, 1.0],
			opacity: 1.0,
		}
	}
}

impl Transform {
	/// Layer space (origin at the layer center) → parent space.
	pub fn local_matrix(&self) -> Affine {
		Affine::translate(self.position[0], self.position[1])
			.then_apply(Affine::rotate(self.rotation[2]))
			.then_apply(Affine::scale(self.scale[0], self.scale[1]))
			.then_apply(Affine::translate(-self.anchor[0], -self.anchor[1]))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn anchor_is_the_pivot() {
		let transform = Transform {
			anchor: [10.0, 0.0, 0.0],
			position: [100.0, 50.0, 0.0],
			rotation: [0.0, 0.0, 90.0],
			scale: [2.0, 2.0, 1.0],
			opacity: 1.0,
		};
		let matrix = transform.local_matrix();
		let pivot = matrix.apply([10.0, 0.0]);
		assert!((pivot[0] - 100.0).abs() < 1e-4 && (pivot[1] - 50.0).abs() < 1e-4);
		// 中心はアンカーの逆側へ 2 倍・90° 回転して (100, 30)
		let center = matrix.apply([0.0, 0.0]);
		assert!((center[0] - 100.0).abs() < 1e-4 && (center[1] - 30.0).abs() < 1e-4);
	}

	#[test]
	fn set_time_requests_render() {
		let (model, effects) = update(Model::default(), Msg::SetTime(1.5));
		assert!((model.preview.time - 1.5).abs() < f32::EPSILON);
		assert_eq!(effects.len(), 1);
		assert!(matches!(
			effects[0],
			CoreEffect::RenderFrame {
				time,
				width: 640,
				height: 360
			} if (time - 1.5).abs() < f32::EPSILON
		));
	}

	#[test]
	fn tick_while_playing_advances_time_and_renders() {
		let mut model = Model::default();
		model.preview.is_playing = true;
		let (model, effects) = update(model, Msg::Tick);
		assert!((model.preview.time - 1.0 / 60.0).abs() < f32::EPSILON);
		assert_eq!(effects.len(), 1);
	}

	#[test]
	fn advance_uses_wall_clock_delta() {
		let mut model = Model::default();
		model.preview.is_playing = true;
		let (model, effects) = update(model, Msg::Advance(0.05));
		assert!((model.preview.time - 0.05).abs() < f32::EPSILON);
		assert_eq!(effects.len(), 1);

		let (paused, effects) = update(Model::default(), Msg::Advance(0.05));
		assert_eq!(paused.preview.time, 0.0);
		assert!(effects.is_empty());
	}

	#[test]
	fn tick_while_paused_is_noop() {
		let (model, effects) = update(Model::default(), Msg::Tick);
		assert_eq!(model.preview.time, 0.0);
		assert!(effects.is_empty());
	}
}
