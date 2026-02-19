pub mod rectangle;

use crate::Transform;
pub use rectangle::RectangleObject;

// =============================================================================
// シーンオブジェクトID
// =============================================================================

/// シーンオブジェクトを一意に識別するためのID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneObjectId(pub u64);

// =============================================================================
// シーンオブジェクトトレイト
// =============================================================================

/// シーンオブジェクトの共通インターフェース
///
/// すべての描画可能なオブジェクトはこのトレイトを実装します。
/// タイムライン上での時間制御とトランスフォームをサポートします。
pub trait SceneObject: Send + Sync {
	/// オブジェクトのIDを取得
	fn id(&self) -> SceneObjectId;

	/// オブジェクト名を取得
	fn name(&self) -> &str;

	/// トランスフォームを取得
	fn transform(&self) -> &Transform;

	/// トランスフォームを設定
	fn set_transform(&mut self, transform: Transform);

	/// 開始時間（秒）
	fn start_time(&self) -> f32;

	/// 終了時間（秒）
	fn end_time(&self) -> f32;

	/// 持続時間（秒）
	fn duration(&self) -> f32 {
		self.end_time() - self.start_time()
	}

	/// 指定時間で可視かどうか
	fn is_visible_at(&self, time: f32) -> bool {
		time >= self.start_time() && time < self.end_time()
	}

	/// 開始時間を設定
	fn set_start_time(&mut self, time: f32);

	/// 持続時間を設定
	fn set_duration(&mut self, duration: f32);
}

// =============================================================================
// シーンオブジェクトデータトレイト
// =============================================================================

/// SceneObjectを型消去して扱うためのトレイト
pub trait SceneObjectData: SceneObject + std::fmt::Debug {
	/// 矩形オブジェクトとしてダウンキャスト（参照）
	fn as_rectangle(&self) -> Option<&RectangleObject> {
		None
	}

	/// 矩形オブジェクトとしてダウンキャスト（可変参照）
	fn as_rectangle_mut(&mut self) -> Option<&mut RectangleObject> {
		None
	}
}
