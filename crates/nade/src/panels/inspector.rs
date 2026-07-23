use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use constants::style::{
	FONT_TINY, FONT_TITLE, FONT_UI, PAD_PANEL, SPACE_1, SPACE_2, SPACE_3, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR,
	TEXT_SECONDARY_COLOR,
};
use constants::widgets;
use core::Transform;
use domain::InstanceContent;
use iced::widget::{column, container, row, rule, text_input};
use iced::{Element, Length};
use inspector_panel::InspectorMessage;
use panel_system::PanelSystemMessage;

pub struct InspectorSelection<'a> {
	pub transform: &'a Transform,
	pub content: &'a InstanceContent,
}

pub fn view<'a>(
	selection: Option<InspectorSelection<'a>>,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	container(if let Some(selection) = selection {
		let transform = selection.transform;
		let mut col = column![
			section_label("Transform"),
			property_row("Location", &transform.position, |axis, val| {
				InspectorMessage::UpdateTransform {
					field: "position".into(),
					index: axis,
					value: val,
				}
			}),
			property_row("Rotation", &transform.rotation, |axis, val| {
				InspectorMessage::UpdateTransform {
					field: "rotation".into(),
					index: axis,
					value: val,
				}
			}),
			property_row("Scale", &transform.scale, |axis, val| {
				InspectorMessage::UpdateTransform {
					field: "scale".into(),
					index: axis,
					value: val,
				}
			}),
			divider(),
			row![
				widgets::ui_label("Opacity", FONT_UI)
					.color(TEXT_SECONDARY_COLOR)
					.width(64),
				crate::widgets::draggable_number::draggable_number(
					transform.opacity,
					1.0,
					InspectorMessage::SetOpacity
				)
				.step(0.01),
			]
			.spacing(SPACE_2)
			.align_y(iced::Alignment::Center),
		]
		.spacing(SPACE_3);

		if let Some((text, _font_path, font_size, _spacing, fill_color)) = selection.content.as_text() {
			col = col.push(divider()).push(section_label("Text")).push(
				row![
					widgets::ui_label("Content", FONT_UI)
						.color(TEXT_SECONDARY_COLOR)
						.width(64),
					text_input("Text", text)
						.on_input(InspectorMessage::SetText)
						.padding([SPACE_1, SPACE_2])
						.width(Length::Fill)
						.style(widgets::text_input),
				]
				.spacing(SPACE_2)
				.align_y(iced::Alignment::Center),
			);

			col = col.push(
				row![
					widgets::ui_label("Size", FONT_UI).color(TEXT_SECONDARY_COLOR).width(64),
					crate::widgets::draggable_number::draggable_number(font_size, 48.0, InspectorMessage::SetFontSize)
						.step(1.0),
				]
				.spacing(SPACE_2)
				.align_y(iced::Alignment::Center),
			);

			col = col.push(color_row("Color", fill_color));
		}

		Element::from(col).map(|msg| PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(msg)))
	} else {
		let col = column![
			section_label("Properties"),
			widgets::ui_label("Nothing selected", FONT_UI).color(TEXT_MUTED_COLOR),
		]
		.spacing(SPACE_2);

		Element::from(col).map(|_: InspectorMessage| {
			PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(InspectorMessage::SetOpacity(0.0)))
		})
	})
	.padding(PAD_PANEL)
	.width(Length::Fill)
	.height(Length::Fill)
	.style(widgets::panel)
	.into()
}

fn section_label<'a>(label: &'a str) -> Element<'a, InspectorMessage> {
	widgets::ui_label(label, FONT_TITLE).color(TEXT_PRIMARY_COLOR).into()
}

fn divider<'a>() -> Element<'a, InspectorMessage> {
	rule::horizontal(1)
		.style(|_theme| rule::Style {
			color: constants::style::BORDER_SUBTLE_COLOR,
			radius: 0.0.into(),
			fill_mode: rule::FillMode::Full,
			snap: true,
		})
		.into()
}

fn property_row<'a, F>(label: &'a str, values: &[f32; 3], message_fn: F) -> Element<'a, InspectorMessage>
where
	F: Fn(usize, f32) -> InspectorMessage + 'a + Clone,
{
	use crate::widgets::draggable_number::draggable_number;

	let axis = |name: &'a str, index: usize, value: f32, message_fn: F| {
		row![
			widgets::ui_label(name, FONT_TINY).color(TEXT_MUTED_COLOR).width(10),
			draggable_number(value, 0.0, move |v| message_fn(index, v)).step(0.1),
		]
		.spacing(SPACE_1)
		.align_y(iced::Alignment::Center)
	};

	row![
		widgets::ui_label(label, FONT_UI).color(TEXT_SECONDARY_COLOR).width(64),
		axis("X", 0, values[0], message_fn.clone()),
		axis("Y", 1, values[1], message_fn.clone()),
		axis("Z", 2, values[2], message_fn),
	]
	.spacing(SPACE_2)
	.align_y(iced::Alignment::Center)
	.into()
}

fn color_row<'a>(label: &'a str, values: [f32; 4]) -> Element<'a, InspectorMessage> {
	use crate::widgets::draggable_number::draggable_number;

	let channel = |name: &'a str, index: usize, value: f32| {
		row![
			widgets::ui_label(name, FONT_TINY).color(TEXT_MUTED_COLOR).width(10),
			draggable_number(value, 1.0, move |v| InspectorMessage::SetFillColor { index, value: v }).step(0.01),
		]
		.spacing(SPACE_1)
		.align_y(iced::Alignment::Center)
	};

	row![
		widgets::ui_label(label, FONT_UI).color(TEXT_SECONDARY_COLOR).width(64),
		channel("R", 0, values[0]),
		channel("G", 1, values[1]),
		channel("B", 2, values[2]),
		channel("A", 3, values[3]),
	]
	.spacing(SPACE_2)
	.align_y(iced::Alignment::Center)
	.into()
}
