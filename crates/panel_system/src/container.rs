//! エリア（Blender 風の単一エディタ領域）

/// レイアウト上の1エリア
#[derive(Debug, Clone)]
pub struct Area<C: Clone + std::fmt::Debug> {
	pub id: usize,
	pub content: C,
}

impl<C: Clone + std::fmt::Debug> Area<C> {
	pub fn new(id: usize, content: C) -> Self {
		Self { id, content }
	}
}
