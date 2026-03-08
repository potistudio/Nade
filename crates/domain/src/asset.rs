use core::AssetId;

#[derive(Debug)]
pub enum AssetType {
	Image,
	Video,
	Audio,
	Composition,
	Folder,
}

#[derive(Debug)]
pub struct Asset {
	id: AssetId,
	kind: AssetType,

	pub name: String,
	pub parent: Option<AssetId>,
	pub children: Vec<AssetId>,
}

impl Asset {
	pub fn new(id: AssetId, name: String, kind: AssetType, parent: Option<AssetId>) -> Self {
		Self {
			id,
			name,
			kind,
			parent,
			children: Vec::new(),
		}
	}

	pub fn id(&self) -> AssetId {
		self.id
	}

	pub fn kind(&self) -> &AssetType {
		&self.kind
	}
}
