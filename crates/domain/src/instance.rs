use core::{AnimationCurve, NodeId, Transform, id::InstanceId};

/// A scalar channel of an instance transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum TransformProperty {
	PositionX,
	PositionY,
	PositionZ,
	RotationX,
	RotationY,
	RotationZ,
	ScaleX,
	ScaleY,
	ScaleZ,
	Opacity,
}

impl TransformProperty {
	pub const ALL: [Self; 10] = [
		Self::PositionX,
		Self::PositionY,
		Self::PositionZ,
		Self::RotationX,
		Self::RotationY,
		Self::RotationZ,
		Self::ScaleX,
		Self::ScaleY,
		Self::ScaleZ,
		Self::Opacity,
	];

	pub fn label(self) -> &'static str {
		match self {
			Self::PositionX => "Position X",
			Self::PositionY => "Position Y",
			Self::PositionZ => "Position Z",
			Self::RotationX => "Rotation X",
			Self::RotationY => "Rotation Y",
			Self::RotationZ => "Rotation Z",
			Self::ScaleX => "Scale X",
			Self::ScaleY => "Scale Y",
			Self::ScaleZ => "Scale Z",
			Self::Opacity => "Opacity",
		}
	}

	pub fn value(self, transform: &Transform) -> f32 {
		match self {
			Self::PositionX => transform.position[0],
			Self::PositionY => transform.position[1],
			Self::PositionZ => transform.position[2],
			Self::RotationX => transform.rotation[0],
			Self::RotationY => transform.rotation[1],
			Self::RotationZ => transform.rotation[2],
			Self::ScaleX => transform.scale[0],
			Self::ScaleY => transform.scale[1],
			Self::ScaleZ => transform.scale[2],
			Self::Opacity => transform.opacity,
		}
	}

	fn set_value(self, transform: &mut Transform, value: f32) {
		match self {
			Self::PositionX => transform.position[0] = value,
			Self::PositionY => transform.position[1] = value,
			Self::PositionZ => transform.position[2] = value,
			Self::RotationX => transform.rotation[0] = value,
			Self::RotationY => transform.rotation[1] = value,
			Self::RotationZ => transform.rotation[2] = value,
			Self::ScaleX => transform.scale[0] = value,
			Self::ScaleY => transform.scale[1] = value,
			Self::ScaleZ => transform.scale[2] = value,
			Self::Opacity => transform.opacity = value.clamp(0.0, 1.0),
		}
	}
}

/// Keyframe curves for every scalar transform channel.
#[derive(Debug, Clone, Default)]
pub struct TransformAnimation {
	curves: [AnimationCurve; 10],
}

impl TransformAnimation {
	pub fn curve(&self, property: TransformProperty) -> &AnimationCurve {
		&self.curves[property as usize]
	}

	pub fn curve_mut(&mut self, property: TransformProperty) -> &mut AnimationCurve {
		&mut self.curves[property as usize]
	}

	pub fn evaluate(&self, base: Transform, time: f32) -> Transform {
		let mut result = base;
		for property in TransformProperty::ALL {
			let value = self.curve(property).evaluate(property.value(&base), time);
			property.set_value(&mut result, value);
		}
		result
	}
}

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

/// コンポジション上のインスタンス（タイムラインクリップのソース）
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

	/// タイムライン上のトラック（レイヤー）インデックス。0 が最前面。
	pub track_index: usize,

	/// Transform of the instance
	pub transform: Transform,

	/// Time-varying transform channels
	pub animation: TransformAnimation,

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
			track_index: 0,
			transform: Transform::default(),
			animation: TransformAnimation::default(),
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

	pub fn transform_at(&self, time: f32) -> Transform {
		self.animation.evaluate(self.transform, time)
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

	pub fn track_index(&self) -> usize {
		self.track_index
	}

	pub fn set_track_index(&mut self, track_index: usize) {
		self.track_index = track_index;
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

	#[test]
	fn transform_at_evaluates_animated_channels_only() {
		let mut instance = Instance::new(InstanceId::new(0), NodeId::new(0), "A", 0.0, 5.0);
		instance.transform.position = [10.0, 20.0, 30.0];
		let curve = instance.animation.curve_mut(TransformProperty::PositionX);
		let first = curve.add_key(0.0, 0.0);
		curve.add_key(2.0, 100.0);
		curve.set_interpolation(first, core::Interpolation::Linear);

		let transform = instance.transform_at(1.0);
		assert!((transform.position[0] - 50.0).abs() < 0.001);
		assert_eq!(transform.position[1], 20.0);
	}

	#[test]
	fn animated_opacity_is_clamped() {
		let mut animation = TransformAnimation::default();
		animation.curve_mut(TransformProperty::Opacity).add_key(0.0, 2.0);
		assert_eq!(animation.evaluate(Transform::default(), 0.0).opacity, 1.0);
	}
}
