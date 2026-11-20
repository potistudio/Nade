//! View model of the asset browser.
//!
//! The widget only renders [`AssetBrowserView`]; it never touches [`Project`]
//! or the tree storage. Flattening, filtering and sibling sorting happen here,
//! in a single cached-index pass, so the rules stay testable without building
//! elements.

use crate::{AssetBrowserState, SortOrder};
use nade_core::AssetId;
use nade_domain::{Asset, AssetType, Project};
use std::collections::{HashMap, HashSet};

/// One visible outline row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutlineRow<'a> {
	Asset(AssetRow<'a>),
	/// Placeholder for a file whose media is still being probed.
	Importing(ImportingRow<'a>),
}

impl<'a> OutlineRow<'a> {
	pub fn label(&self) -> &'a str {
		match self {
			Self::Asset(row) => row.label,
			Self::Importing(row) => row.label,
		}
	}

	pub fn asset(&self) -> Option<&AssetRow<'a>> {
		match self {
			Self::Asset(row) => Some(row),
			Self::Importing(_) => None,
		}
	}
}

/// A row placeholder shown at the destination while a file is imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportingRow<'a> {
	pub depth: usize,
	pub label: &'a str,
	pub kind: AssetType,
}

/// Where a file dropped now would land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropTarget<'a> {
	/// Destination folder name; `None` is the project root.
	pub folder: Option<&'a str>,
}

/// An asset row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRow<'a> {
	pub id: AssetId,
	/// Nesting level, used for the indent.
	pub depth: usize,
	pub label: &'a str,
	pub kind: AssetType,
	/// Structural children exist, so the disclosure chevron is shown.
	pub has_children: bool,
	/// Children are currently listed under this row.
	pub expanded: bool,
	pub selected: bool,
	/// Draft text while this row's name is being edited in place.
	pub rename: Option<&'a str>,
}

/// Everything the asset browser needs to render one frame.
#[derive(Debug)]
pub struct AssetBrowserView<'a> {
	/// Raw search text (the text input value, spacing included).
	pub filter: &'a str,
	pub rows: Vec<OutlineRow<'a>>,
	/// Shown while importable files are dragged over the window.
	pub drop_target: Option<DropTarget<'a>>,
	/// Active column header and its direction indicator.
	pub sort: SortOrder,
	/// The selected, currently visible asset can be sent to the timeline.
	pub can_add_to_timeline: bool,
}

impl AssetBrowserView<'_> {
	pub fn is_searching(&self) -> bool {
		!self.filter.trim().is_empty()
	}
}

// -------- Public API ---------------------------------------------------------
impl AssetBrowserState {
	/// Derives the render model from the project tree and the panel state.
	///
	/// Visits every asset at most twice (match pass, flatten pass) through one
	/// ID index, instead of resolving children with a linear scan per row.
	pub fn view_model<'a>(&'a self, project: &'a Project) -> AssetBrowserView<'a> {
		let filter = self.filter.trim().to_lowercase();
		let index: HashMap<AssetId, &'a Asset> = project.iter_assets().map(|asset| (asset.id(), asset)).collect();
		let mut roots: Vec<&'a Asset> = project.iter_assets().filter(|asset| asset.parent.is_none()).collect();
		self.sort.sort(&mut roots);

		let mut rows = Vec::new();
		if filter.is_empty() {
			for root in roots {
				self.push_rows(root, &index, None, 0, &mut rows);
			}
			self.push_importing(None, 0, &mut rows);
		} else {
			let mut matched = HashSet::new();
			for &root in &roots {
				collect_matches(root, &index, &filter, &mut matched);
			}
			for root in roots {
				self.push_rows(root, &index, Some(&matched), 0, &mut rows);
			}
			self.push_importing(None, 0, &mut rows);
		}

		// フィルタで隠れた選択は「押せるのに何も起きない」ので無効扱いにする
		let can_add_to_timeline = rows
			.iter()
			.filter_map(OutlineRow::asset)
			.find(|row| row.selected)
			.is_some_and(|row| row.kind.is_media());

		let drop_target = self.drop_hover.then(|| DropTarget {
			folder: self
				.import_parent(project)
				.and_then(|id| index.get(&id))
				.map(|folder| folder.name.as_str()),
		});

		AssetBrowserView {
			filter: &self.filter,
			rows,
			drop_target,
			sort: self.sort,
			can_add_to_timeline,
		}
	}
}

// -------- Internal -----------------------------------------------------------
impl AssetBrowserState {
	/// Appends `asset` and, when open, its visible descendants in sort order.
	fn push_rows<'a>(
		&'a self,
		asset: &'a Asset,
		index: &HashMap<AssetId, &'a Asset>,
		only: Option<&HashSet<AssetId>>,
		depth: usize,
		rows: &mut Vec<OutlineRow<'a>>,
	) {
		let id = asset.id();
		// 検索中は一致した行だけを並べる
		if only.is_some_and(|only| !only.contains(&id)) {
			return;
		}

		let has_children =
			!asset.children.is_empty() || self.importing.iter().any(|pending| pending.parent == Some(id));
		// 検索中は一致を見せるためフォルダを開いた状態で表示する
		let expanded = has_children && (only.is_some() || self.expanded_ids.contains(&id));

		rows.push(OutlineRow::Asset(AssetRow {
			id,
			depth,
			label: &asset.name,
			kind: asset.kind(),
			has_children,
			expanded,
			selected: self.selected_id == Some(id),
			rename: self
				.rename
				.as_ref()
				.filter(|draft| draft.id == id)
				.map(|draft| draft.text.as_str()),
		}));

		if !expanded {
			return;
		}
		// 並べ替えは開いているフォルダの子だけ、表示のたびに行う
		let mut children: Vec<&'a Asset> = asset
			.children
			.iter()
			.filter_map(|child| index.get(child).copied())
			.collect();
		self.sort.sort(&mut children);
		for child in children {
			self.push_rows(child, index, only, depth + 1, rows);
		}
		self.push_importing(Some(id), depth + 1, rows);
	}

	/// Appends the placeholders bound for `parent` after its real children.
	fn push_importing<'a>(&'a self, parent: Option<AssetId>, depth: usize, rows: &mut Vec<OutlineRow<'a>>) {
		rows.extend(
			self.importing
				.iter()
				.filter(|pending| pending.parent == parent)
				.map(|pending| {
					OutlineRow::Importing(ImportingRow {
						depth,
						label: &pending.name,
						kind: pending.kind,
					})
				}),
		);
	}
}

/// Marks every asset that matches, or has a matching descendant, so folders
/// stay visible as context.
fn collect_matches(
	asset: &Asset,
	index: &HashMap<AssetId, &Asset>,
	filter: &str,
	matched: &mut HashSet<AssetId>,
) -> bool {
	// 名前の小文字化はノードごとに一度だけ
	let mut any = asset.name.to_lowercase().contains(filter);
	for child in asset.children.iter().filter_map(|child| index.get(child).copied()) {
		any |= collect_matches(child, index, filter, matched);
	}
	if any {
		matched.insert(asset.id());
	}
	any
}

#[cfg(test)]
mod tests {
	use super::*;
	use nade_core::{BitDepth, SampleRate};

	fn project() -> Project {
		Project::new("Test", BitDepth::F32, SampleRate::Hz48000)
	}

	fn labels<'a>(view: &AssetBrowserView<'a>) -> Vec<&'a str> {
		view.rows.iter().map(OutlineRow::label).collect()
	}

	fn asset_row<'v, 'a>(view: &'v AssetBrowserView<'a>, index: usize) -> &'v AssetRow<'a> {
		view.rows[index].asset().expect("asset row")
	}

	/// A collapsed folder lists only itself, even though it has children.
	#[test]
	fn collapsed_folders_list_only_their_row() {
		let mut project = project();
		let folder = project.create_folder("Footage", None);
		project.create_asset_in(Some(folder), "clip.mov", AssetType::Video);
		let state = AssetBrowserState::default();

		let view = state.view_model(&project);

		assert_eq!(labels(&view), ["Footage"]);
		assert!(asset_row(&view, 0).has_children);
		assert!(!asset_row(&view, 0).expanded);
	}

	/// Expanding a folder appends its children one level deeper.
	#[test]
	fn expanded_folders_list_children_with_depth() {
		let mut project = project();
		let folder = project.create_folder("Footage", None);
		project.create_asset_in(Some(folder), "clip.mov", AssetType::Video);
		let mut state = AssetBrowserState::default();
		state.expanded_ids.insert(folder);

		let view = state.view_model(&project);

		assert_eq!(labels(&view), ["Footage", "clip.mov"]);
		assert_eq!(asset_row(&view, 1).depth, 1);
	}

	/// A matching descendant keeps its folders visible and opens them, without
	/// touching the persisted expansion state.
	#[test]
	fn search_opens_matching_folders_without_persisting_it() {
		let mut project = project();
		let folder = project.create_folder("Footage", None);
		project.create_asset_in(Some(folder), "clip.mov", AssetType::Video);
		let state = AssetBrowserState {
			filter: "clip".into(),
			..Default::default()
		};

		let view = state.view_model(&project);

		assert_eq!(labels(&view), ["Footage", "clip.mov"]);
		assert!(asset_row(&view, 0).expanded);
		assert!(state.expanded_ids.is_empty());
		assert!(view.is_searching());
	}

	/// Rows whose name and descendants all miss the filter disappear.
	#[test]
	fn search_hides_non_matching_rows() {
		let mut project = project();
		project.create_asset("clip.mov", AssetType::Video);
		project.create_asset("score.wav", AssetType::Audio);
		let state = AssetBrowserState {
			filter: "  SCORE ".into(),
			..Default::default()
		};

		let view = state.view_model(&project);

		assert_eq!(labels(&view), ["score.wav"]);
	}

	/// Siblings follow the sort order at every level, folders first.
	#[test]
	fn rows_follow_the_sort_order() {
		let mut project = project();
		project.create_asset("b.wav", AssetType::Audio);
		let folder = project.create_folder("Footage", None);
		project.create_asset_in(Some(folder), "clip10.mov", AssetType::Video);
		project.create_asset_in(Some(folder), "clip2.mov", AssetType::Video);
		project.create_asset("a.png", AssetType::Image);
		let mut state = AssetBrowserState::default();
		state.expanded_ids.insert(folder);

		let view = state.view_model(&project);
		assert_eq!(labels(&view), ["Footage", "clip2.mov", "clip10.mov", "a.png", "b.wav"]);

		state.sort = state.sort.toggled(crate::SortKey::Name);
		let view = state.view_model(&project);
		assert_eq!(labels(&view), ["Footage", "clip10.mov", "clip2.mov", "b.wav", "a.png"]);
	}

	/// Only the row being renamed carries the draft text.
	#[test]
	fn rename_draft_lands_on_its_row() {
		let mut project = project();
		let clip = project.create_asset("clip.mov", AssetType::Video);
		project.create_asset("score.wav", AssetType::Audio);
		let state = AssetBrowserState {
			rename: Some(crate::RenameDraft {
				id: clip,
				text: "intro".into(),
			}),
			..Default::default()
		};

		let view = state.view_model(&project);

		let drafts: Vec<_> = view
			.rows
			.iter()
			.filter_map(OutlineRow::asset)
			.map(|row| row.rename)
			.collect();
		assert_eq!(drafts, [Some("intro"), None]);
	}

	/// Placeholders sit after the real children of their destination folder,
	/// which shows a chevron even while it has no assets yet.
	#[test]
	fn importing_rows_appear_at_their_destination() {
		let mut project = project();
		let folder = project.create_folder("Footage", None);
		let mut state = AssetBrowserState {
			selected_id: Some(folder),
			..Default::default()
		};
		state.begin_import(
			&project,
			std::path::Path::new("/tmp/clip.mov"),
			std::time::Instant::now(),
		);
		state.selected_id = None;
		state.begin_import(
			&project,
			std::path::Path::new("/tmp/score.wav"),
			std::time::Instant::now(),
		);

		let view = state.view_model(&project);

		assert_eq!(labels(&view), ["Footage", "clip", "score"]);
		assert!(asset_row(&view, 0).has_children);
		assert_eq!(
			view.rows[1],
			OutlineRow::Importing(ImportingRow {
				depth: 1,
				label: "clip",
				kind: AssetType::Video,
			})
		);
	}

	/// Hovering files names the folder they would land in.
	#[test]
	fn drop_target_names_the_destination() {
		let mut project = project();
		let folder = project.create_folder("Footage", None);
		let mut state = AssetBrowserState::default();

		assert_eq!(state.view_model(&project).drop_target, None);

		state.drop_hover = true;
		assert_eq!(
			state.view_model(&project).drop_target,
			Some(DropTarget { folder: None })
		);

		state.selected_id = Some(folder);
		assert_eq!(
			state.view_model(&project).drop_target,
			Some(DropTarget {
				folder: Some("Footage"),
			})
		);
	}

	/// Only a selected media asset that is on screen can go to the timeline.
	#[test]
	fn add_to_timeline_needs_a_visible_media_selection() {
		let mut project = project();
		let folder = project.create_folder("Footage", None);
		let clip = project.create_asset_in(Some(folder), "clip.mov", AssetType::Video);
		let mut state = AssetBrowserState::default();

		assert!(!state.view_model(&project).can_add_to_timeline);

		state.selected_id = Some(folder);
		assert!(!state.view_model(&project).can_add_to_timeline);

		state.selected_id = Some(clip);
		// 折りたたまれたフォルダの中身は見えていないので追加できない
		assert!(!state.view_model(&project).can_add_to_timeline);

		state.expanded_ids.insert(folder);
		assert!(state.view_model(&project).can_add_to_timeline);

		state.filter = "score".into();
		assert!(!state.view_model(&project).can_add_to_timeline);
	}
}
