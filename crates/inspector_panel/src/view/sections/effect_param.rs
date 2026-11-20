//! One effect parameter row with its key cluster.

use iced::widget::{Space, button, container, pick_list, row, slider, tooltip};
use iced::{Alignment, Color, Element, Length};
use nade_constants::icons::{Icon, icon};
use nade_constants::style::{
	AXIS_X_COLOR, AXIS_Y_COLOR, FONT_UI, KEYFRAME_COLOR, SPACE_1, SPACE_2, SPACE_3, TEXT_MUTED_COLOR,
	TEXT_SECONDARY_COLOR,
};
use nade_constants::widgets::{self, button_body_center, ui_label};
use nade_domain::{LayerEffect, ParamKind};

use crate::draggable_number::{KeyState, number_field};
use crate::target::LayerTarget;
use crate::view::sections::effects::effect_message;
use crate::view::{EFFECT_LABEL_WIDTH, KEY_CLUSTER_WIDTH, ROW_HEIGHT};
use crate::{EffectEdit, InspectorMessage};

pub(crate) fn effect_key_state(effect: &LayerEffect, slot: usize, time: f32) -> KeyState {
	let curve = effect.curve(slot);
	if curve.is_empty() {
		KeyState::Static
	} else if curve.key_at(time).is_some() {
		KeyState::OnKey
	} else {
		KeyState::Animated
	}
}

/// `[‹][◆][›]` for an effect parameter.
pub(crate) fn effect_key_cluster<'a>(effect: &LayerEffect, param: usize, time: f32) -> Element<'a, InspectorMessage> {
	let id = effect.id;
	let animated = effect.is_animated(param);
	let on_key = effect.has_key_at(param, time);
	let (glyph, color, label) = match (animated, on_key) {
		(_, true) => (Icon::KeyframeFilled, KEYFRAME_COLOR, "Remove Keyframe"),
		(true, false) => (Icon::Keyframe, KEYFRAME_COLOR, "Add Keyframe"),
		(false, false) => (Icon::Keyframe, TEXT_MUTED_COLOR, "Animate Property"),
	};
	let diamond = widgets::tip(
		button(button_body_center(icon(glyph, 12.0, color)))
			.padding(0)
			.width(18)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press(effect_message(id, EffectEdit::ToggleKey { param })),
		label,
		tooltip::Position::Left,
	);
	let navigate = |forward: bool| -> Element<'a, InspectorMessage> {
		if !animated {
			return Space::new().width(12).into();
		}
		let exists = effect.adjacent_key_time(param, time, forward).is_some();
		let glyph = if forward { Icon::ChevronRight } else { Icon::ChevronLeft };
		let color = if exists {
			TEXT_SECONDARY_COLOR
		} else {
			Color {
				a: 0.3,
				..TEXT_MUTED_COLOR
			}
		};
		button(button_body_center(icon(glyph, 9.0, color)))
			.padding(0)
			.width(12)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press_maybe(exists.then_some(effect_message(id, EffectEdit::JumpToKey { param, forward })))
			.into()
	};
	row![navigate(false), diamond, navigate(true)]
		.spacing(0)
		.align_y(Alignment::Center)
		.into()
}

/// A pick list entry of a choice parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChoiceOption {
	index: usize,
	label: &'static str,
}

impl std::fmt::Display for ChoiceOption {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.label)
	}
}

pub(crate) fn effect_param_row<'a>(
	layer: &LayerTarget<'a>,
	effect: &'a LayerEffect,
	param: usize,
) -> Element<'a, InspectorMessage> {
	let time = layer.time;
	let id = effect.id;
	let spec = effect.kind.params()[param];
	let slots = effect.kind.slot_range(param);
	let first = slots.start;
	let value = |slot: usize| effect.value_at(slot, time);
	let set = move |slot: usize| move |value: f32| effect_message(id, EffectEdit::SetValue { slot, value });
	let field = |slot: usize| {
		let channel = slot - first;
		number_field(value(slot), set(slot))
			.default_value(spec.default.get(channel).copied().unwrap_or(0.0))
			.key_state(effect_key_state(effect, slot, time))
	};

	let control: Element<'a, InspectorMessage> = match spec.kind {
		ParamKind::Number {
			step,
			precision,
			suffix,
			..
		} => field(first).step(step).precision(precision).suffix(suffix).into(),
		ParamKind::Percent { min, max } => row![
			slider(min..=max, value(first), set(first))
				.step(0.01_f32)
				.width(Length::Fill)
				.style(widgets::slider),
			field(first).scale(100.0).suffix("%").step(0.005).precision(0).width(58),
		]
		.spacing(SPACE_3 + 2.0)
		.align_y(Alignment::Center)
		.into(),
		ParamKind::Angle => field(first).suffix("°").step(0.5).precision(1).into(),
		ParamKind::Pixels { .. } => field(first).suffix(" px").step(0.1).precision(1).into(),
		ParamKind::Point => row![
			field(first).prefix("X", AXIS_X_COLOR).step(1.0).precision(1),
			field(first + 1).prefix("Y", AXIS_Y_COLOR).step(1.0).precision(1),
		]
		.spacing(SPACE_2)
		.into(),
		ParamKind::Color => {
			let swatch = Color::from_rgb(value(first), value(first + 1), value(first + 2));
			let channel = |slot: usize, label: &'static str| {
				field(slot)
					.prefix(label, TEXT_MUTED_COLOR)
					.scale(255.0)
					.step(0.004)
					.precision(0)
			};
			row![
				container(Space::new())
					.width(16)
					.height(16)
					.style(widgets::swatch(swatch)),
				channel(first, "R"),
				channel(first + 1, "G"),
				channel(first + 2, "B"),
			]
			.spacing(SPACE_1 + 1.0)
			.align_y(Alignment::Center)
			.into()
		}
		ParamKind::Choice(options) => {
			let choices: Vec<ChoiceOption> = options
				.iter()
				.enumerate()
				.map(|(index, label)| ChoiceOption { index, label })
				.collect();
			let selected = choices.get(value(first).round().max(0.0) as usize).copied();
			pick_list(choices, selected, move |choice: ChoiceOption| {
				effect_message(
					id,
					EffectEdit::SetValue {
						slot: first,
						value: choice.index as f32,
					},
				)
			})
			.text_size(FONT_UI)
			.padding([2, 8])
			.width(Length::Fill)
			.style(widgets::pick_list)
			.menu_style(widgets::menu)
			.into()
		}
		ParamKind::Toggle => {
			let on = value(first) >= 0.5;
			row![
				button(button_body_center(icon(
					Icon::Check,
					11.0,
					if on {
						nade_constants::style::ACCENT_BRIGHT_COLOR
					} else {
						Color::TRANSPARENT
					},
				)))
				.padding(0)
				.width(ROW_HEIGHT)
				.height(ROW_HEIGHT)
				.style(widgets::button_toggle(on))
				.on_press(effect_message(
					id,
					EffectEdit::SetValue {
						slot: first,
						value: if on { 0.0 } else { 1.0 },
					},
				)),
				Space::new().width(Length::Fill),
			]
			.align_y(Alignment::Center)
			.into()
		}
	};

	let keys: Element<'a, InspectorMessage> = if spec.kind.is_animatable() {
		effect_key_cluster(effect, param, time)
	} else {
		Space::new().width(KEY_CLUSTER_WIDTH).into()
	};
	row![
		keys,
		ui_label(spec.name, FONT_UI)
			.color(TEXT_SECONDARY_COLOR)
			.wrapping(iced::widget::text::Wrapping::None)
			.width(EFFECT_LABEL_WIDTH),
		control,
	]
	.spacing(SPACE_2)
	.align_y(Alignment::Center)
	.height(ROW_HEIGHT)
	.into()
}
