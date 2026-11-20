//! Effects section: "Add Effect" picker and the layer's effect stack.

use constants::icons::{Icon, icon};
use constants::style::{FONT_UI, SPACE_1, SPACE_2, TEXT_MUTED_COLOR, TEXT_SECONDARY_COLOR};
use constants::widgets::{self, button_body_center, ui_label};
use domain::{EffectKind, InstanceContent, MediaKind};
use iced::widget::{button, column, container, pick_list, row, tooltip};
use iced::{Alignment, Element, Length};

use crate::state::InspectorUi;
use crate::target::LayerTarget;
use crate::view::ROW_HEIGHT;
use crate::view::sections::effect_block::effect_block;
use crate::{EffectEdit, InspectorMessage};

/// Whether the layer draws pixels effects could change.
pub(crate) fn accepts_effects(content: &InstanceContent) -> bool {
	match content {
		InstanceContent::Empty => false,
		InstanceContent::Media(source) => source.kind != MediaKind::Audio,
		_ => true,
	}
}

pub(crate) fn remove_all_effects_button<'a>() -> Element<'a, InspectorMessage> {
	widgets::tip(
		button(button_body_center(icon(Icon::Trash, 12.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press(InspectorMessage::RemoveAllEffects),
		"Remove All Effects",
		tooltip::Position::Left,
	)
}

/// "Add Effect" picker followed by every effect of the stack.
pub(crate) fn effect_rows<'a>(layer: &LayerTarget<'a>, ui: &'a InspectorUi) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;
	let add = pick_list(EffectKind::ALL, None::<EffectKind>, InspectorMessage::AddEffect)
		.placeholder("Add Effect…")
		.text_size(FONT_UI)
		.padding([2, 8])
		.width(Length::Fill)
		.style(widgets::pick_list)
		.menu_style(widgets::menu);

	let mut rows = column![
		row![icon(Icon::Plus, 12.0, TEXT_MUTED_COLOR), add]
			.spacing(SPACE_2)
			.align_y(Alignment::Center)
			.height(ROW_HEIGHT)
	]
	.spacing(SPACE_2);

	if instance.effects.is_empty() {
		rows = rows.push(
			container(ui_label("No effects. Pick one above or from the Effect menu.", FONT_UI).color(TEXT_MUTED_COLOR))
				.padding([SPACE_1, SPACE_1]),
		);
	}
	let count = instance.effects.len();
	for (index, effect) in instance.effects.iter().enumerate() {
		rows = rows.push(effect_block(layer, effect, index, count, ui));
	}
	rows.into()
}

pub(crate) fn effect_message(id: u32, edit: EffectEdit) -> InspectorMessage {
	InspectorMessage::Effect { id, edit }
}
