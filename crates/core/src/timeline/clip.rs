use crate::{CompositionId, InstanceId, SceneObjectId};

/// タイムラインクリップ
#[derive(Debug, Clone)]
pub struct TimelineClip {
	pub id: usize,
	pub name: String,
	pub start_time: f32,
	pub duration: f32,
	pub scene_object_id: Option<SceneObjectId>,
	/// 紐づく Composition（オブジェクトブラウザ連携用）
	pub composition_id: Option<CompositionId>,
	/// 紐づく Instance（オブジェクトブラウザ連携用）
	pub instance_id: Option<InstanceId>,
}

impl TimelineClip {
	pub fn new(id: usize, name: &str, start_time: f32, duration: f32) -> Self {
		Self {
			id,
			name: name.to_string(),
			start_time,
			duration,
			scene_object_id: None,
			composition_id: None,
			instance_id: None,
		}
	}

	pub fn from_scene_object(
		id: usize,
		name: &str,
		start_time: f32,
		duration: f32,
		scene_object_id: SceneObjectId,
	) -> Self {
		Self {
			id,
			name: name.to_string(),
			start_time,
			duration,
			scene_object_id: Some(scene_object_id),
			composition_id: None,
			instance_id: None,
		}
	}

	pub fn from_instance(
		id: usize,
		name: &str,
		start_time: f32,
		duration: f32,
		composition_id: CompositionId,
		instance_id: InstanceId,
	) -> Self {
		Self {
			id,
			name: name.to_string(),
			start_time,
			duration,
			scene_object_id: None,
			composition_id: Some(composition_id),
			instance_id: Some(instance_id),
		}
	}

	#[inline]
	pub fn end_time(&self) -> f32 {
		self.start_time + self.duration
	}
}
