//! Solid section: size and fill.

use iced::Element;
use iced::widget::{column, row};
use nade_constants::style::{AXIS_X_COLOR, AXIS_Y_COLOR, SPACE_1, SPACE_2};

use crate::InspectorMessage;
use crate::draggable_number::number_field;
use crate::state::InspectorUi;
use crate::view::labelled;
use crate::view::sections::fill::fill_rows;

pub(crate) fn solid_rows<'a>(size: [f32; 2], color: [f32; 4], ui: &'a InspectorUi) -> Element<'a, InspectorMessage> {
	let [width, height] = size;
	let dimensions = row![
		number_field(width, move |width| InspectorMessage::SetSolidSize {
			width: width.max(1.0),
			height,
		})
		.prefix("W", AXIS_X_COLOR)
		.suffix(" px")
		.step(1.0)
		.precision(0),
		number_field(height, move |height| InspectorMessage::SetSolidSize {
			width,
			height: height.max(1.0),
		})
		.prefix("H", AXIS_Y_COLOR)
		.suffix(" px")
		.step(1.0)
		.precision(0),
	]
	.spacing(SPACE_2);

	let mut rows = column![labelled("Size", dimensions.into())].spacing(SPACE_1 + 1.0);
	for fill_row in fill_rows(color, ui) {
		rows = rows.push(fill_row);
	}
	rows.into()
}
