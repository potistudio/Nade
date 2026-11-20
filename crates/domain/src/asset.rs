use nade_core::{AssetId, MediaInfo};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum AssetType {
	Image,
	Video,
	Audio,
	Composition,
	Folder,
}

impl AssetType {
	/// Infer a media asset type from a file path extension.
	pub fn from_path(path: &Path) -> Option<Self> {
		match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
			"png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff" => Some(Self::Image),
			"mov" | "mp4" | "mkv" | "avi" | "webm" | "m4v" => Some(Self::Video),
			"mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => Some(Self::Audio),
			_ => None,
		}
	}

	pub fn is_media(self) -> bool {
		matches!(self, Self::Image | Self::Video | Self::Audio)
	}
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Asset {
	id: AssetId,
	kind: AssetType,

	pub name: String,
	pub parent: Option<AssetId>,
	pub children: Vec<AssetId>,
	/// Source media path when this asset was imported from disk.
	pub source_path: Option<PathBuf>,
	/// Probed stream facts (set once the file has been inspected).
	pub media_info: Option<MediaInfo>,
}

impl Asset {
	pub fn new(id: AssetId, name: String, kind: AssetType, parent: Option<AssetId>) -> Self {
		Self {
			id,
			name,
			kind,
			parent,
			children: Vec::new(),
			source_path: None,
			media_info: None,
		}
	}

	pub fn id(&self) -> AssetId {
		self.id
	}

	pub fn kind(&self) -> AssetType {
		self.kind
	}
}
