use core::{AssetId, CompositionId, Node, NodeId};
use std::{collections::HashMap, hash::Hash};

use crate::{
	asset::{Asset, AssetType},
	composition::Composition,
};

/// Project is the top-level container that holds all compositions, assets, and nodes in a project.
#[derive(Debug)]
pub struct Project {
	/// Name of the project
	pub name: String,

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
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			compositions: HashMap::new(),
			assets: Vec::new(),
			nodes: Vec::new(),
		}
	}

	//==== Getter ==============================================================
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
	pub fn create_asset(&mut self, name: String, kind: AssetType) -> AssetId {
		let id = AssetId::new(self.assets.len());
		let asset = Asset::new(id, name, kind, None);

		self.assets.push(asset);

		id
	}

	/// Adds a new composition to the project with the given name and returns its ID
	pub fn create_composition(&mut self, name: impl Into<String>) -> CompositionId {
		let id = CompositionId::new(self.compositions.len());
		let composition = Composition::new(id, name);

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
	pub fn asset(&self, id: &AssetId) -> Option<&Asset> {
		self.assets.iter().find(|asset| asset.id() == *id)
	}

	/// Returns a mutable reference to the asset with the given ID.
	pub fn asset_mut(&mut self, id: &AssetId) -> Option<&mut Asset> {
		self.assets.iter_mut().find(|asset| asset.id() == *id)
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
}

impl Default for Project {
	fn default() -> Self {
		Self::new("Untitled Project")
	}
}
