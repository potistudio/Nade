//! View model of the asset browser.
//!
//! The widget only renders [`AssetBrowserView`]; it never touches [`Project`]
//! or the tree storage. Flattening and filtering happen here, in a single
//! cached-index pass, so the rules stay testable without building elements.

use crate::AssetBrowserState;
use nade_core::AssetId;
use nade_domain::{Asset, AssetType, Project};
use std::collections::{HashMap, HashSet};

/// One visible outline row.
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
}

/// Everything the asset browser needs to render one frame.
#[derive(Debug)]
pub struct AssetBrowserView<'a> {
	/// Raw search text (the text input value, spacing included).
	pub filter: &'a str,
	pub rows: Vec<AssetRow<'a>>,
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
		let roots = || project.iter_assets().filter(|asset| asset.parent.is_none());

		let mut rows = Vec::new();
		if filter.is_empty() {
			for root in roots() {
				self.push_rows(root, &index, None, 0, &mut rows);
			}
		} else {
			let mut matched = HashSet::new();
			for root in roots() {
				collect_matches(root, &index, &filter, &mut matched);
			}
			for root in roots() {
				self.push_rows(root, &index, Some(&matched), 0, &mut rows);
			}
		}

		// フィルタで隠れた選択は「押せるのに何も起きない」ので無効扱いにする
		let can_add_to_timeline = rows
			.iter()
			.find(|row| row.selected)
			.is_some_and(|row| row.kind.is_media());

		AssetBrowserView {
			filter: &self.filter,
			rows,
			can_add_to_timeline,
		}
	}
}

// -------- Internal -----------------------------------------------------------
impl AssetBrowserState {
	/// Appends `asset` and, when open, its visible descendants in tree order.
	fn push_rows<'a>(
		&self,
		asset: &'a Asset,
		index: &HashMap<AssetId, &'a Asset>,
		only: Option<&HashSet<AssetId>>,
		depth: usize,
		rows: &mut Vec<AssetRow<'a>>,
	) {
		let id = asset.id();
		// 検索中は一致した行だけを並べる
		if only.is_some_and(|only| !only.contains(&id)) {
			return;
		}

		let has_children = !asset.children.is_empty();
		// 検索中は一致を見せるためフォルダを開いた状態で表示する
		let expanded = has_children && (only.is_some() || self.expanded_ids.contains(&id));

		rows.push(AssetRow {
			id,
			depth,
			label: &asset.name,
			kind: asset.kind(),
			has_children,
			expanded,
			selected: self.selected_id == Some(id),
		});

		if !expanded {
			return;
		}
		for child in asset.children.iter().filter_map(|child| index.get(child).copied()) {
			self.push_rows(child, index, only, depth + 1, rows);
		}
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
		view.rows.iter().map(|row| row.label).collect()
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
		assert!(view.rows[0].has_children);
		assert!(!view.rows[0].expanded);
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
		assert_eq!(view.rows[1].depth, 1);
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
		assert!(view.rows[0].expanded);
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
