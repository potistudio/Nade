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
		// 既存オブジェクトの後ろに並べて、タイムライン上で重ならないようにする
		let start_time = self
			.instances
			.iter()
			.map(|instance| instance.end_time())
			.fold(0.0_f32, f32::max);
		let instance = Instance::new(next_id, node_id, &name, start_time, 5.0);

		self.instances.push(instance);
		self.instances.last_mut().unwrap() // safe because we just pushed an element
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
