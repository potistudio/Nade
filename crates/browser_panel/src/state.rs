use core::AssetId;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct ProjectPaneState {
	pub expanded_ids: HashSet<AssetId>,
	pub selected_id: Option<AssetId>,
}
