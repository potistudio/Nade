use core::AssetId;

/// Messages from the asset browser panel.
#[derive(Debug, Clone)]
pub enum AssetBrowserMessage {
	ToggleExpand(AssetId),
	Select(AssetId),
	ClearSelection,
	OpenItem(AssetId),
	/// Open a file dialog and import the chosen media file.
	ImportMedia,
	/// Create a new folder under the selected folder (or at root).
	NewFolder,
	/// Add the selected media asset to the timeline.
	AddToTimeline,
}
