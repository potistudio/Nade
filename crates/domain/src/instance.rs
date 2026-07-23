use core::{NodeId, Transform, id::InstanceId};

/// インスタンスが持つ描画コンテンツ
#[derive(Debug, Clone)]
pub enum InstanceContent {
	/// 中身のないプレースホルダ
	Empty,
	/// テキストオブジェクト
	Text {
		text: String,
		font_path: String,
		font_size: f32,
		spacing: f32,
		fill_color: [f32; 4],
	},
}

impl Default for InstanceContent {
	fn default() -> Self {
		Self::Empty
	}
}

impl InstanceContent {
	/// デフォルトのテキストコンテンツ
	pub fn text_default() -> Self {
		Self::Text {
			text: "Text".to_string(),
			font_path: "assets/fonts/Rubik/Rubik_regular.ttf".to_string(),
			font_size: 48.0,
			spacing: 0.0,
			fill_color: [1.0, 1.0, 1.0, 1.0],
		}
	}

	/// 指定文字列のテキストコンテンツ
	pub fn text(text: impl Into<String>) -> Self {
		Self::Text {
			text: text.into(),
			font_path: "assets/fonts/Rubik/Rubik_regular.ttf".to_string(),
			font_size: 48.0,
			spacing: 0.0,
			fill_color: [1.0, 1.0, 1.0, 1.0],
		}
	}

	pub fn as_text(&self) -> Option<(&str, &str, f32, f32, [f32; 4])> {
		match self {
			Self::Text {
				text,
				font_path,
				font_size,
				spacing,
				fill_color,
			} => Some((text.as_str(), font_path.as_str(), *font_size, *spacing, *fill_color)),
			Self::Empty => None,
		}
	}

	pub fn text_mut(&mut self) -> Option<&mut String> {
		match self {
			Self::Text { text, .. } => Some(text),
			Self::Empty => None,
		}
	}

	pub fn font_size_mut(&mut self) -> Option<&mut f32> {
		match self {
			Self::Text { font_size, .. } => Some(font_size),
			Self::Empty => None,
		}
	}

	pub fn fill_color_mut(&mut self) -> Option<&mut [f32; 4]> {
		match self {
			Self::Text { fill_color, .. } => Some(fill_color),
			Self::Empty => None,
		}
	}
}

/// タイムラインクリップ
///
/// シーンオブジェクトをタイムライン上で視覚的に表現するためのクリップです。
#[derive(Debug)]
pub struct Instance {
	/// ID of the instance
	id: InstanceId,

	target_node_id: NodeId,

	/// Name of the instance
	pub name: String,

	/// Start time of the instance (sec)
	pub start_time: f32,

	/// Duration of the instance (sec)
	pub duration: f32,

	/// Transform of the instance
	pub transform: Transform,

	/// Drawable content
	pub content: InstanceContent,
}

impl Instance {
	/// Create a new instance
	pub fn new(id: InstanceId, target_node_id: NodeId, name: &str, start_time: f32, duration: f32) -> Self {
		Self {
			id,
			target_node_id,
			name: name.to_string(),
			start_time,
			duration,
			transform: Transform::default(),
			content: InstanceContent::default(),
		}
	}

	pub fn id(&self) -> InstanceId {
		self.id
	}

	pub fn target_node_id(&self) -> NodeId {
		self.target_node_id
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn start_time(&self) -> f32 {
		self.start_time
	}

	pub fn duration(&self) -> f32 {
		self.duration
	}

	pub fn transform(&self) -> &Transform {
		&self.transform
	}

	pub fn transform_mut(&mut self) -> &mut Transform {
		&mut self.transform
	}

	pub fn set_transform(&mut self, transform: Transform) {
		self.transform = transform;
	}

	pub fn content(&self) -> &InstanceContent {
		&self.content
	}

	pub fn content_mut(&mut self) -> &mut InstanceContent {
		&mut self.content
	}

	pub fn set_content(&mut self, content: InstanceContent) {
		self.content = content;
	}

	pub fn set_start_time(&mut self, time: f32) {
		self.start_time = time;
	}

	pub fn set_duration(&mut self, duration: f32) {
		self.duration = duration;
	}

	/// Get the end time of the instance
	pub fn end_time(&self) -> f32 {
		self.start_time + self.duration
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use core::NodeId;

	#[test]
	fn text_default_has_visible_string() {
		let content = InstanceContent::text_default();
		let (text, _, size, _, _) = content.as_text().expect("text content");
		assert_eq!(text, "Text");
		assert!((size - 48.0).abs() < f32::EPSILON);
	}

	#[test]
	fn new_instance_starts_empty() {
		let instance = Instance::new(InstanceId::new(0), NodeId::new(0), "A", 0.0, 5.0);
		assert!(matches!(instance.content(), InstanceContent::Empty));
	}
}
