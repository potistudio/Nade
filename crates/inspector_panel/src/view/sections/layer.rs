//! Layer section: parent, blend mode and nested composition.

use constants::icons::{Icon, icon};
use constants::style::{FONT_UI, SPACE_1, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use constants::widgets::{self, button_body_center, ui_label};
use core::BlendMode;
use iced::widget::{button, column, pick_list, tooltip};
use iced::{Element, Length};

use crate::InspectorMessage;
use crate::target::{LayerTarget, ParentChoice};
use crate::view::{ROW_HEIGHT, labelled};

pub(crate) fn open_comp_button<'a>() -> Element<'a, InspectorMessage> {
	widgets::tip(
		button(button_body_center(icon(Icon::Open, 12.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press(InspectorMessage::OpenNestedComposition),
		"Open Composition",
		tooltip::Position::Left,
	)
}

/// Parent, blend mode and (for comp layers) the nested composition.
pub(crate) fn layer_rows<'a>(layer: &LayerTarget<'a>) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;
	let current = layer
		.parents
		.iter()
		.find(|choice| choice.id == instance.parent)
		.cloned()
		.unwrap_or_else(ParentChoice::none);
	let parent = pick_list(layer.parents.clone(), Some(current), |choice: ParentChoice| {
		InspectorMessage::SetParent(choice.id)
	})
	.text_size(FONT_UI)
	.padding([2, 8])
	.width(Length::Fill)
	.style(widgets::pick_list)
	.menu_style(widgets::menu);
	let blend = pick_list(
		BlendMode::ALL,
		Some(instance.blend_mode),
		InspectorMessage::SetBlendMode,
	)
	.text_size(FONT_UI)
	.padding([2, 8])
	.width(Length::Fill)
	.style(widgets::pick_list)
	.menu_style(widgets::menu);

	let mut rows = column![labelled("Parent", parent.into()), labelled("Blend", blend.into())].spacing(SPACE_1 + 1.0);
	if let Some(name) = layer.nested {
		rows = rows.push(labelled(
			"Source",
			ui_label(name, FONT_UI)
				.color(TEXT_PRIMARY_COLOR)
				.wrapping(iced::widget::text::Wrapping::None)
				.into(),
		));
	}
	rows.into()
}
