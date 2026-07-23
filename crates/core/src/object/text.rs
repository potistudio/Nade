use super::{SceneObject, SceneObjectData, SceneObjectId};
use crate::Transform;

/// テキスト描画オブジェクト
///
/// 指定された文字列をフォント・サイズ・色で描画します。
/// トランスフォーム（位置、回転、スケール）を適用可能です。
#[derive(Debug, Clone)]
pub struct TextObject {
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
	/// 表示する文字列
	pub text: String,
	/// フォントファイルパス
	pub font_path: String,
	/// フォントサイズ（ピクセル、ppem）
	pub font_size: f32,
	/// 文字間隔（ピクセル）
	pub spacing: f32,
	/// 塗りつぶし色 (RGBA, 0.0-1.0)
	pub fill_color: [f32; 4],
}

impl Default for TextObject {
	fn default() -> Self {
		Self {
			id: SceneObjectId(0),
			name: "Text".to_string(),
			transform: Transform::default(),
			start_time: 0.0,
			duration: 5.0,
			text: "Text".to_string(),
			font_path: "assets/fonts/Rubik/Rubik_regular.ttf".to_string(),
			font_size: 48.0,
			spacing: 0.0,
			fill_color: [1.0, 1.0, 1.0, 1.0],
		}
	}
}

impl TextObject {
	/// 新しいテキストオブジェクトを作成
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

	/// 表示文字列を設定するビルダーメソッド
	pub fn with_text(mut self, text: impl Into<String>) -> Self {
		self.text = text.into();
		self
	}

	/// フォントパスを設定するビルダーメソッド
	pub fn with_font_path(mut self, path: impl Into<String>) -> Self {
		self.font_path = path.into();
		self
	}

	/// フォントサイズを設定するビルダーメソッド
	pub fn with_font_size(mut self, size: f32) -> Self {
		self.font_size = size;
		self
	}

	/// 文字間隔を設定するビルダーメソッド
	pub fn with_spacing(mut self, spacing: f32) -> Self {
		self.spacing = spacing;
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

impl SceneObject for TextObject {
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

impl SceneObjectData for TextObject {
	fn as_text(&self) -> Option<&TextObject> {
		Some(self)
	}

	fn as_text_mut(&mut self) -> Option<&mut TextObject> {
		Some(self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_text_object_is_visible_in_range() {
		let text = TextObject::default();
		assert!(text.is_visible_at(0.0));
		assert!(text.is_visible_at(4.9));
		assert!(!text.is_visible_at(5.0));
	}

	#[test]
	fn builder_sets_text_properties() {
		let text = TextObject::new("Title")
			.with_text("Hello")
			.with_font_size(72.0)
			.with_spacing(2.0)
			.with_fill_color(1.0, 0.0, 0.0, 1.0)
			.with_start_time(1.0)
			.with_duration(2.0);

		assert_eq!(text.name(), "Title");
		assert_eq!(text.text, "Hello");
		assert!((text.font_size - 72.0).abs() < f32::EPSILON);
		assert!((text.spacing - 2.0).abs() < f32::EPSILON);
		assert_eq!(text.fill_color, [1.0, 0.0, 0.0, 1.0]);
		assert!((text.start_time() - 1.0).abs() < f32::EPSILON);
		assert!((text.duration() - 2.0).abs() < f32::EPSILON);
		assert!(text.as_text().is_some());
		assert!(text.as_rectangle().is_none());
	}
}
