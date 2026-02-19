use crate::AssetId;
use crate::asset::{Asset, AssetType};
use crate::composition::Composition;

#[derive(Debug, Default)]
pub struct Project {
	compositions: Vec<Composition>,
	assets: Vec<Asset>,
}

impl Project {
	pub fn add_asset(&mut self, name: String, kind: AssetType) -> AssetId {
		let id = AssetId::new(self.assets.len());
		let asset = Asset::new(id, name, kind, None);

		self.assets.push(asset);

		id
	}

	pub fn assets(&self) -> Vec<AssetId> {
		self.assets.iter().map(|asset| asset.id()).collect()
	}

	pub fn get_asset(&self, id: &AssetId) -> Option<&Asset> {
		self.assets.iter().find(|asset| asset.id() == *id)
	}
}
