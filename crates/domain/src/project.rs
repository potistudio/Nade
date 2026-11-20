use core::{AssetId, BitDepth, CompositionId, NodeId, SampleRate};
use std::collections::HashMap;
use std::path::Path;

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

	/// Returns IDs of all assets in the project (flat).
	pub fn assets(&self) -> Vec<AssetId> {
		self.assets.iter().map(|asset| asset.id()).collect()
	}

	/// Returns IDs of root assets (no parent) for the project tree.
	pub fn root_assets(&self) -> Vec<AssetId> {
		self.assets
			.iter()
			.filter(|asset| asset.parent.is_none())
			.map(|asset| asset.id())
			.collect()
	}

	/// Returns a reference to the list of nodes in the project
	pub fn nodes(&self) -> Vec<NodeId> {
		self.nodes.iter().map(|node| node.id()).collect()
	}

	//==== Factory Method ======================================================
	/// Adds a new root asset to the project with the given name and type.
	pub fn create_asset(&mut self, name: impl Into<String>, kind: AssetType) -> AssetId {
		self.create_asset_in(None, name, kind)
	}

	/// Adds a new asset under an optional parent folder.
	pub fn create_asset_in(&mut self, parent: Option<AssetId>, name: impl Into<String>, kind: AssetType) -> AssetId {
		let id = AssetId::new(self.assets.len());
		self.assets.push(Asset::new(id, name.into(), kind, parent));

		if let Some(parent_id) = parent
			&& let Some(parent_asset) = self.assets.iter_mut().find(|asset| asset.id() == parent_id)
		{
			parent_asset.children.push(id);
		}

		id
	}

	/// Creates a folder asset under an optional parent.
	pub fn create_folder(&mut self, name: impl Into<String>, parent: Option<AssetId>) -> AssetId {
		self.create_asset_in(parent, name, AssetType::Folder)
	}

	/// Imports a media file into the project tree.
	///
	/// Returns `None` when the extension is not a supported media type.
	pub fn import_media(&mut self, path: impl AsRef<Path>, parent: Option<AssetId>) -> Option<AssetId> {
		let path = path.as_ref();
		let kind = AssetType::from_path(path)?;
		let name = path.file_name()?.to_string_lossy().into_owned();
		let id = self.create_asset_in(parent, name, kind);
		if let Some(asset) = self.asset_mut(id) {
			asset.source_path = Some(path.to_path_buf());
		}
		Some(id)
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

	//==== History =============================================================
	/// Copy of every composition, used for undo / redo.
	pub fn snapshot(&self) -> ProjectSnapshot {
		ProjectSnapshot(self.compositions.clone())
	}

	/// Restore compositions captured by [`Project::snapshot`].
	pub fn restore(&mut self, snapshot: ProjectSnapshot) {
		self.compositions = snapshot.0;
	}

	//==== Update Method =======================================================
	/// Writes the project to its file. Fails when the project was never saved.
	pub fn save(&self) -> Result<(), std::io::Error> {
		let path = self
			.path
			.as_ref()
			.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "project has no file yet"))?;
		self.write_to(path)
	}

	/// Saves the project to the given file path and remembers it. The project
	/// takes the file name (without extension) as its name.
	pub fn save_as(&mut self, path: impl Into<std::path::PathBuf>) -> Result<(), std::io::Error> {
		let path = path.into();
		if let Some(stem) = path.file_stem() {
			self.name = stem.to_string_lossy().into_owned();
		}
		self.write_to(&path)?;
		self.path = Some(path);
		Ok(())
	}

	fn write_to(&self, path: &Path) -> Result<(), std::io::Error> {
		let mut compositions: Vec<&Composition> = self.compositions.values().collect();
		compositions.sort_by_key(|comp| comp.id());
		let file = ProjectFileRef {
			format: PROJECT_FORMAT,
			name: &self.name,
			bitdepth: self.bitdepth,
			sample_rate: self.sample_rate,
			compositions,
			assets: &self.assets,
			nodes: self.nodes.len(),
		};
		let json = serde_json::to_vec_pretty(&file).map_err(std::io::Error::other)?;
		// 書き込み途中で落ちても元のファイルを壊さないよう、一時ファイルから置き換える
		let temporary = path.with_extension("nade-tmp");
		std::fs::write(&temporary, json)?;
		std::fs::rename(&temporary, path)
	}

	/// Reads a project written by [`Project::save_as`].
	pub fn open(path: impl Into<std::path::PathBuf>) -> Result<Self, std::io::Error> {
		let path = path.into();
		let bytes = std::fs::read(&path)?;
		let file: ProjectFile = serde_json::from_slice(&bytes)
			.map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
		if file.format > PROJECT_FORMAT {
			return Err(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				format!("project format {} is newer than this version of Nade", file.format),
			));
		}
		Ok(Self {
			name: file.name,
			path: Some(path),
			bitdepth: file.bitdepth,
			sample_rate: file.sample_rate,
			compositions: file
				.compositions
				.into_iter()
				.map(|composition| (composition.id(), composition))
				.collect(),
			assets: file.assets,
			nodes: (0..file.nodes).map(|index| Node::new(NodeId::new(index))).collect(),
		})
	}
}

/// Version of the `.nade` file layout.
const PROJECT_FORMAT: u32 = 1;

#[derive(serde::Serialize)]
struct ProjectFileRef<'a> {
	format: u32,
	name: &'a str,
	bitdepth: BitDepth,
	sample_rate: SampleRate,
	compositions: Vec<&'a Composition>,
	assets: &'a [Asset],
	nodes: usize,
}

#[derive(serde::Deserialize)]
struct ProjectFile {
	format: u32,
	name: String,
	bitdepth: BitDepth,
	sample_rate: SampleRate,
	compositions: Vec<Composition>,
	assets: Vec<Asset>,
	nodes: usize,
}

/// Editable state of a project at one point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSnapshot(HashMap<CompositionId, Composition>);

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

	#[test]
	fn create_asset_in_builds_hierarchy() {
		let mut project = Project::default();
		let folder = project.create_folder("Assets", None);
		let child = project.create_asset_in(Some(folder), "clip.mov", AssetType::Video);

		assert_eq!(project.root_assets(), vec![folder]);
		assert_eq!(project.asset(folder).unwrap().children, vec![child]);
		assert_eq!(project.asset(child).unwrap().parent, Some(folder));
	}

	#[test]
	fn snapshot_restores_compositions() {
		let mut project = Project::default();
		let comp = project.create_composition("Main", 1920, 1080, 30.0);
		let before = project.snapshot();
		let node = project.create_node();
		project.composition_mut(&comp).unwrap().add_instance(node);
		assert_ne!(project.snapshot(), before);

		project.restore(before.clone());
		assert_eq!(project.snapshot(), before);
		assert!(project.composition(&comp).unwrap().is_empty());
	}

	#[test]
	fn save_and_open_round_trip() {
		let dir = std::env::temp_dir().join(format!("nade-project-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("Round Trip.nade");

		let mut project = Project::default();
		let folder = project.create_folder("Assets", None);
		let clip = project.import_media("/tmp/clip.mp4", Some(folder)).unwrap();
		project.asset_mut(clip).unwrap().media_info = Some(core::MediaInfo::still(4, 4));
		let comp = project.create_composition("Main", 1280, 720, 24.0);
		let node = project.create_node();
		{
			let composition = project.composition_mut(&comp).unwrap();
			composition.set_background([0.1, 0.2, 0.3, 1.0]);
			let layer = composition.add_instance_named(node, "Title");
			layer.set_content(crate::InstanceContent::text("Hello"));
			layer
				.animation
				.curve_mut(crate::TransformProperty::Opacity)
				.add_key(1.0, 0.5);
			layer.source_offset = 0.25;
		}
		assert!(project.save().is_err(), "unsaved projects need a path");
		project.save_as(&path).unwrap();
		assert_eq!(project.name(), "Round Trip");

		let opened = Project::open(&path).unwrap();
		assert_eq!(opened.snapshot(), project.snapshot());
		assert_eq!(opened.name(), "Round Trip");
		assert_eq!(opened.path(), Some(&path));
		assert_eq!(opened.asset(clip), project.asset(clip));
		assert_eq!(opened.nodes().len(), 1);
		assert!(Project::open(dir.join("missing.nade")).is_err());
	}

	#[test]
	fn import_media_sets_source_path() {
		let mut project = Project::default();
		let folder = project.create_folder("Assets", None);
		let id = project
			.import_media("/tmp/demo.png", Some(folder))
			.expect("png is supported");

		let asset = project.asset(id).unwrap();
		assert_eq!(asset.kind(), AssetType::Image);
		assert_eq!(asset.name, "demo.png");
		assert_eq!(
			asset.source_path.as_deref(),
			Some(std::path::Path::new("/tmp/demo.png"))
		);
	}
}
