use crate::message::{AppPanelMessage, PropertyMessage};
use crate::panel_content::PanelContent;
use crate::theme::panel::PROPERTIES_BG;
use crate::theme::{TEXT_MUTED, TEXT_PRIMARY};
use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use nade_core::Transform;
use panel_system::PanelSystemMessage;

pub fn view<'a>(
	selection: Option<&Transform>,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	container(if let Some(transform) = selection {
		let col = column![
			text("Properties").size(14).color(TEXT_PRIMARY),
			// Position
			transform_row("Position", &transform.position, |axis, val| {
				PropertyMessage::Position(axis, val)
			}),
			// Rotation
			transform_row("Rotation", &transform.rotation, |axis, val| {
				PropertyMessage::Rotation(axis, val)
			}),
			// Scale
			transform_row("Scale", &transform.scale, |axis, val| {
				PropertyMessage::Scale(axis, val)
			}),
			// Opacity
			column![
				text("Opacity").size(12).color(TEXT_MUTED),
				crate::widgets::draggable_number::draggable_number(
					transform.opacity,
					1.0,
					PropertyMessage::Opacity
				)
				.step(0.01),
			]
			.spacing(5),
		]
		.spacing(15);

		Element::from(col).map(|msg| PanelSystemMessage::AppMessage(AppPanelMessage::Property(msg)))
	} else {
		let col = column![
			text("Properties").size(14).color(TEXT_PRIMARY),
			text("No selection").size(12).color(TEXT_MUTED),
		]
		.spacing(10);

		Element::from(col).map(|_: PropertyMessage| {
			// This branch has no interactive elements, so this is unreachable
			PanelSystemMessage::AppMessage(AppPanelMessage::Property(PropertyMessage::Opacity(0.0)))
		})
	})
	.padding(10)
	.width(Length::Fill)
	.height(Length::Fill)
	.style(|_| container::Style {
		background: Some(PROPERTIES_BG.into()),
		..Default::default()
	})
	.into()
}

fn transform_row<'a, F>(
	label: &'a str,
	values: &[f32; 3],
	message_fn: F,
) -> Element<'a, PropertyMessage>
where
	F: Fn(usize, f32) -> PropertyMessage + 'a + Clone,
{
	let axis_label = |t: &'a str| text(t).size(12).color(TEXT_MUTED).width(15);
	use crate::widgets::draggable_number::draggable_number;

	column![
		text(label).size(12).color(TEXT_MUTED),
		row![
			row![axis_label("X"), {
				let func = message_fn.clone();
				draggable_number(values[0], 0.0, move |v| func(0, v)).step(0.1)
			}]
			.spacing(5),
			row![axis_label("Y"), {
				let func = message_fn.clone();
				draggable_number(values[1], 0.0, move |v| func(1, v)).step(0.1)
			}]
			.spacing(5),
			row![axis_label("Z"), {
				let func = message_fn;
				draggable_number(values[2], 0.0, move |v| func(2, v)).step(0.1)
			}]
			.spacing(5),
		]
		.spacing(10)
	]
	.spacing(5)
	.into()
}
