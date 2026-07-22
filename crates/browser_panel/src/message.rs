use core::AssetId;

/// Messages that the project pane can send.
#[derive(Debug, Clone)]
pub enum ProjectPaneMessage {
	ToggleExpand(AssetId),
	Select(AssetId),
	ClearSelection,
	OpenItem(AssetId),
	/// Open a file dialog and import the chosen media file.
	ImportMedia,
	/// Create a new folder under the selected folder (or at root).
	NewFolder,
	/// Add the selected media asset to the first timeline track.
	AddToTimeline,
}
