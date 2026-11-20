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
	/// In-place name edit, while one is open.
	pub rename: Option<RenameDraft>,
}

/// Name being typed into a row's rename field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDraft {
	pub id: AssetId,
	pub text: String,
}

// -------- Public API ---------------------------------------------------------
impl AssetBrowserState {
	/// Applies panel-local messages to the view state and returns the
	/// app-level work for the caller to perform, in order.
	pub fn apply(&mut self, project: &Project, msg: AssetBrowserMessage) -> Vec<AssetBrowserRequest> {
		let mut requests = Vec::new();
		// 編集中に別の操作が来たら、Finder と同じく入力中の名前で確定してから進める
		if !msg.is_rename_edit() {
			requests.extend(self.commit_rename(project));
		}

		match msg {
			AssetBrowserMessage::ToggleExpand(id) => {
				if self.expanded_ids.contains(&id) {
					self.expanded_ids.remove(&id);
				} else {
					self.expanded_ids.insert(id);
				}
			}
			AssetBrowserMessage::Select(id) => self.selected_id = Some(id),
			AssetBrowserMessage::ClearSelection => self.selected_id = None,
			AssetBrowserMessage::OpenItem(id) => {
				self.selected_id = Some(id);
				if let Some(asset) = project.asset(id)
					&& asset.kind() == AssetType::Folder
				{
					self.expanded_ids.insert(id);
				}
			}
			AssetBrowserMessage::FilterChanged(filter) => self.filter = filter,
			AssetBrowserMessage::SortBy(key) => self.sort = self.sort.toggled(key),
			AssetBrowserMessage::StartRename => {
				if let Some(id) = self.renamable_id(project)
					&& let Some(asset) = project.asset(id)
				{
					self.rename = Some(RenameDraft {
						id,
						text: asset.name.clone(),
					});
					requests.push(AssetBrowserRequest::FocusRename);
				}
			}
			AssetBrowserMessage::RenameInput(text) => {
				if let Some(draft) = &mut self.rename {
					draft.text = text;
				}
			}
			AssetBrowserMessage::CommitRename => requests.extend(self.commit_rename(project)),
			AssetBrowserMessage::CancelRename => self.rename = None,
			AssetBrowserMessage::ImportMedia => requests.push(AssetBrowserRequest::ImportMedia),
			AssetBrowserMessage::NewFolder => requests.push(AssetBrowserRequest::NewFolder),
			AssetBrowserMessage::AddToTimeline => requests.push(AssetBrowserRequest::AddToTimeline),
		}
		requests
	}

	/// Closes the rename field. Returns the rename to apply when the name is
	/// non-empty after trimming and differs from the current one.
	///
	/// Also called by the shell when focus leaves the panel.
	pub fn commit_rename(&mut self, project: &Project) -> Option<AssetBrowserRequest> {
		let draft = self.rename.take()?;
		let name = draft.text.trim();
		let asset = project.asset(draft.id)?;
		(!name.is_empty() && name != asset.name).then(|| AssetBrowserRequest::Rename {
			id: draft.id,
			name: name.to_owned(),
		})
	}

	/// The selection, when it is on screen and can be renamed in place.
	pub fn renamable_id(&self, project: &Project) -> Option<AssetId> {
		let id = self.selected_id?;
		// 折りたたみや検索で隠れた行は編集欄を出せない
		self.view_model(project)
			.rows
			.iter()
			.any(|row| row.id == id)
			.then_some(id)
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
			[AssetBrowserRequest::ImportMedia]
		);
		assert_eq!(
			state.apply(&project, AssetBrowserMessage::NewFolder),
			[AssetBrowserRequest::NewFolder]
		);
		assert_eq!(
			state.apply(&project, AssetBrowserMessage::AddToTimeline),
			[AssetBrowserRequest::AddToTimeline]
		);
	}

	/// Starting a rename opens a draft of the current name and asks for focus.
	#[test]
	fn start_rename_opens_a_draft_of_the_selection() {
		let mut project = project();
		let id = project.create_asset("clip.mov", AssetType::Video);
		let mut state = AssetBrowserState::default();

		assert!(state.apply(&project, AssetBrowserMessage::StartRename).is_empty());
		assert_eq!(state.rename, None);

		state.selected_id = Some(id);
		assert_eq!(
			state.apply(&project, AssetBrowserMessage::StartRename),
			[AssetBrowserRequest::FocusRename]
		);
		assert_eq!(
			state.rename,
			Some(RenameDraft {
				id,
				text: "clip.mov".into(),
			})
		);
	}

	/// Rows hidden by a collapsed folder cannot be renamed in place.
	#[test]
	fn start_rename_ignores_hidden_selections() {
		let mut project = project();
		let folder = project.create_folder("Footage", None);
		let clip = project.create_asset_in(Some(folder), "clip.mov", AssetType::Video);
		let mut state = AssetBrowserState {
			selected_id: Some(clip),
			..Default::default()
		};

		assert!(state.apply(&project, AssetBrowserMessage::StartRename).is_empty());
		assert_eq!(state.rename, None);
	}

	/// Enter applies the trimmed name; blank or unchanged names do nothing.
	#[test]
	fn commit_rename_returns_only_real_changes() {
		let mut project = project();
		let id = project.create_asset("clip.mov", AssetType::Video);
		let mut state = AssetBrowserState {
			selected_id: Some(id),
			..Default::default()
		};
		let rename_to = |state: &mut AssetBrowserState, text: &str| {
			state.apply(&project, AssetBrowserMessage::StartRename);
			state.apply(&project, AssetBrowserMessage::RenameInput(text.into()));
			let requests = state.apply(&project, AssetBrowserMessage::CommitRename);
			assert_eq!(state.rename, None);
			requests
		};

		assert_eq!(
			rename_to(&mut state, "  intro.mov "),
			[AssetBrowserRequest::Rename {
				id,
				name: "intro.mov".into(),
			}]
		);
		assert!(rename_to(&mut state, "   ").is_empty());
		assert!(rename_to(&mut state, "clip.mov").is_empty());
	}

	/// Esc drops the draft without renaming.
	#[test]
	fn cancel_rename_keeps_the_old_name() {
		let mut project = project();
		let id = project.create_asset("clip.mov", AssetType::Video);
		let mut state = AssetBrowserState {
			selected_id: Some(id),
			..Default::default()
		};

		state.apply(&project, AssetBrowserMessage::StartRename);
		state.apply(&project, AssetBrowserMessage::RenameInput("intro.mov".into()));

		assert!(state.apply(&project, AssetBrowserMessage::CancelRename).is_empty());
		assert_eq!(state.rename, None);
	}

	/// Clicking elsewhere in the panel commits before handling the click.
	#[test]
	fn other_messages_commit_an_open_rename() {
		let mut project = project();
		let clip = project.create_asset("clip.mov", AssetType::Video);
		let other = project.create_asset("score.wav", AssetType::Audio);
		let mut state = AssetBrowserState {
			selected_id: Some(clip),
			..Default::default()
		};

		state.apply(&project, AssetBrowserMessage::StartRename);
		state.apply(&project, AssetBrowserMessage::RenameInput("intro.mov".into()));

		assert_eq!(
			state.apply(&project, AssetBrowserMessage::Select(other)),
			[AssetBrowserRequest::Rename {
				id: clip,
				name: "intro.mov".into(),
			}]
		);
		assert_eq!(state.rename, None);
		assert_eq!(state.selected_id, Some(other));
	}
}
