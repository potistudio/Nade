use crate::{ObjectId, ProjectPaneMessage, ProjectPaneState};
use constants::icons::{Icon, icon};
use constants::style::*;
use constants::tree::{Disclosure, TreeRow};
use constants::widgets::{self, button_body_center, ui_label};
use core::CompositionId;
use domain::{Composition, Instance, Project};
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, tooltip};
use iced::{Alignment, Element, Length};

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
			ui_label("COMPOSITIONS", FONT_TINY).color(TEXT_MUTED_COLOR),
			Space::new().width(Length::Fill),
			tool_button(Icon::Comp, "New Composition", ProjectPaneMessage::NewComposition),
			tool_button(Icon::LayerText, "New Text Layer", ProjectPaneMessage::AddObject),
		]
		.spacing(SPACE_1)
		.align_y(Alignment::Center)
		.padding([SPACE_2, SPACE_4 + 2.0]);

		let mut composition_ids = project.compositions();
		composition_ids.sort_by_key(|id| id.value());

		let body: Element<'a, ProjectPaneMessage> = if composition_ids.is_empty() {
			empty_state()
		} else {
			let tree = column(
				composition_ids
					.iter()
					.filter_map(|id| project.composition(id).map(|comp| view_composition(comp, state))),
			)
			.spacing(1);

			let clear_area = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill))
				.on_press(ProjectPaneMessage::ClearSelection);

			scrollable(column![tree, clear_area].width(Length::Fill))
				.height(Length::Fill)
				.style(widgets::scrollable)
				.into()
		};

		container(column![toolbar, body])
			.width(Length::Fill)
			.height(Length::Fill)
			.style(widgets::panel_surface)
			.into()
	}
}

//* -------- Private API -------- */
fn tool_button<'a>(glyph: Icon, label: &'a str, message: ProjectPaneMessage) -> Element<'a, ProjectPaneMessage> {
	widgets::tip(
		button(button_body_center(icon(glyph, 13.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(22)
			.style(widgets::button_ghost)
			.on_press(message),
		label,
		tooltip::Position::Bottom,
	)
}

fn empty_state<'a>() -> Element<'a, ProjectPaneMessage> {
	container(
		column![
			icon(Icon::Comp, 24.0, TEXT_MUTED_COLOR),
			ui_label("No Compositions", FONT_UI).color(TEXT_SECONDARY_COLOR),
		]
		.spacing(SPACE_3)
		.align_x(Alignment::Center),
	)
	.center(Length::Fill)
	.into()
}

fn view_composition<'a>(composition: &'a Composition, state: &'a ProjectPaneState) -> Element<'a, ProjectPaneMessage> {
	let id = composition.id();
	let object_id = ObjectId::Composition(id);
	let is_expanded = state.expanded_ids.contains(&id);

	let mut children = vec![
		TreeRow {
			depth: 0,
			disclosure: composition.has_objects().then_some(Disclosure {
				expanded: is_expanded,
				on_toggle: ProjectPaneMessage::ToggleExpand(id),
			}),
			icon: Icon::Comp,
			icon_color: ACCENT_BRIGHT_COLOR,
			label: composition.name(),
			meta: Some(format!("{}×{}", composition.width(), composition.height())),
			selected: state.selected_id == Some(object_id),
			on_select: ProjectPaneMessage::Select(object_id),
			on_open: ProjectPaneMessage::OpenItem(object_id),
		}
		.view(),
	];

	if is_expanded {
		// AE と同じくレイヤー順（上のトラックほど前面）で並べる
		let mut layers: Vec<&Instance> = composition.all_objects().collect();
		layers.sort_by_key(|instance| (instance.track_index(), instance.id()));
		children.extend(layers.into_iter().map(|instance| view_instance(id, instance, state)));
	}

	column(children).spacing(1).into()
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

	let glyph = match instance.content() {
		domain::InstanceContent::Text { .. } => Icon::LayerText,
		domain::InstanceContent::Empty => Icon::LayerNull,
	};

	TreeRow {
		depth: 1,
		disclosure: None,
		icon: glyph,
		icon_color: label_color(instance.track_index()),
		label: instance.name(),
		meta: Some(format!("{}", instance.track_index() + 1)),
		selected: state.selected_id == Some(object_id),
		on_select: ProjectPaneMessage::Select(object_id),
		on_open: ProjectPaneMessage::OpenItem(object_id),
	}
	.view()
}
