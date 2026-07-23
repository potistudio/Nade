use core::{CompositionId, NodeId, id::InstanceId};

use crate::instance::Instance;

/// Composition is a container that manages multiple instances and controls them over time.
#[derive(Debug)]
pub struct Composition {
	/// ID of the composition
	id: CompositionId,

	/// Name of the composition
	name: String,

	/// Width of the composition in pixels
	width: u32,

	/// Height of the composition in pixels
	height: u32,

	/// Frame rate of the composition
	fps: f32,

	/// Start time of the composition in seconds
	start_time: f64,

	/// End time of the composition in seconds
	end_time: f64,

	/// Vector of instances in the composition
	instances: Vec<Instance>,
}

impl Composition {
	//==== Constructor =========================================================
	/// Creates a new composition with the given ID and name
	pub fn new(id: CompositionId, name: impl Into<String>, width: u32, height: u32, fps: f32) -> Self {
		Self {
			id,
			name: name.into(),
			width,
			height,
			fps,
			start_time: 0.0,
			end_time: 10.0, // デフォルトの終了時間は10秒
			instances: Vec::new(),
		}
	}

	//==== Getter ==============================================================
	/// Returns the ID of the composition
	pub fn id(&self) -> CompositionId {
		self.id
	}

	/// Returns the name of the composition
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Returns whether this composition has any objects
	pub fn has_objects(&self) -> bool {
		!self.instances.is_empty()
	}

	//==== Add Method ==========================================================
	pub fn add_instance(&mut self, node_id: NodeId) -> &mut Instance {
		self.add_instance_named(node_id, "Instance")
	}

	pub fn add_instance_named(&mut self, node_id: NodeId, name: impl Into<String>) -> &mut Instance {
		let next_id = InstanceId::new(self.instances.len());
		let name = name.into();
		// 先頭から重ね、空いているトラック（レイヤー）に配置する
		let start_time = 0.0;
		let duration = 5.0;
		let track_index = self.find_free_track(start_time, duration);
		let mut instance = Instance::new(next_id, node_id, &name, start_time, duration);
		instance.set_track_index(track_index);

		self.instances.push(instance);
		self.instances.last_mut().unwrap() // safe because we just pushed an element
	}

	/// `[start, start+duration)` と重ならない最初のトラックを返す
	fn find_free_track(&self, start_time: f32, duration: f32) -> usize {
		let end = start_time + duration;
		let max_track = self
			.instances
			.iter()
			.map(|instance| instance.track_index())
			.max()
			.unwrap_or(0);

		for track_index in 0..=max_track {
			let occupied = self.instances.iter().any(|instance| {
				instance.track_index() == track_index && instance.start_time() < end && instance.end_time() > start_time
			});
			if !occupied {
				return track_index;
			}
		}

		max_track + 1
	}

	/// 描画順（背面 → 前面）。トラック番号が大きいほど背面。
	pub fn objects_in_draw_order(&self) -> impl Iterator<Item = &Instance> {
		let mut ordered: Vec<&Instance> = self.instances.iter().collect();
		ordered.sort_by(|a, b| b.track_index().cmp(&a.track_index()).then_with(|| a.id().cmp(&b.id())));
		ordered.into_iter()
	}

	/// タイムライン同期用: 全インスタンスを走査
	pub fn all_objects(&self) -> impl Iterator<Item = &Instance> {
		self.instances.iter()
	}

	/// IDでオブジェクトを取得
	pub fn get(&self, id: InstanceId) -> Option<&Instance> {
		self.instances.iter().find(|obj| obj.id() == id)
	}

	/// IDでオブジェクトを可変参照で取得
	pub fn get_mut(&mut self, id: InstanceId) -> Option<&mut Instance> {
		self.instances.iter_mut().find(|obj| obj.id() == id)
	}

	/// オブジェクトを削除
	pub fn remove(&mut self, id: InstanceId) {
		if let Some(pos) = self.instances.iter().position(|obj| obj.id() == id) {
			self.instances.remove(pos);
		}
	}

	/// オブジェクトの数を取得
	pub fn len(&self) -> usize {
		self.instances.len()
	}

	/// オブジェクトが空かどうか
	pub fn is_empty(&self) -> bool {
		self.instances.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use core::NodeId;

	#[test]
	fn new_instances_stack_on_free_tracks() {
		let mut composition = Composition::new(CompositionId::new(0), "Test", 1920, 1080, 60.0);
		let a = composition.add_instance_named(NodeId::new(0), "A").track_index();
		let b = composition.add_instance_named(NodeId::new(1), "B").track_index();
		let c = composition.add_instance_named(NodeId::new(2), "C").track_index();

		assert_eq!(a, 0);
		assert_eq!(b, 1);
		assert_eq!(c, 2);
		assert!(composition.all_objects().all(|instance| instance.start_time() == 0.0));
	}

	#[test]
	fn draw_order_puts_lower_track_on_top() {
		let mut composition = Composition::new(CompositionId::new(0), "Test", 1920, 1080, 60.0);
		composition.add_instance_named(NodeId::new(0), "Top");
		composition.add_instance_named(NodeId::new(1), "Bottom");

		let names: Vec<_> = composition
			.objects_in_draw_order()
			.map(|instance| instance.name().to_string())
			.collect();
		// 背面から前面へ: 大きい track_index が先、0 が最後（最前面）
		assert_eq!(names, vec!["Bottom".to_string(), "Top".to_string()]);
	}
}
