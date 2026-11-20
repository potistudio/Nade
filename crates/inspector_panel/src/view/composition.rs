//! Composition settings: facts, duration, background and nesting.

use constants::icons::{Icon, icon};
use constants::style::{
	FONT_TINY, FONT_TITLE, FONT_UI, LABEL_COLORS, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_5, TEXT_MUTED_COLOR,
	TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body_center, mono_label, ui_label};
use core::CompositionSummary;
use iced::widget::{Space, button, column, container, pick_list, row, scrollable, tooltip};
use iced::{Alignment, Border, Color, Element, Length};

use crate::InspectorMessage;
use crate::draggable_number::number_field;
use crate::target::CompositionChoice;
use crate::view::ROW_HEIGHT;
use crate::view::header::badge;

pub(crate) fn composition_view<'a>(
	summary: CompositionSummary<'a>,
	layers: usize,
	background: [f32; 4],
	nest_targets: Vec<CompositionChoice>,
) -> Element<'a, InspectorMessage> {
	let header = row![
		badge(Icon::Comp, constants::style::ACCENT_COLOR),
		column![
			ui_label(summary.name, FONT_TITLE + 1.0).color(TEXT_PRIMARY_COLOR),
			ui_label("Composition", FONT_TINY + 1.0).color(TEXT_MUTED_COLOR),
		]
		.spacing(SPACE_2 + 1.0),
	]
	.spacing(SPACE_4 + 2.0)
	.align_y(Alignment::Center);

	let fact = |label: &'a str, value: String| -> Element<'a, InspectorMessage> {
		row![
			ui_label(label, FONT_UI).color(TEXT_SECONDARY_COLOR).width(Length::Fill),
			mono_label(value, FONT_UI).color(TEXT_PRIMARY_COLOR),
		]
		.align_y(Alignment::Center)
		.height(ROW_HEIGHT)
		.into()
	};

	let facts = container(
		column![
			fact("Resolution", format!("{} × {}", summary.width, summary.height)),
			fact("Frame Rate", format!("{:.2} fps", summary.fps)),
			fact("Layers", layers.to_string()),
		]
		.spacing(SPACE_1),
	)
	.padding([SPACE_3, SPACE_4 + 2.0])
	.style(widgets::card);

	let duration = row![
		number_field(summary.duration, InspectorMessage::SetCompositionDuration)
			.default_value(10.0)
			.suffix(" s")
			.step(0.05)
			.precision(2),
		mono_label(core::timecode::format(summary.duration, summary.fps), FONT_UI).color(TEXT_MUTED_COLOR),
		widgets::tip(
			button(button_body_center(icon(Icon::Fit, 12.0, TEXT_SECONDARY_COLOR)))
				.padding(0)
				.width(24)
				.height(ROW_HEIGHT)
				.style(widgets::button_ghost)
				.on_press(InspectorMessage::FitCompositionToLayers),
			"Fit Duration to Layers",
			tooltip::Position::Left,
		),
	]
	.spacing(SPACE_2)
	.align_y(Alignment::Center);

	let swatches = [
		Color::BLACK,
		Color::from_rgb(0.18, 0.18, 0.18),
		Color::from_rgb(0.5, 0.5, 0.5),
		Color::WHITE,
	];
	let backgrounds = row(swatches.into_iter().chain(LABEL_COLORS).map(|well| {
		let selected = (well.r - background[0]).abs() < 0.01
			&& (well.g - background[1]).abs() < 0.01
			&& (well.b - background[2]).abs() < 0.01;
		button(Space::new().width(14).height(14))
			.padding(0)
			.style(move |_theme, status| button::Style {
				background: Some(well.into()),
				border: Border {
					color: if selected {
						Color::WHITE
					} else if matches!(status, button::Status::Hovered) {
						Color::from_rgba(1.0, 1.0, 1.0, 0.6)
					} else {
						Color::from_rgba(1.0, 1.0, 1.0, 0.18)
					},
					width: if selected { 2.0 } else { 1.0 },
					radius: 7.0.into(),
				},
				..button::Style::default()
			})
			.on_press(InspectorMessage::SetCompositionBackground([well.r, well.g, well.b]))
			.into()
	}))
	.spacing(SPACE_2)
	.wrap()
	.vertical_spacing(SPACE_2);

	let setting_label = |label: &'a str| ui_label(label, FONT_UI).color(TEXT_SECONDARY_COLOR).width(76);
	let settings = container(
		column![
			row![setting_label("Duration"), duration]
				.spacing(SPACE_2)
				.align_y(Alignment::Center)
				.height(ROW_HEIGHT),
			row![setting_label("Background"), backgrounds]
				.spacing(SPACE_2)
				.align_y(Alignment::Center),
			row![
				setting_label("Add to"),
				pick_list(nest_targets, None::<CompositionChoice>, |choice: CompositionChoice| {
					InspectorMessage::NestInto(choice.id)
				})
				.placeholder("Nest in composition…")
				.text_size(FONT_UI)
				.padding([2, 8])
				.width(Length::Fill)
				.style(widgets::pick_list)
				.menu_style(widgets::menu),
			]
			.spacing(SPACE_2)
			.align_y(Alignment::Center)
			.height(ROW_HEIGHT),
		]
		.spacing(SPACE_2),
	)
	.padding([SPACE_3, SPACE_4 + 2.0])
	.style(widgets::card);

	scrollable(
		column![header, facts, settings]
			.spacing(SPACE_5)
			.padding([SPACE_5, SPACE_4 + 2.0]),
	)
	.style(widgets::scrollable)
	.into()
}
