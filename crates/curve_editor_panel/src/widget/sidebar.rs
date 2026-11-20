//! Property list with key counts.

use crate::TOOLBAR_HEIGHT;
use crate::helpers::{EDITABLE_PROPERTIES, property_color};
use crate::message::CurveEditorMessage;
use crate::widget::CurveEditorWidget;
use constants::icons::{Icon, icon};
use constants::style::{
	FONT_TINY, FONT_UI, KEYFRAME_COLOR, SPACE_1, SPACE_3, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body, mono_label, ui_label};
use iced::{
	Alignment, Border, Element, Length,
	widget::{Space, button, column, container, row},
};

impl<'a> CurveEditorWidget<'a> {
	/// Property list with key counts.
	pub(super) fn sidebar(&self) -> Element<'a, CurveEditorMessage> {
		EDITABLE_PROPERTIES
			.into_iter()
			.fold(column![].spacing(1), |column, property| {
				let selected = property == self.state.selected_property;
				let keys = self.animation.curve(property).keys().len();
				let dot = container(Space::new().width(8).height(8)).style(move |_| container::Style {
					background: Some(property_color(property).into()),
					border: Border {
						radius: 4.0.into(),
						..Default::default()
					},
					..Default::default()
				});
				let mut content = row![
					dot,
					ui_label(property.label(), FONT_UI).color(if selected {
						TEXT_PRIMARY_COLOR
					} else {
						TEXT_SECONDARY_COLOR
					}),
					Space::new().width(Length::Fill),
				]
				.spacing(SPACE_3 + 1.0)
				.align_y(Alignment::Center);
				if keys > 0 {
					content = content.push(
						row![
							icon(Icon::KeyframeFilled, 8.0, KEYFRAME_COLOR),
							mono_label(keys.to_string(), FONT_TINY).color(TEXT_MUTED_COLOR),
						]
						.spacing(SPACE_1 + 1.0)
						.align_y(Alignment::Center),
					);
				}
				column.push(
					button(button_body(content))
						.padding([0.0, SPACE_3 + 2.0])
						.width(Length::Fill)
						.height(TOOLBAR_HEIGHT)
						.style(widgets::button_toggle(selected))
						.on_press(CurveEditorMessage::SelectProperty(property)),
				)
			})
			.into()
	}
}
