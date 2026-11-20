//! Fill rows: swatch, hex field, opacity and color wells.

use iced::widget::{Space, button, container, row, text_input};
use iced::{Alignment, Border, Color, Element, Length};
use nade_constants::style::{FONT_UI, LABEL_COLORS, MONOSPACE_FONT, SPACE_2};
use nade_constants::widgets;

use crate::InspectorMessage;
use crate::draggable_number::number_field;
use crate::state::InspectorUi;
use crate::view::{LABEL_WIDTH, labelled};

pub(crate) fn to_hex(color: [f32; 4]) -> String {
	let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
	format!(
		"#{:02X}{:02X}{:02X}",
		channel(color[0]),
		channel(color[1]),
		channel(color[2])
	)
}

pub(crate) fn fill_rows<'a>(fill: [f32; 4], ui: &'a InspectorUi) -> Vec<Element<'a, InspectorMessage>> {
	let color = Color::from_rgba(fill[0], fill[1], fill[2], 1.0);
	let hex = ui.hex_draft.clone().unwrap_or_else(|| to_hex(fill));
	let fill_row = row![
		container(Space::new())
			.width(22)
			.height(20)
			.style(widgets::swatch(color)),
		text_input("#FFFFFF", &hex)
			.on_input(InspectorMessage::HexEdited)
			.on_submit(InspectorMessage::HexSubmitted)
			.font(MONOSPACE_FONT)
			.size(FONT_UI)
			.padding([2, 6])
			.width(Length::Fill)
			.style(widgets::text_input),
		number_field(fill[3], |value| InspectorMessage::SetFillChannel { index: 3, value })
			.default_value(1.0)
			.scale(100.0)
			.suffix("%")
			.step(0.005)
			.precision(0)
			.width(58),
	]
	.spacing(SPACE_2)
	.align_y(Alignment::Center);

	// Apple 風カラーウェル
	let wells = row(std::iter::once(Color::WHITE).chain(LABEL_COLORS).map(|well| {
		let selected =
			(well.r - fill[0]).abs() < 0.01 && (well.g - fill[1]).abs() < 0.01 && (well.b - fill[2]).abs() < 0.01;
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
						Color::from_rgba(0.0, 0.0, 0.0, 0.25)
					},
					width: if selected { 2.0 } else { 1.0 },
					radius: 7.0.into(),
				},
				..button::Style::default()
			})
			.on_press(InspectorMessage::SetFillRgb([well.r, well.g, well.b]))
			.into()
	}))
	.spacing(SPACE_2)
	.wrap()
	.vertical_spacing(SPACE_2);

	vec![
		labelled("Fill", fill_row.into()),
		row![Space::new().width(42.0 + LABEL_WIDTH + SPACE_2 * 2.0), wells].into(),
	]
}

#[cfg(test)]
mod tests {
	use super::to_hex;

	#[test]
	fn formats_fill_as_hex() {
		assert_eq!(to_hex([1.0, 0.0, 0.5, 1.0]), "#FF0080");
		assert_eq!(to_hex([2.0, -1.0, 0.0, 1.0]), "#FF0000");
	}
}
