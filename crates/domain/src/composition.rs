use core::{CompositionId, NodeId, id::InstanceId};

use crate::instance::Instance;

/// Composition is a container that manages multiple instances and controls them over time.
#[derive(Debug, Clone, PartialEq)]
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

	/// Background color (linear RGBA, 0..1)
	background: [f32; 4],

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
			background: [0.0, 0.0, 0.0, 1.0],
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

	/// Returns the width of the composition in pixels
	pub fn width(&self) -> u32 {
		self.width
	}

	/// Returns the height of the composition in pixels
	pub fn height(&self) -> u32 {
		self.height
	}

	/// Returns the frame rate of the composition
	pub fn fps(&self) -> f32 {
		self.fps
	}

	/// Returns the playable duration of the composition in seconds
	pub fn duration(&self) -> f32 {
		(self.end_time - self.start_time).max(0.0) as f32
	}

	/// Changes the playable duration (seconds, at least one frame).
	pub fn set_duration(&mut self, seconds: f32) {
		let frame = 1.0 / self.fps.max(1.0);
		self.end_time = self.start_time + f64::from(seconds.max(frame));
	}

	/// Background color (RGBA, 0..1)
	pub fn background(&self) -> [f32; 4] {
		self.background
	}

	pub fn set_background(&mut self, color: [f32; 4]) {
		self.background = color.map(|channel| channel.clamp(0.0, 1.0));
	}

	/// End of the last layer in seconds.
	pub fn content_end(&self) -> f32 {
		self.instances.iter().map(Instance::end_time).fold(0.0, f32::max)
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
		// 先頭から重ね、空いているトラック（レイヤー）に配置する
		self.add_instance_at(node_id, name, 0.0, 5.0)
	}

	/// `[start, start+duration)` に新しいインスタンスを空いているトラックへ追加する
	pub fn add_instance_at(
		&mut self,
		node_id: NodeId,
		name: impl Into<String>,
		start_time: f32,
		duration: f32,
	) -> &mut Instance {
		let name = name.into();
		let instance = Instance::new(self.next_instance_id(), node_id, &name, start_time, duration);
		let id = self.insert_instance(instance, None);
		self.get_mut(id).expect("just inserted")
	}

	/// 削除後も ID が衝突しないよう、既存の最大 ID の次を採番する
	fn next_instance_id(&self) -> InstanceId {
		InstanceId::new(
			self.instances
				.iter()
				.map(|instance| instance.id().value() + 1)
				.max()
				.unwrap_or(0),
		)
	}

	/// 既存インスタンスの複製（コピー＆ペースト等）を新しい ID で追加する。
	/// `track` が空いていればそこへ、そうでなければ空いているトラックへ置く。
	pub fn insert_instance(&mut self, mut instance: Instance, track: Option<usize>) -> InstanceId {
		let id = self.next_instance_id();
		instance.set_id(id);
		let (start, duration) = (instance.start_time(), instance.duration());
		let track_index = track
			.filter(|track| self.is_track_free(*track, start, duration, None))
			.unwrap_or_else(|| self.find_free_track(start, duration));
		instance.set_track_index(track_index);
		self.instances.push(instance);
		id
	}

	/// 同じ区間に複製を追加する（After Effects の Duplicate Layer）
	pub fn duplicate_instance(&mut self, id: InstanceId) -> Option<InstanceId> {
		let copy = self.get(id)?.clone();
		Some(self.insert_instance(copy, None))
	}

	/// `time` でインスタンスを 2 つに分割し、後半の ID を返す
	pub fn split_instance(&mut self, id: InstanceId, time: f32) -> Option<InstanceId> {
		let next_id = self.next_instance_id();
		let instance = self.get_mut(id)?;
		let mut right = instance.split_at(time)?;
		right.set_id(next_id);
		self.instances.push(right);
		Some(next_id)
	}

	/// `time` を跨ぐインスタンスの ID（タイムラインの上から順）
	pub fn instances_at(&self, time: f32) -> Vec<InstanceId> {
		let mut hits: Vec<&Instance> = self
			.instances
			.iter()
			.filter(|instance| instance.is_active_at(time))
			.collect();
		hits.sort_by_key(|instance| instance.track_index());
		hits.into_iter().map(Instance::id).collect()
	}

	fn is_track_free(&self, track_index: usize, start_time: f32, duration: f32, exclude: Option<InstanceId>) -> bool {
		let end = start_time + duration;
		!self.instances.iter().any(|instance| {
			Some(instance.id()) != exclude
				&& instance.track_index() == track_index
				&& instance.start_time() < end
				&& instance.end_time() > start_time
		})
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
	fn instance_ids_stay_unique_after_removal() {
		let mut composition = Composition::new(CompositionId::new(0), "Test", 1920, 1080, 60.0);
		let a = composition.add_instance_named(NodeId::new(0), "A").id();
		let b = composition.add_instance_named(NodeId::new(1), "B").id();
		composition.remove(a);
		let c = composition.add_instance_named(NodeId::new(2), "C").id();

		assert_ne!(b, c);
		assert!(composition.get(b).is_some_and(|instance| instance.name() == "B"));
	}

	#[test]
	fn split_creates_adjacent_instance_on_same_track() {
		let mut composition = Composition::new(CompositionId::new(0), "Test", 1920, 1080, 60.0);
		let a = composition.add_instance_named(NodeId::new(0), "A").id();
		let b = composition.split_instance(a, 2.0).expect("2s is inside 0..5");

		let (left, right) = (composition.get(a).unwrap(), composition.get(b).unwrap());
		assert_ne!(a, b);
		assert_eq!(left.end_time(), 2.0);
		assert_eq!(right.start_time(), 2.0);
		assert_eq!(right.end_time(), 5.0);
		assert_eq!(left.track_index(), right.track_index());
		assert!(composition.split_instance(a, 4.0).is_none());
	}

	#[test]
	fn duplicate_goes_to_a_free_track() {
		let mut composition = Composition::new(CompositionId::new(0), "Test", 1920, 1080, 60.0);
		let a = composition.add_instance_named(NodeId::new(0), "A").id();
		let copy = composition.duplicate_instance(a).expect("source exists");

		let copy = composition.get(copy).unwrap();
		assert_eq!(copy.name(), "A");
		assert_ne!(copy.track_index(), composition.get(a).unwrap().track_index());
	}

	#[test]
	fn insert_prefers_requested_free_track() {
		let mut composition = Composition::new(CompositionId::new(0), "Test", 1920, 1080, 60.0);
		let a = composition.add_instance_named(NodeId::new(0), "A").id();
		let mut later = composition.get(a).unwrap().clone();
		later.set_start_time(6.0);
		let pasted = composition.insert_instance(later, Some(0));
		assert_eq!(composition.get(pasted).unwrap().track_index(), 0);
		assert_eq!(composition.instances_at(7.0), vec![pasted]);
	}

	#[test]
	fn duration_is_at_least_one_frame() {
		let mut composition = Composition::new(CompositionId::new(0), "Test", 1920, 1080, 10.0);
		composition.set_duration(0.0);
		assert!((composition.duration() - 0.1).abs() < 1e-6);
		composition.set_duration(42.0);
		assert_eq!(composition.duration(), 42.0);
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
