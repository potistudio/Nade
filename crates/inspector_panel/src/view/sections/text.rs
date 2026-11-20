//! Text section: content, font metrics and fill.

use iced::Element;
use iced::widget::{column, row, text_input};
use nade_constants::style::{FONT_UI, SPACE_1, SPACE_2, TEXT_MUTED_COLOR};
use nade_constants::widgets;

use crate::InspectorMessage;
use crate::draggable_number::number_field;
use crate::state::InspectorUi;
use crate::view::labelled;
use crate::view::sections::fill::fill_rows;

pub(crate) fn text_rows<'a>(
	text: &'a str,
	font_size: f32,
	spacing: f32,
	fill: [f32; 4],
	ui: &'a InspectorUi,
) -> Element<'a, InspectorMessage> {
	let content = text_input("Text", text)
		.on_input(InspectorMessage::SetText)
		.size(FONT_UI)
		.padding([2, 7])
		.style(widgets::text_input);

	let metrics = row![
		number_field(font_size, InspectorMessage::SetFontSize)
			.default_value(48.0)
			.suffix(" px")
			.step(0.5)
			.precision(0),
		number_field(spacing, InspectorMessage::SetTracking)
			.prefix("⇔", TEXT_MUTED_COLOR)
			.default_value(0.0)
			.step(0.1)
			.precision(1),
	]
	.spacing(SPACE_2);

	let mut rows =
		column![labelled("Content", content.into()), labelled("Font", metrics.into())].spacing(SPACE_1 + 1.0);
	for fill_row in fill_rows(fill, ui) {
		rows = rows.push(fill_row);
	}
	rows.into()
}
