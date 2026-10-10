use super::{SceneObject, SceneObjectData, SceneObjectId};
use crate::Transform;

// =============================================================================
// 矩形オブジェクト
// =============================================================================

/// 矩形描画オブジェクト
///
/// 指定された位置、サイズ、色で矩形を描画します。
/// トランスフォーム（位置、回転、スケール）を適用可能です。
#[derive(Debug)]
pub struct RectangleObject {
	/// オブジェクトID
	pub id: SceneObjectId,
	/// オブジェクト名
	pub name: String,
	/// トランスフォーム
	pub transform: Transform,
	/// 開始時間（秒）
	pub start_time: f32,
	/// 持続時間（秒）
	pub duration: f32,
	/// 塗りつぶし色 (RGBA, 0.0-1.0)
	pub fill_color: [f32; 4],
	/// 幅（ローカル単位、ピクセル）
	pub width: f32,
	/// 高さ（ローカル単位、ピクセル）
	pub height: f32,
}

impl Default for RectangleObject {
	fn default() -> Self {
		Self {
			id: SceneObjectId(0),
			name: "Rectangle".to_string(),
			transform: Transform::default(),
			start_time: 0.0,
			duration: 5.0,
			fill_color: [1.0, 1.0, 1.0, 1.0], // 白
			width: 100.0,
			height: 100.0,
		}
	}
}

impl RectangleObject {
	/// 新しい矩形オブジェクトを作成
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			..Default::default()
		}
	}

	/// 開始時間を設定するビルダーメソッド
	pub fn with_start_time(mut self, time: f32) -> Self {
		self.start_time = time;
		self
	}

	/// 持続時間を設定するビルダーメソッド
	pub fn with_duration(mut self, duration: f32) -> Self {
		self.duration = duration;
		self
	}

	/// サイズを設定するビルダーメソッド
	pub fn with_size(mut self, width: f32, height: f32) -> Self {
		self.width = width;
		self.height = height;
		self
	}

	/// 塗りつぶし色を設定するビルダーメソッド
	pub fn with_fill_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
		self.fill_color = [r, g, b, a];
		self
	}

	/// トランスフォームを設定するビルダーメソッド
	pub fn with_transform(mut self, transform: Transform) -> Self {
		self.transform = transform;
		self
	}
}

impl SceneObject for RectangleObject {
	fn id(&self) -> SceneObjectId {
		self.id
	}

	fn name(&self) -> &str {
		&self.name
	}

	fn transform(&self) -> &Transform {
		&self.transform
	}

	fn set_transform(&mut self, transform: Transform) {
		self.transform = transform;
	}

	fn start_time(&self) -> f32 {
		self.start_time
	}

	fn end_time(&self) -> f32 {
		self.start_time + self.duration
	}

	fn set_start_time(&mut self, time: f32) {
		self.start_time = time;
	}

	fn set_duration(&mut self, duration: f32) {
		self.duration = duration;
	}
}

impl SceneObjectData for RectangleObject {
	fn as_rectangle(&self) -> Option<&RectangleObject> {
		Some(self)
	}

	fn as_rectangle_mut(&mut self) -> Option<&mut RectangleObject> {
		Some(self)
	}
}
