use crate::{AssetBrowserMessage, AssetBrowserRequest, SortOrder};
use nade_core::AssetId;
use nade_domain::{AssetType, Project};
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct AssetBrowserState {
	pub expanded_ids: HashSet<AssetId>,
	pub selected_id: Option<AssetId>,
	/// Case-insensitive name filter from the search field.
	pub filter: String,
	/// Sibling order of the outline (display only; the project keeps its own).
	pub sort: SortOrder,
}

// -------- Public API ---------------------------------------------------------
impl AssetBrowserState {
	/// Applies panel-local messages to the view state and returns the
	/// app-level work (if any) for the caller to perform.
	pub fn apply(&mut self, project: &Project, msg: AssetBrowserMessage) -> Option<AssetBrowserRequest> {
		match msg {
			AssetBrowserMessage::ToggleExpand(id) => {
				if self.expanded_ids.contains(&id) {
					self.expanded_ids.remove(&id);
				} else {
					self.expanded_ids.insert(id);
				}
				None
			}
			AssetBrowserMessage::Select(id) => {
				self.selected_id = Some(id);
				None
			}
			AssetBrowserMessage::ClearSelection => {
				self.selected_id = None;
				None
			}
			AssetBrowserMessage::OpenItem(id) => {
				self.selected_id = Some(id);
				if let Some(asset) = project.asset(id)
					&& asset.kind() == AssetType::Folder
				{
					self.expanded_ids.insert(id);
				}
				None
			}
			AssetBrowserMessage::FilterChanged(filter) => {
				self.filter = filter;
				None
			}
			AssetBrowserMessage::SortBy(key) => {
				self.sort = self.sort.toggled(key);
				None
			}
			AssetBrowserMessage::ImportMedia => Some(AssetBrowserRequest::ImportMedia),
			AssetBrowserMessage::NewFolder => Some(AssetBrowserRequest::NewFolder),
			AssetBrowserMessage::AddToTimeline => Some(AssetBrowserRequest::AddToTimeline),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{SortDirection, SortKey};
	use nade_core::{BitDepth, SampleRate};

	fn project() -> Project {
		Project::new("Test", BitDepth::F32, SampleRate::Hz48000)
	}

	/// Toggling disclosure twice collapses the folder again.
	#[test]
	fn toggle_expand_is_reversible() {
		let mut project = project();
		let id = project.create_folder("Folder", None);
		let mut state = AssetBrowserState::default();

		state.apply(&project, AssetBrowserMessage::ToggleExpand(id));
		assert!(state.expanded_ids.contains(&id));

		state.apply(&project, AssetBrowserMessage::ToggleExpand(id));
		assert!(!state.expanded_ids.contains(&id));
	}

	/// Selecting then clearing empties the selection.
	#[test]
	fn select_then_clear_empties_the_selection() {
		let mut project = project();
		let id = project.create_folder("Folder", None);
		let mut state = AssetBrowserState::default();

		state.apply(&project, AssetBrowserMessage::Select(id));
		assert_eq!(state.selected_id, Some(id));

		state.apply(&project, AssetBrowserMessage::ClearSelection);
		assert_eq!(state.selected_id, None);
	}

	/// Opening an item selects it; folders additionally expand.
	#[test]
	fn open_item_expands_folders() {
		let mut project = project();
		let folder = project.create_folder("Folder", None);
		let mut state = AssetBrowserState::default();

		state.apply(&project, AssetBrowserMessage::OpenItem(folder));
		assert_eq!(state.selected_id, Some(folder));
		assert!(state.expanded_ids.contains(&folder));
	}

	/// Search text lands in the filter verbatim.
	#[test]
	fn filter_changed_updates_the_filter() {
		let project = project();
		let mut state = AssetBrowserState::default();

		state.apply(&project, AssetBrowserMessage::FilterChanged("zoom".into()));
		assert_eq!(state.filter, "zoom");
	}

	/// Header clicks flip the active column and reset a new one to ascending.
	#[test]
	fn sort_by_toggles_the_sort_order() {
		let project = project();
		let mut state = AssetBrowserState::default();

		state.apply(&project, AssetBrowserMessage::SortBy(SortKey::Name));
		assert_eq!(state.sort.key, SortKey::Name);
		assert_eq!(state.sort.direction, SortDirection::Descending);

		state.apply(&project, AssetBrowserMessage::SortBy(SortKey::Kind));
		assert_eq!(state.sort.key, SortKey::Kind);
		assert_eq!(state.sort.direction, SortDirection::Ascending);
	}

	/// App-level messages come back as requests for the shell.
	#[test]
	fn app_level_messages_return_requests() {
		let project = project();
		let mut state = AssetBrowserState::default();

		assert_eq!(
			state.apply(&project, AssetBrowserMessage::ImportMedia),
			Some(AssetBrowserRequest::ImportMedia)
		);
		assert_eq!(
			state.apply(&project, AssetBrowserMessage::NewFolder),
			Some(AssetBrowserRequest::NewFolder)
		);
		assert_eq!(
			state.apply(&project, AssetBrowserMessage::AddToTimeline),
			Some(AssetBrowserRequest::AddToTimeline)
		);
	}
}
