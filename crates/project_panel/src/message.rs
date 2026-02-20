use nade_core::core::AssetId;

/// Messsages that the project pane can send.
#[derive(Debug, Clone)]
pub enum ProjectPaneMessage {
	ToggleExpand(AssetId),
	Select(AssetId),
	ClearSelection,
	OpenItem(AssetId),
}
