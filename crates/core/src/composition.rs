use crate::object::{SceneObjectData, SceneObjectId, rectangle::RectangleObject};

/// コンポジション（シーン）
///
/// 複数のシーンオブジェクトを管理し、時間に応じた可視性を制御します。
#[derive(Debug, Clone, Default)]
pub struct Composition {
	/// シーンオブジェクトのリスト
	pub objects: Vec<Box<dyn SceneObjectData>>,
	/// 次に割り当てるオブジェクトID
	next_id: u64,
}

impl Composition {
	/// 新しい空のコンポジションを作成
	pub fn new() -> Self {
		Self::default()
	}

	/// 次のオブジェクトIDを取得（内部使用）
	fn next_object_id(&mut self) -> SceneObjectId {
		let id = SceneObjectId(self.next_id);
		self.next_id += 1;
		id
	}

	/// 矩形オブジェクトを追加
	///
	/// オブジェクトにはユニークなIDが自動的に割り当てられます。
	pub fn add_rectangle(&mut self, mut rect: RectangleObject) -> SceneObjectId {
		let id = self.next_object_id();
		rect.id = id;
		self.objects.push(Box::new(rect));
		id
	}

	/// 指定時間で可視なオブジェクトを取得
	pub fn visible_objects_at(&self, time: f32) -> Vec<&dyn SceneObjectData> {
		self.objects
			.iter()
			.filter(|obj| obj.is_visible_at(time))
			.map(|obj| obj.as_ref())
			.collect()
	}

	/// IDでオブジェクトを取得
	pub fn get(&self, id: SceneObjectId) -> Option<&dyn SceneObjectData> {
		self.objects
			.iter()
			.find(|obj| obj.id() == id)
			.map(|obj| obj.as_ref())
	}

	/// IDでオブジェクトを可変参照で取得
	pub fn get_mut(&mut self, id: SceneObjectId) -> Option<&mut Box<dyn SceneObjectData>> {
		self.objects.iter_mut().find(|obj| obj.id() == id)
	}

	/// すべてのオブジェクトを取得
	pub fn all_objects(&self) -> &[Box<dyn SceneObjectData>] {
		&self.objects
	}

	/// オブジェクトを削除
	pub fn remove(&mut self, id: SceneObjectId) -> bool {
		if let Some(pos) = self.objects.iter().position(|obj| obj.id() == id) {
			self.objects.remove(pos);
			true
		} else {
			false
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
