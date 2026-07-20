use crate::{ProjectPaneMessage, ProjectPaneState};
use constants::style::*;
use constants::widgets;
use domain::{Asset, AssetType, Project};
use iced::widget::{column, container, mouse_area, row, text};
use iced::{Element, Length};

#[derive(Debug)]
pub struct ProjectPaneWidget<'a> {
	project: &'a Project,
}

//* -------- Public API -------- */
impl<'a> ProjectPaneWidget<'a> {
	pub fn new(project: &'a Project) -> Self {
		Self { project }
	}

	pub fn view(self, state: &'a ProjectPaneState) -> Element<'a, ProjectPaneMessage> {
		let project = self.project;
		let root_assets = project.assets();

		// No inner header — dock chrome already provides the editor title.
		let tree = column(
			root_assets
				.iter()
				.filter_map(|id| project.asset(*id).map(|asset| view_item(project, asset, state, 0))),
		)
		.spacing(0);

		let clear_area = mouse_area(container(text("")).width(Length::Fill).height(Length::Fill))
			.on_press(ProjectPaneMessage::ClearSelection);

		container(column![tree, clear_area])
			.width(Length::Fill)
			.height(Length::Fill)
			.padding([SPACE_1, 0.0])
			.style(widgets::panel)
			.into()
	}
}

//* -------- Private API -------- */
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
		" "
	};

	let kind_tag = match asset.kind() {
		AssetType::Folder => "DIR",
		AssetType::Composition => "COMP",
		AssetType::Image => "IMG",
		AssetType::Video => "VID",
		AssetType::Audio => "AUD",
	};

	let text_color = if is_selected {
		TEXT_PRIMARY_COLOR_INVERTED
	} else {
		TEXT_PRIMARY_COLOR
	};
	let meta_color = if is_selected {
		TEXT_PRIMARY_COLOR_INVERTED
	} else {
		TEXT_SECONDARY_COLOR
	};

	let expander = container(constants::widgets::ui_label(expander_symbol, FONT_UI).color(meta_color))
		.height(ROW_HEIGHT)
		.center_y(ROW_HEIGHT);
	let expander_element: Element<'a, ProjectPaneMessage> = if has_children {
		mouse_area(expander)
			.on_press(ProjectPaneMessage::ToggleExpand(asset.id()))
			.into()
	} else {
		expander.into()
	};

	let row_content = row![
		container(text("")).width((depth as f32 * TREE_INDENT) + SPACE_2),
		expander_element,
		container(constants::widgets::ui_label(kind_tag, FONT_TINY).color(meta_color))
			.width(36.0)
			.height(ROW_HEIGHT)
			.align_y(iced::Alignment::Center),
		container(constants::widgets::ui_label(&asset.name, FONT_LABEL).color(text_color))
			.width(Length::Fill)
			.height(ROW_HEIGHT)
			.align_y(iced::Alignment::Center),
	]
	.align_y(iced::Alignment::Center)
	.spacing(SPACE_2)
	.height(ROW_HEIGHT);

	let row_container = container(row_content)
		.width(Length::Fill)
		.height(ROW_HEIGHT + SPACE_1 * 2.0)
		.padding([SPACE_1, SPACE_2])
		.align_y(iced::Alignment::Center)
		.style(widgets::list_row(is_selected));

	let selectable_row = mouse_area(row_container).on_press(ProjectPaneMessage::Select(asset.id()));
	let mut children = vec![selectable_row.into()];

	if is_expanded {
		for child in &asset.children {
			if let Some(child_asset) = project.asset(*child) {
				children.push(view_item(project, child_asset, state, depth + 1));
			}
		}
	}

	column(children).into()
}
