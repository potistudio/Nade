//! One effect of the stack: header controls and its parameters.

use constants::icons::{Icon, icon, icon_transformed};
use constants::style::{FONT_UI, SPACE_1, SPACE_2, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use constants::widgets::{self, button_body_center, ui_label};
use domain::LayerEffect;
use iced::widget::{button, column, container, row, tooltip};
use iced::{Alignment, Color, Element, Length};

use crate::state::InspectorUi;
use crate::target::LayerTarget;
use crate::view::ROW_HEIGHT;
use crate::view::sections::effect_param::effect_param_row;
use crate::view::sections::effects::effect_message;
use crate::{EffectEdit, InspectorMessage};

pub(crate) fn small_button<'a>(
	content: Element<'a, InspectorMessage>,
	label: &'a str,
	message: Option<InspectorMessage>,
) -> Element<'a, InspectorMessage> {
	widgets::tip(
		button(button_body_center(content))
			.padding(0)
			.width(18)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press_maybe(message),
		label,
		tooltip::Position::Left,
	)
}

/// One effect: header (fold, fx switch, name, order, reset, remove) and controls.
pub(crate) fn effect_block<'a>(
	layer: &LayerTarget<'a>,
	effect: &'a LayerEffect,
	index: usize,
	count: usize,
	ui: &'a InspectorUi,
) -> Element<'a, InspectorMessage> {
	let id = effect.id;
	let open = ui.is_effect_open(layer.instance.id(), id);
	let name_color = if effect.enabled {
		TEXT_PRIMARY_COLOR
	} else {
		TEXT_MUTED_COLOR
	};

	let fold = button(widgets::button_body(
		row![
			icon_transformed(
				Icon::ChevronRight,
				9.0,
				TEXT_SECONDARY_COLOR,
				if open { std::f32::consts::FRAC_PI_2 } else { 0.0 },
				1.0,
			),
			ui_label(effect.name(), FONT_UI)
				.color(name_color)
				.wrapping(iced::widget::text::Wrapping::None),
		]
		.spacing(SPACE_2 + 1.0)
		.align_y(Alignment::Center),
	))
	.padding([0.0, SPACE_1])
	.width(Length::Fill)
	.height(ROW_HEIGHT)
	.style(widgets::button_ghost)
	.on_press(effect_message(id, EffectEdit::ToggleExpanded));

	let switch = widgets::tip(
		button(button_body_center(icon(
			Icon::Sparkle,
			11.0,
			if effect.enabled {
				constants::style::ACCENT_BRIGHT_COLOR
			} else {
				TEXT_MUTED_COLOR
			},
		)))
		.padding(0)
		.width(20)
		.height(ROW_HEIGHT)
		.style(widgets::button_toggle(effect.enabled))
		.on_press(effect_message(id, EffectEdit::ToggleEnabled)),
		if effect.enabled {
			"Turn Effect Off"
		} else {
			"Turn Effect On"
		},
		tooltip::Position::Left,
	);

	let arrow = |up: bool, enabled: bool| {
		let color = if enabled {
			TEXT_SECONDARY_COLOR
		} else {
			Color {
				a: 0.3,
				..TEXT_MUTED_COLOR
			}
		};
		let rotation = if up {
			-std::f32::consts::FRAC_PI_2
		} else {
			std::f32::consts::FRAC_PI_2
		};
		small_button(
			icon_transformed(Icon::ChevronRight, 9.0, color, rotation, 1.0),
			if up { "Move Up" } else { "Move Down" },
			enabled.then_some(effect_message(id, EffectEdit::Move { up })),
		)
	};

	let header = row![
		switch,
		fold,
		arrow(true, index > 0),
		arrow(false, index + 1 < count),
		small_button(
			icon(Icon::Reset, 11.0, TEXT_SECONDARY_COLOR),
			"Reset Effect",
			Some(effect_message(id, EffectEdit::Reset)),
		),
		small_button(
			icon(Icon::Close, 10.0, TEXT_SECONDARY_COLOR),
			"Remove Effect",
			Some(effect_message(id, EffectEdit::Remove)),
		),
	]
	.spacing(SPACE_1)
	.align_y(Alignment::Center);

	let mut content = column![header].spacing(SPACE_1 + 1.0);
	if open {
		for (param, _) in effect.kind.params().iter().enumerate() {
			content = content.push(effect_param_row(layer, effect, param));
		}
	}
	container(content)
		.padding([SPACE_1, SPACE_1])
		.width(Length::Fill)
		.style(widgets::well)
		.into()
}
