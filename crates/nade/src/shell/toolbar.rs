//! Unified toolbar: tool palette and create actions.

use constants::icons::{Icon, icon};
use constants::style::{SPACE_2, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use constants::widgets::{self, button_body_center};
use iced::widget::{Space, button, container, row, tooltip};
use iced::{Alignment, Element, Length};

use crate::message::{Message, ShellMessage};
use crate::theme::{SHELL_GUTTER, SHELL_TOOLBAR_HEIGHT, TOOL_SEGMENT};
use crate::widgets::segmented::{Segment, segmented};
use viewer_panel::Tool;

pub struct ToolbarState {
	pub tool: Tool,
	pub tool_pill: f32,
}

pub fn view<'a>(state: ToolbarState) -> Element<'a, Message> {
	let tools = segmented(
		Tool::ALL
			.into_iter()
			.map(|tool| {
				let color = if tool == state.tool {
					TEXT_PRIMARY_COLOR
				} else {
					TEXT_SECONDARY_COLOR
				};
				Segment::new(tool, icon(tool.icon(), 15.0, color)).tooltip(tool.tooltip())
			})
			.collect(),
		state.tool,
		state.tool_pill,
		TOOL_SEGMENT,
		SPACE_2,
		|tool| Message::Shell(ShellMessage::SelectTool(tool)),
	);

	let actions = row![
		action(Icon::Open, "Open Project…  ⌘O", ShellMessage::OpenProject),
		action(Icon::Save, "Save Project  ⌘S", ShellMessage::SaveProject),
		widgets::vertical_separator(12.0),
		action(Icon::Import, "Import Media…  ⌘I", ShellMessage::ImportMedia),
		action(Icon::Comp, "New Composition", ShellMessage::NewComposition),
		action(Icon::LayerText, "New Text Layer", ShellMessage::NewTextLayer),
		action(Icon::LayerSolid, "New Solid  ⌘Y", ShellMessage::NewSolid),
		action(Icon::Export, "Export…  ⌘M", ShellMessage::Export),
	]
	.spacing(SPACE_2)
	.align_y(Alignment::Center);

	container(
		row![tools, Space::new().width(Length::Fill), actions]
			.align_y(Alignment::Center)
			.width(Length::Fill)
			.height(Length::Fill),
	)
	.width(Length::Fill)
	.height(SHELL_TOOLBAR_HEIGHT)
	.padding(SHELL_GUTTER)
	.into()
}

fn action<'a>(glyph: Icon, label: &'a str, message: ShellMessage) -> Element<'a, Message> {
	widgets::tip(
		button(button_body_center(icon(glyph, 15.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(TOOL_SEGMENT[0])
			.height(TOOL_SEGMENT[1])
			.style(widgets::button_ghost)
			.on_press(Message::Shell(message)),
		label,
		tooltip::Position::Bottom,
	)
}
