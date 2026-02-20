use crate::{ProjectPaneMessage, ProjectPaneState};
use constants::style::*;
use iced::widget::container::Style;
use iced::widget::{column, container, mouse_area, row, text};
use iced::{Color, Element, Length};
use nade_core::Project;
use nade_core::{Asset, AssetId, AssetType};

#[derive(Debug)]
pub struct ProjectPaneWidget<'a> {
	project: &'a Project,
}

/// -------- Public API --------
impl<'a> ProjectPaneWidget<'a> {
	pub fn new(project: &'a Project) -> Self {
		Self { project }
	}

	pub fn view(self, state: &'a ProjectPaneState) -> Element<'a, ProjectPaneMessage> {
		let project = self.project;
		let root_assets = project.assets();
		let summary = summarize_items(project, &root_assets);

		let header = container(
			column![
				text("PROJECT HUB").size(22).color(TEXT_PRIMARY_COLOR),
				text("Assets and compositions")
					.size(12)
					.color(TEXT_PRIMARY_COLOR),
				row![
					info_pill(format!("{} Items", summary.total)),
					info_pill(format!("{} Folders", summary.folders)),
					info_pill(format!("{} Comps", summary.compositions)),
					info_pill(format!("{} Media", summary.media)),
				]
				.spacing(6)
			]
			.spacing(8),
		)
		.padding(8)
		.style(move |_| container::Style {
			background: Some(BACKGROUND_COLOR.into()),
			..Default::default()
		});

		let tree = column(root_assets.iter().filter_map(|id| {
			project
				.get_asset(id)
				.map(|asset| view_item(project, asset, state, 0))
		}))
		.spacing(4);

		let clear_area = mouse_area(container(text("")).width(Length::Fill).height(Length::Fill))
			.on_press(ProjectPaneMessage::ClearSelection);

		let tree_card = container(column![tree, clear_area].spacing(4))
			.width(Length::Fill)
			.height(Length::Fill)
			.padding(8)
			.style(move |_| container::Style {
				background: Some(BACKGROUND_COLOR.into()),
				..Default::default()
			});

		container(column![header, tree_card].spacing(10).height(Length::Fill))
			.width(Length::Fill)
			.height(Length::Fill)
			.padding(10)
			.style(move |_| container::Style {
				background: Some(BACKGROUND_COLOR.into()),
				..Default::default()
			})
			.into()
	}
}

// -------- Private API --------
fn summarize_items(project: &Project, items: &[AssetId]) -> ProjectSummary {
	let mut summary = ProjectSummary::default();

	fn walk(project: &Project, asset_id: &AssetId, summary: &mut ProjectSummary) {
		summary.total += 1;

		if let Some(asset) = project.get_asset(asset_id) {
			match asset.kind() {
				AssetType::Folder => summary.folders += 1,
				AssetType::Composition => summary.compositions += 1,
				AssetType::Image | AssetType::Video | AssetType::Audio => summary.media += 1,
			}

			for child in &asset.children {
				walk(project, child, summary);
			}
		}
	}

	for item in items {
		walk(project, item, &mut summary);
	}

	summary
}

fn view_item<'a>(
	project: &'a Project,
	asset: &'a Asset,
	state: &'a ProjectPaneState,
	depth: usize,
) -> Element<'a, ProjectPaneMessage> {
	let is_expanded = state.expanded_ids.contains(&asset.id());
	let is_selected = state.selected_id == Some(asset.id());
	let has_children = !asset.children.is_empty();
	let expander_symbol = if has_children {
		if is_expanded { "▾" } else { "▸" }
	} else {
		"·"
	};

	let kind_tag = match asset.kind() {
		AssetType::Folder => "DIR",
		AssetType::Composition => "COMP",
		AssetType::Image => "IMG",
		AssetType::Video => "VID",
		AssetType::Audio => "AUD",
	};

	let kind_hint = match asset.kind() {
		AssetType::Folder => "Folder",
		AssetType::Composition => "Composition",
		AssetType::Image => "Image",
		AssetType::Video => "Video",
		AssetType::Audio => "Audio",
	};

	let expander = container(text(expander_symbol).size(12).color(TEXT_PRIMARY_COLOR));
	let expander_element: Element<'a, ProjectPaneMessage> = if has_children {
		mouse_area(expander)
			.on_press(ProjectPaneMessage::ToggleExpand(asset.id()))
			.into()
	} else {
		expander.into()
	};

	let kind_badge = container(text(kind_tag).size(10).color(ACCENT_COLOR))
		.padding([2, 6])
		.style(move |_| container::Style {
			background: Some(
				Color {
					a: 0.16,
					..ACCENT_COLOR
				}
				.into(),
			),
			..Default::default()
		});

	let name_label = text(&asset.name).size(12).color(if is_selected {
		TEXT_PRIMARY_COLOR_INVERTED
	} else {
		TEXT_PRIMARY_COLOR
	});

	let meta_text = if has_children {
		format!("{} items", asset.children.len())
	} else {
		kind_hint.to_string()
	};

	let row_background = if depth.is_multiple_of(2) {
		BACKGROUND_COLOR
	} else {
		BACKGROUND_ACTIVE_COLOR
	};

	let row_content = row![
		container(text("")).width((depth as f32 * 16.0) + 2.0),
		expander_element,
		kind_badge,
		container(name_label).width(Length::Fill),
		text(meta_text).size(11).color(TEXT_PRIMARY_COLOR),
	]
	.align_y(iced::Alignment::Center)
	.height(16);

	let row_container = container(row_content)
		.width(Length::Fill)
		.padding([6, 8])
		.style(move |_| Style {
			background: Some(
				(if is_selected {
					BACKGROUND_ACTIVE_COLOR
				} else {
					row_background
				})
				.into(),
			),
			..Default::default()
		});

	let selectable_row = mouse_area(row_container).on_press(ProjectPaneMessage::Select(asset.id()));
	let mut children = vec![selectable_row.into()];

	if is_expanded {
		for child in &asset.children {
			if let Some(child_asset) = project.get_asset(child) {
				children.push(view_item(project, child_asset, state, depth + 1));
			}
		}
	}

	column(children).into()
}

fn info_pill<'a>(label: String) -> Element<'a, ProjectPaneMessage> {
	container(text(label).size(11).color(ACCENT_COLOR))
		.padding([2, 8])
		.style(move |_| container::Style {
			background: Some(
				Color {
					a: 0.14,
					..ACCENT_COLOR
				}
				.into(),
			),
			..Default::default()
		})
		.into()
}

#[derive(Default)]
struct ProjectSummary {
	total: usize,
	folders: usize,
	compositions: usize,
	media: usize,
}
