use core::{AnimationCurve, AssetId, BlendMode, CompositionId, MediaInfo, NodeId, Transform, id::InstanceId};
use std::path::PathBuf;

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
	AnchorX,
	AnchorY,
}

impl TransformProperty {
	pub const ALL: [Self; 12] = [
		Self::AnchorX,
		Self::AnchorY,
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
			Self::AnchorX => "Anchor X",
			Self::AnchorY => "Anchor Y",
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
			Self::AnchorX => transform.anchor[0],
			Self::AnchorY => transform.anchor[1],
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
			Self::AnchorX => transform.anchor[0] = value,
			Self::AnchorY => transform.anchor[1] = value,
		}
	}
}

/// Keyframe curves for every scalar transform channel.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransformAnimation {
	curves: [AnimationCurve; 10],
	/// Anchor X / Y (stored apart so files written before anchors still load).
	#[serde(default)]
	anchor: [AnimationCurve; 2],
}

impl TransformAnimation {
	pub fn curve(&self, property: TransformProperty) -> &AnimationCurve {
		match property {
			TransformProperty::AnchorX => &self.anchor[0],
			TransformProperty::AnchorY => &self.anchor[1],
			_ => &self.curves[property as usize],
		}
	}

	pub fn curve_mut(&mut self, property: TransformProperty) -> &mut AnimationCurve {
		match property {
			TransformProperty::AnchorX => &mut self.anchor[0],
			TransformProperty::AnchorY => &mut self.anchor[1],
			_ => &mut self.curves[property as usize],
		}
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

/// Kind of footage a media layer plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MediaKind {
	Image,
	Video,
	Audio,
}

/// Footage referenced by a media layer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MediaSource {
	/// Asset the layer was created from.
	pub asset: AssetId,
	pub kind: MediaKind,
	/// File on disk.
	pub path: PathBuf,
	pub info: MediaInfo,
}

impl MediaSource {
	/// Size of the picture in composition pixels (zero for audio).
	pub fn size(&self) -> [f32; 2] {
		match self.kind {
			MediaKind::Audio => [0.0, 0.0],
			MediaKind::Image | MediaKind::Video => [self.info.width as f32, self.info.height as f32],
		}
	}

	/// Whether the footage carries sound.
	pub fn has_audio(&self) -> bool {
		matches!(self.kind, MediaKind::Video | MediaKind::Audio) && self.info.has_audio
	}
}

/// インスタンスが持つ描画コンテンツ
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum InstanceContent {
	/// 中身のないプレースホルダ
	#[default]
	Empty,
	/// テキストオブジェクト
	Text {
		text: String,
		font_path: String,
		font_size: f32,
		spacing: f32,
		fill_color: [f32; 4],
	},
	/// 単色の平面（ソリッド）
	Solid { width: f32, height: f32, color: [f32; 4] },
	/// 画像・動画・音声フッテージ
	Media(MediaSource),
	/// 別のコンポジション（ネスト / プリコンポーズ）
	Composition(CompositionId),
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
			_ => None,
		}
	}

	pub fn text_mut(&mut self) -> Option<&mut String> {
		match self {
			Self::Text { text, .. } => Some(text),
			_ => None,
		}
	}

	pub fn font_size_mut(&mut self) -> Option<&mut f32> {
		match self {
			Self::Text { font_size, .. } => Some(font_size),
			_ => None,
		}
	}

	pub fn spacing_mut(&mut self) -> Option<&mut f32> {
		match self {
			Self::Text { spacing, .. } => Some(spacing),
			_ => None,
		}
	}

	pub fn fill_color_mut(&mut self) -> Option<&mut [f32; 4]> {
		match self {
			Self::Text { fill_color, .. } | Self::Solid { color: fill_color, .. } => Some(fill_color),
			_ => None,
		}
	}

	/// A solid of the given size and color.
	pub fn solid(width: f32, height: f32, color: [f32; 4]) -> Self {
		Self::Solid { width, height, color }
	}

	/// `[width, height]` and color of a solid.
	pub fn as_solid(&self) -> Option<([f32; 2], [f32; 4])> {
		match self {
			Self::Solid { width, height, color } => Some(([*width, *height], *color)),
			_ => None,
		}
	}

	pub fn solid_size_mut(&mut self) -> Option<(&mut f32, &mut f32)> {
		match self {
			Self::Solid { width, height, .. } => Some((width, height)),
			_ => None,
		}
	}

	pub fn as_media(&self) -> Option<&MediaSource> {
		match self {
			Self::Media(source) => Some(source),
			_ => None,
		}
	}

	/// Composition a nested-comp layer plays.
	pub fn as_composition(&self) -> Option<CompositionId> {
		match self {
			Self::Composition(id) => Some(*id),
			_ => None,
		}
	}

	/// Whether the layer's length is bound to footage that has its own timing.
	pub fn is_timed_media(&self) -> bool {
		self.as_media()
			.is_some_and(|source| matches!(source.kind, MediaKind::Video | MediaKind::Audio))
	}

	/// Human readable layer kind.
	pub fn kind_label(&self) -> &'static str {
		match self {
			Self::Empty => "Null Object",
			Self::Text { .. } => "Text Layer",
			Self::Solid { .. } => "Solid Layer",
			Self::Media(source) => match source.kind {
				MediaKind::Image => "Image Layer",
				MediaKind::Video => "Video Layer",
				MediaKind::Audio => "Audio Layer",
			},
			Self::Composition(_) => "Composition Layer",
		}
	}
}

/// コンポジション上のインスタンス（タイムラインクリップのソース）
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

	/// フッテージの再生開始位置（秒）。`start_time` の瞬間に表示されるソース時刻。
	pub source_offset: f32,

	/// Transform of the instance
	pub transform: Transform,

	/// Time-varying transform channels
	pub animation: TransformAnimation,

	/// Drawable content
	pub content: InstanceContent,

	/// Layer whose transform this one inherits (AE-style parenting).
	#[serde(default)]
	pub parent: Option<InstanceId>,

	/// How this layer composites onto the layers below.
	#[serde(default)]
	pub blend_mode: BlendMode,
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
			source_offset: 0.0,
			transform: Transform::default(),
			animation: TransformAnimation::default(),
			content: InstanceContent::default(),
			parent: None,
			blend_mode: BlendMode::Normal,
		}
	}

	pub fn id(&self) -> InstanceId {
		self.id
	}

	pub(crate) fn set_id(&mut self, id: InstanceId) {
		self.id = id;
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

	/// Whether the layer is shown at composition time `time`.
	pub fn is_active_at(&self, time: f32) -> bool {
		time >= self.start_time && time < self.end_time()
	}

	/// Footage time shown at composition time `time`.
	pub fn source_time_at(&self, time: f32) -> f32 {
		(time - self.start_time + self.source_offset).max(0.0)
	}

	/// Cut the layer at `time`. Shortens `self` to end at `time` and returns
	/// the right half (same id until the composition assigns a new one), or
	/// `None` when `time` does not fall strictly inside the layer.
	pub fn split_at(&mut self, time: f32) -> Option<Self> {
		const MIN_PART: f32 = 1.0 / 240.0;
		if time <= self.start_time + MIN_PART || time >= self.end_time() - MIN_PART {
			return None;
		}
		let mut right = self.clone();
		let left_duration = time - self.start_time;
		right.start_time = time;
		right.duration = self.duration - left_duration;
		right.source_offset = self.source_offset + left_duration;
		self.duration = left_duration;
		Some(right)
	}

	/// Whether the channel has keyframes.
	pub fn is_animated(&self, property: TransformProperty) -> bool {
		!self.animation.curve(property).is_empty()
	}

	/// Evaluated channel value at `time`.
	pub fn property_at(&self, property: TransformProperty, time: f32) -> f32 {
		self.animation
			.curve(property)
			.evaluate(property.value(&self.transform), time)
	}

	/// Write a channel the way motion graphics tools do: animated channels get
	/// a keyframe at `time`, static channels change the base value.
	pub fn set_property_at(&mut self, property: TransformProperty, time: f32, value: f32) {
		let value = match property {
			TransformProperty::Opacity => value.clamp(0.0, 1.0),
			_ => value,
		};
		if self.is_animated(property) {
			self.animation.curve_mut(property).add_key(time, value);
		} else {
			property.set_value(&mut self.transform, value);
		}
	}

	/// Stopwatch toggle: removes the key at `time`, or keys the current value.
	/// Returns `true` when a key was added.
	pub fn toggle_key_at(&mut self, property: TransformProperty, time: f32) -> bool {
		let value = self.property_at(property, time);
		let curve = self.animation.curve_mut(property);
		if let Some(id) = curve.key_at(time).map(|key| key.id) {
			curve.remove_key(id);
			// 最後のキーを消したら静的な値として残す
			if curve.is_empty() {
				property.set_value(&mut self.transform, value);
			}
			false
		} else {
			curve.add_key(time, value);
			true
		}
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
	fn editing_animated_channel_sets_key() {
		let mut instance = Instance::new(InstanceId::new(0), NodeId::new(0), "A", 0.0, 5.0);
		instance.set_property_at(TransformProperty::PositionX, 1.0, 40.0);
		assert_eq!(instance.transform.position[0], 40.0);
		assert!(!instance.is_animated(TransformProperty::PositionX));

		assert!(instance.toggle_key_at(TransformProperty::PositionX, 0.0));
		instance.set_property_at(TransformProperty::PositionX, 2.0, 100.0);
		let curve = instance.animation.curve(TransformProperty::PositionX);
		assert_eq!(curve.keys().len(), 2);
		assert_eq!(instance.property_at(TransformProperty::PositionX, 2.0), 100.0);
	}

	#[test]
	fn removing_last_key_keeps_value() {
		let mut instance = Instance::new(InstanceId::new(0), NodeId::new(0), "A", 0.0, 5.0);
		instance.toggle_key_at(TransformProperty::ScaleX, 1.0);
		instance.set_property_at(TransformProperty::ScaleX, 1.0, 2.5);
		assert!(!instance.toggle_key_at(TransformProperty::ScaleX, 1.0));
		assert!(!instance.is_animated(TransformProperty::ScaleX));
		assert_eq!(instance.transform.scale[0], 2.5);
	}

	#[test]
	fn split_keeps_footage_continuous() {
		let mut instance = Instance::new(InstanceId::new(0), NodeId::new(0), "A", 1.0, 4.0);
		instance.source_offset = 0.5;
		let right = instance.split_at(2.0).expect("inside the layer");

		assert_eq!(instance.duration(), 1.0);
		assert_eq!(right.start_time(), 2.0);
		assert_eq!(right.duration(), 3.0);
		assert!((right.source_offset - 1.5).abs() < 1e-6);
		// 分割点の前後でソース時刻が連続する
		assert!((instance.source_time_at(1.999) - right.source_time_at(2.0)).abs() < 0.01);
		assert!(instance.split_at(1.0).is_none());
		assert!(instance.split_at(5.0).is_none());
	}

	#[test]
	fn solid_color_is_editable_as_fill() {
		let mut content = InstanceContent::solid(100.0, 50.0, [1.0, 0.0, 0.0, 1.0]);
		content.fill_color_mut().expect("solid has a fill")[1] = 1.0;
		assert_eq!(content.as_solid(), Some(([100.0, 50.0], [1.0, 1.0, 0.0, 1.0])));
		assert_eq!(content.kind_label(), "Solid Layer");
	}

	#[test]
	fn animated_opacity_is_clamped() {
		let mut animation = TransformAnimation::default();
		animation.curve_mut(TransformProperty::Opacity).add_key(0.0, 2.0);
		assert_eq!(animation.evaluate(Transform::default(), 0.0).opacity, 1.0);
	}
}
