use crate::{ObjectId, ProjectPaneMessage, ProjectPaneState};
use constants::style::*;
use constants::widgets;
use core::CompositionId;
use domain::{Composition, Instance, Project};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text};
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

		let toolbar = row![
			tool_button("Comp", ProjectPaneMessage::NewComposition),
			tool_button("Object", ProjectPaneMessage::AddObject),
		]
		.spacing(SPACE_1)
		.padding([SPACE_1, SPACE_2]);

		let mut composition_ids = project.compositions();
		composition_ids.sort_by_key(|id| id.value());

		let tree = column(
			composition_ids
				.iter()
				.filter_map(|id| project.composition(id).map(|comp| view_composition(comp, state))),
		)
		.spacing(0);

		let clear_area = mouse_area(container(text("")).width(Length::Fill).height(Length::Fill))
			.on_press(ProjectPaneMessage::ClearSelection);

		let body = scrollable(column![tree, clear_area].width(Length::Fill)).height(Length::Fill);

		container(column![toolbar, body].spacing(SPACE_1))
			.width(Length::Fill)
			.height(Length::Fill)
			.padding([SPACE_1, 0.0])
			.style(widgets::panel)
			.into()
	}
}

//* -------- Private API -------- */
fn tool_button<'a>(label: &'a str, message: ProjectPaneMessage) -> Element<'a, ProjectPaneMessage> {
	button(widgets::button_body(
		widgets::ui_label(label, FONT_TINY).color(TEXT_PRIMARY_COLOR),
	))
	.height(ROW_HEIGHT + SPACE_1 * 2.0)
	.padding([0.0, SPACE_2])
	.style(widgets::button_ghost)
	.on_press(message)
	.into()
}

fn view_composition<'a>(composition: &'a Composition, state: &'a ProjectPaneState) -> Element<'a, ProjectPaneMessage> {
	let id = composition.id();
	let object_id = ObjectId::Composition(id);
	let is_expanded = state.expanded_ids.contains(&id);
	let is_selected = state.selected_id == Some(object_id);
	let has_children = composition.has_objects();

	let mut children = vec![view_row(
		0,
		"COMP",
		composition.name(),
		has_children,
		is_expanded,
		is_selected,
		Some(ProjectPaneMessage::ToggleExpand(id)),
		object_id,
	)];

	if is_expanded {
		for instance in composition.all_objects() {
			children.push(view_instance(id, instance, state));
		}
	}

	column(children).into()
}

fn view_instance<'a>(
	composition: CompositionId,
	instance: &'a Instance,
	state: &'a ProjectPaneState,
) -> Element<'a, ProjectPaneMessage> {
	let object_id = ObjectId::Instance {
		composition,
		instance: instance.id(),
	};
	let is_selected = state.selected_id == Some(object_id);

	view_row(1, "OBJ", instance.name(), false, false, is_selected, None, object_id)
}

fn view_row<'a>(
	depth: usize,
	kind_tag: &'a str,
	name: &'a str,
	has_children: bool,
	is_expanded: bool,
	is_selected: bool,
	expand_message: Option<ProjectPaneMessage>,
	object_id: ObjectId,
) -> Element<'a, ProjectPaneMessage> {
	let expander_symbol = if has_children {
		if is_expanded { "▾" } else { "▸" }
	} else {
		" "
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
	let expander_element: Element<'a, ProjectPaneMessage> = if let Some(message) = expand_message {
		mouse_area(expander).on_press(message).into()
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
		container(constants::widgets::ui_label(name, FONT_LABEL).color(text_color))
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

	mouse_area(row_container)
		.on_press(ProjectPaneMessage::Select(object_id))
		.on_double_click(ProjectPaneMessage::OpenItem(object_id))
		.into()
}
