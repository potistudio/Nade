use core::CompositionId;
use std::collections::HashSet;

use crate::ObjectId;

#[derive(Debug, Clone, Default)]
pub struct ProjectPaneState {
	pub expanded_ids: HashSet<CompositionId>,
	pub selected_id: Option<ObjectId>,
}
