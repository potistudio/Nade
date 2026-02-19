use crate::{
	core::NodeId,
	instance::{Instance, InstanceId},
	object::RectangleObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompositionId(usize);

/// Composition
///
/// Composition is a container that manages multiple instances and controls them over time.
#[derive(Debug)]
pub struct Composition {
	/// ID of the composition
	id: CompositionId,

	/// Vector of instances contained in the composition
	objects: Vec<Instance>,
}

impl Composition {
	pub fn new() -> Self {
		Self {
			id: CompositionId(0),
			objects: Vec::new(),
		}
	}

	pub fn add_instance(&mut self, node_id: NodeId) -> &mut Instance {
		let next_id = InstanceId::new(self.objects.len());
		let instance = Instance::new(next_id, node_id, "Instance", 0.0, 5.0);

		self.objects.push(instance);
		self.objects.last_mut().unwrap() // safe because we just pushed an element
	}

	/// 既存UI互換: RectangleObject からインスタンスを生成
	pub fn add_rectangle(&mut self, rect: RectangleObject) -> InstanceId {
		let instance = self.add_instance(NodeId::new(self.objects.len()));
		instance.name = rect.name;
		instance.start_time = rect.start_time;
		instance.duration = rect.duration;
		instance.id()
	}

	/// タイムライン同期用: 全インスタンスを走査
	pub fn all_objects(&self) -> impl Iterator<Item = &Instance> {
		self.objects.iter()
	}

	/// IDでオブジェクトを取得
	pub fn get(&self, id: InstanceId) -> Option<&Instance> {
		self.objects.iter().find(|obj| obj.id() == id)
	}

	/// IDでオブジェクトを可変参照で取得
	pub fn get_mut(&mut self, id: InstanceId) -> Option<&mut Instance> {
		self.objects.iter_mut().find(|obj| obj.id() == id)
	}

	/// オブジェクトを削除
	pub fn remove(&mut self, id: InstanceId) {
		if let Some(pos) = self.objects.iter().position(|obj| obj.id() == id) {
			self.objects.remove(pos);
		}
	}

	/// オブジェクトの数を取得
	pub fn len(&self) -> usize {
		self.objects.len()
	}

	/// オブジェクトが空かどうか
	pub fn is_empty(&self) -> bool {
		self.objects.is_empty()
	}
}

impl Default for Composition {
	fn default() -> Self {
		Self::new()
	}
}
