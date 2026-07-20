use core::{AssetId, BitDepth, CompositionId, NodeId, SampleRate};
use std::collections::HashMap;

use crate::{
	asset::{Asset, AssetType},
	composition::Composition,
	node::Node,
};

/// Project is the top-level container that holds all compositions, assets, and nodes in a project.
#[derive(Debug)]
pub struct Project {
	/// Name of the project
	name: String,

	/// File path of the project. None if the project is unsaved.
	path: Option<std::path::PathBuf>,

	bitdepth: BitDepth,

	sample_rate: SampleRate,

	/// Vector of compositions in the project
	compositions: HashMap<CompositionId, Composition>,

	/// Vector of assets in the project
	assets: Vec<Asset>,

	/// Vector of nodes in the project
	nodes: Vec<Node>,
}

impl Project {
	//==== Constructor =========================================================
	/// Creates a new project with the given name
	pub fn new(name: impl Into<String>, bitdepth: BitDepth, sample_rate: SampleRate) -> Self {
		Self {
			name: name.into(),
			path: None,
			bitdepth,
			sample_rate,
			compositions: HashMap::new(),
			assets: Vec::new(),
			nodes: Vec::new(),
		}
	}

	//==== Getter ==============================================================
	/// Returns a reference to the name of the project
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Returns a reference to the file path of the project. None if the project is unsaved.
	pub fn path(&self) -> Option<&std::path::PathBuf> {
		self.path.as_ref()
	}

	/// Returns the bit depth of the project
	pub fn bitdepth(&self) -> BitDepth {
		self.bitdepth
	}

	/// Returns the sample rate of the project
	pub fn sample_rate(&self) -> SampleRate {
		self.sample_rate
	}

	/// Returns a reference to the list of compositions in the project
	pub fn compositions(&self) -> Vec<CompositionId> {
		self.compositions.values().map(|comp| comp.id()).collect()
	}

	/// Returns a references to the list of assets in the project
	pub fn assets(&self) -> Vec<AssetId> {
		self.assets.iter().map(|asset| asset.id()).collect()
	}

	/// Returns a reference to the list of nodes in the project
	pub fn nodes(&self) -> Vec<NodeId> {
		self.nodes.iter().map(|node| node.id()).collect()
	}

	//==== Factory Method ======================================================
	/// Adds a new asset to the project with the given name and type, and returns its ID
	pub fn create_asset(&mut self, name: impl Into<String>, kind: AssetType) -> AssetId {
		let id = AssetId::new(self.assets.len());
		let asset = Asset::new(id, name.into(), kind, None);

		self.assets.push(asset);

		id
	}

	/// Adds a new composition to the project with the given description and returns its ID
	pub fn create_composition(&mut self, name: impl Into<String>, width: u32, height: u32, fps: f32) -> CompositionId {
		let id = CompositionId::new(self.compositions.len());
		let composition = Composition::new(id, name, width, height, fps);

		self.compositions.insert(id, composition);

		id
	}

	/// Adds a new node to the project and returns its ID
	pub fn create_node(&mut self) -> NodeId {
		let id = NodeId::new(self.nodes.len());
		let node = Node::new(id);

		self.nodes.push(node);

		id
	}

	//==== Find Method =========================================================
	/// Returns a reference to the asset with the given ID
	pub fn asset(&self, id: AssetId) -> Option<&Asset> {
		self.assets.iter().find(|asset| asset.id() == id)
	}

	/// Returns a mutable reference to the asset with the given ID.
	pub fn asset_mut(&mut self, id: AssetId) -> Option<&mut Asset> {
		self.assets.iter_mut().find(|asset| asset.id() == id)
	}

	/// Returns a reference to the composition with the given ID.
	/// None if it does not exist.
	pub fn composition(&self, id: &CompositionId) -> Option<&Composition> {
		self.compositions.values().find(|comp| comp.id() == *id)
	}

	/// Returns a mutable reference to the composition with the given ID
	/// None if it ID does not exist.
	pub fn composition_mut(&mut self, id: &CompositionId) -> Option<&mut Composition> {
		self.compositions.values_mut().find(|comp| comp.id() == *id)
	}

	/// Returns a reference to the node with the given ID.
	/// None if it does not exist.
	pub fn node(&self, id: &NodeId) -> Option<&Node> {
		self.nodes.iter().find(|node| node.id() == *id)
	}

	/// Returns a mutable reference to the node with the given ID.
	/// None if it does not exist.
	pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
		self.nodes.iter_mut().find(|node| node.id() == *id)
	}

	//==== Update Method =======================================================
	pub fn save(&self) -> Result<(), std::io::Error> {
		Ok(())
	}

	/// Saves the project to the given file path. If the project is already saved, it updates the file path.
	pub fn save_as(&mut self, path: impl Into<std::path::PathBuf>) -> Result<(), std::io::Error> {
		//TODO: Implement save logic here

		self.path = Some(path.into());
		Ok(())
	}
}

impl Default for Project {
	fn default() -> Self {
		Self::new("Untitled Project", BitDepth::I8, SampleRate::Hz48000)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_create_project() {
		let bitdepth_case = BitDepth::I8;
		let sample_rate_case = SampleRate::Hz48000;

		let project = Project::new("Test Project", bitdepth_case, sample_rate_case);

		assert_eq!(project.name, "Test Project");
		assert_eq!(project.bitdepth, bitdepth_case);
		assert_eq!(project.sample_rate, sample_rate_case);
		assert!(project.path.is_none());
		assert!(project.compositions.is_empty());
		assert!(project.assets.is_empty());
		assert!(project.nodes.is_empty());
	}

	#[test]
	fn test_create_asset() {
		let mut project = Project::default();
		let asset_id = project.create_asset("Test Asset", AssetType::Video);

		assert_eq!(asset_id, AssetId::new(0));
		assert_eq!(project.assets.len(), 1);
		project.asset(asset_id).expect("");
		assert_eq!(project.asset(asset_id).unwrap().id(), asset_id);
		assert_eq!(project.asset(asset_id).unwrap().name, "Test Asset");
		assert_eq!(project.asset(asset_id).unwrap().kind(), AssetType::Video);
	}
}
