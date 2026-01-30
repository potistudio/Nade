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

pub mod composition;
pub mod object;

// コンポジション型の再エクスポート
pub use composition::Composition;
// シーンオブジェクト型の再エクスポート
pub use object::{RectangleObject, SceneObject, SceneObjectData, SceneObjectId};

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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_rectangle_object_visibility() {
		let rect = RectangleObject::new("Test")
			.with_start_time(1.0)
			.with_duration(3.0);

		assert!(!rect.is_visible_at(0.5));
		assert!(rect.is_visible_at(1.0));
		assert!(rect.is_visible_at(2.5));
		assert!(!rect.is_visible_at(4.0));
	}

	#[test]
	fn test_composition_add_and_get() {
		let mut comp = Composition::new();
		let id = comp.add_rectangle(RectangleObject::new("Rect1"));

		assert_eq!(comp.len(), 1);
		assert!(comp.get(id).is_some());
		assert_eq!(comp.get(id).unwrap().name(), "Rect1");
	}

	#[test]
	fn test_composition_visible_objects() {
		let mut comp = Composition::new();
		comp.add_rectangle(
			RectangleObject::new("Early")
				.with_start_time(0.0)
				.with_duration(2.0),
		);
		comp.add_rectangle(
			RectangleObject::new("Late")
				.with_start_time(3.0)
				.with_duration(2.0),
		);

		let visible_at_1 = comp.visible_objects_at(1.0);
		assert_eq!(visible_at_1.len(), 1);
		assert_eq!(visible_at_1[0].name(), "Early");

		let visible_at_4 = comp.visible_objects_at(4.0);
		assert_eq!(visible_at_4.len(), 1);
		assert_eq!(visible_at_4[0].name(), "Late");
	}
}
