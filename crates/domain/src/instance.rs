use core::{NodeId, Transform, id::InstanceId};

/// タイムラインクリップ
///
/// シーンオブジェクトをタイムライン上で視覚的に表現するためのクリップです。
/// `scene_object_id` でシーンオブジェクトと紐付けられています。
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
