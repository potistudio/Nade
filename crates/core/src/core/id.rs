#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
	pub fn new(id: usize) -> Self {
		Self(id)
	}

	pub fn value(&self) -> usize {
		self.0
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(usize);

impl AssetId {
	pub fn new(id: usize) -> Self {
		Self(id)
	}

	pub fn value(&self) -> usize {
		self.0
	}
}
