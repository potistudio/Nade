use crate::SortKey;
use nade_core::AssetId;

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

	/// Search field text changed.
	FilterChanged(String),

	/// Column header clicked: sort by it, or flip it when already active.
	SortBy(SortKey),

	/// Start editing the selected row's name in place.
	StartRename,

	/// Rename field text changed.
	RenameInput(String),

	/// Apply the edited name (Enter).
	CommitRename,

	/// Drop the edit and keep the old name (Esc).
	CancelRename,
}

impl AssetBrowserMessage {
	/// Messages that belong to an ongoing rename; anything else commits it
	/// first, as clicking away does in Finder.
	pub fn is_rename_edit(&self) -> bool {
		matches!(self, Self::RenameInput(_) | Self::CommitRename | Self::CancelRename)
	}
}

/// App-level work the panel state cannot perform itself.
///
/// `AssetBrowserState::apply` returns these for the shell to execute
/// (file dialogs, cross-panel edits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetBrowserRequest {
	/// Open a file dialog and import the chosen media files.
	ImportMedia,

	/// Create a new folder under the selected folder (or at root).
	NewFolder,

	/// Add the selected media asset to the timeline.
	AddToTimeline,

	/// Focus the rename field that just appeared and select its text.
	FocusRename,

	/// Give the asset a new (trimmed, non-empty, changed) name.
	Rename { id: AssetId, name: String },
}
