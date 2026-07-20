//! # Core - Shared Types and Traits
//!
//! Nadeアプリケーションで共有される基本型とトレイトを提供します。

pub use core::{
	AssetId, EvalContext, EvalError, Image, NodeId, NodeState, OperatorState, Output, OutputType, Value, ValueKind,
	ValueParam,
};
pub mod bitdepth;
pub mod sample_rate;

pub use bitdepth::BitDepth;
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

pub mod core;
mod hash;
pub mod object;
pub mod ops;
pub mod timeline;

pub use core::*;
pub use object::{RectangleObject, SceneObject, SceneObjectData, SceneObjectId};
pub use ops::{EvalAccess, Operator, ParamDescriptor, ParamKind, ParamValue, ResolvedHashes, ResolvedOp, ValueParamUi};
pub use timeline::{TimelineClip, TimelineModel, TimelineTrack};

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
// トランスフォーム
// =============================================================================

/// 3次元トランスフォーム
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
	pub position: [f32; 3],
	pub rotation: [f32; 3],
	pub scale: [f32; 3],
	pub opacity: f32,
}

impl Default for Transform {
	fn default() -> Self {
		Self {
			position: [0.0, 0.0, 0.0],
			rotation: [0.0, 0.0, 0.0],
			scale: [1.0, 1.0, 1.0],
			opacity: 1.0,
		}
	}
}
