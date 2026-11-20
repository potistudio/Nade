//! Transform section: keyframe-aware anchor / position / scale / rotation / opacity rows.

use iced::widget::{Space, button, row, slider, tooltip};
use iced::{Alignment, Color, Element, Length};
use nade_constants::icons::{Icon, icon};
use nade_constants::style::{
	AXIS_X_COLOR, AXIS_Y_COLOR, FONT_UI, KEYFRAME_COLOR, SPACE_2, SPACE_3, TEXT_MUTED_COLOR, TEXT_SECONDARY_COLOR,
};
use nade_constants::widgets::{self, button_body_center, ui_label};
use nade_domain::{Instance, TransformProperty};

use crate::draggable_number::{KeyState, number_field};
use crate::state::InspectorUi;
use crate::target::LayerTarget;
use crate::view::{LABEL_WIDTH, ROW_HEIGHT};
use crate::{InspectorMessage, PropertyGroup};

pub(crate) fn reset_button<'a>() -> Element<'a, InspectorMessage> {
	widgets::tip(
		button(button_body_center(icon(Icon::Reset, 12.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press(InspectorMessage::ResetTransform),
		"Reset Transform",
		tooltip::Position::Left,
	)
}

pub(crate) fn key_state(instance: &Instance, property: TransformProperty, time: f32) -> KeyState {
	let curve = instance.animation.curve(property);
	if curve.is_empty() {
		KeyState::Static
	} else if curve.key_at(time).is_some() {
		KeyState::OnKey
	} else {
		KeyState::Animated
	}
}

/// `[‹][◆][›]` — key navigation around the stopwatch toggle.
pub(crate) fn key_cluster<'a>(layer: &LayerTarget<'a>, group: PropertyGroup) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;
	let channels = group.channels();
	let animated = channels.iter().any(|property| instance.is_animated(*property));
	let on_key = channels
		.iter()
		.any(|property| instance.animation.curve(*property).key_at(layer.time).is_some());

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
			.on_press(InspectorMessage::ToggleKeyframe(group)),
		label,
		tooltip::Position::Left,
	);

	let navigate = |forward: bool| -> Element<'a, InspectorMessage> {
		if !animated {
			return Space::new().width(12).into();
		}
		let exists = channels.iter().any(|property| {
			let curve = instance.animation.curve(*property);
			if forward {
				curve.next_key_time(layer.time).is_some()
			} else {
				curve.previous_key_time(layer.time).is_some()
			}
		});
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
			.on_press_maybe(exists.then_some(InspectorMessage::JumpToKey { group, forward }))
			.into()
	};

	row![navigate(false), diamond, navigate(true)]
		.spacing(0)
		.align_y(Alignment::Center)
		.into()
}

pub(crate) fn property_row<'a>(
	layer: &LayerTarget<'a>,
	group: PropertyGroup,
	ui: &InspectorUi,
) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;
	let time = layer.time;
	let value = |property: TransformProperty| instance.property_at(property, time);
	let field = |property: TransformProperty| {
		number_field(value(property), move |value| InspectorMessage::SetChannel {
			property,
			value,
		})
		.default_value(PropertyGroup::default_value(property))
		.key_state(key_state(instance, property, time))
	};

	let fields: Element<'a, InspectorMessage> = match group {
		PropertyGroup::Anchor => row![
			field(TransformProperty::AnchorX)
				.prefix("X", AXIS_X_COLOR)
				.step(1.0)
				.precision(1),
			field(TransformProperty::AnchorY)
				.prefix("Y", AXIS_Y_COLOR)
				.step(1.0)
				.precision(1),
		]
		.spacing(SPACE_2)
		.into(),
		PropertyGroup::Position => row![
			field(TransformProperty::PositionX)
				.prefix("X", AXIS_X_COLOR)
				.step(1.0)
				.precision(1),
			field(TransformProperty::PositionY)
				.prefix("Y", AXIS_Y_COLOR)
				.step(1.0)
				.precision(1),
		]
		.spacing(SPACE_2)
		.into(),
		PropertyGroup::Rotation => row![
			field(TransformProperty::RotationZ).suffix("°").step(0.5).precision(1),
			Space::new().width(Length::Fill),
		]
		.spacing(SPACE_2)
		.into(),
		PropertyGroup::Scale => {
			let link = widgets::tip(
				button(button_body_center(icon(
					Icon::Link,
					12.0,
					if ui.scale_linked {
						nade_constants::style::ACCENT_BRIGHT_COLOR
					} else {
						TEXT_MUTED_COLOR
					},
				)))
				.padding(0)
				.width(20)
				.height(ROW_HEIGHT)
				.style(widgets::button_toggle(ui.scale_linked))
				.on_press(InspectorMessage::ToggleScaleLink),
				"Constrain Proportions",
				tooltip::Position::Left,
			);
			row![
				field(TransformProperty::ScaleX)
					.prefix("X", AXIS_X_COLOR)
					.scale(100.0)
					.suffix("%")
					.step(0.005)
					.precision(1),
				field(TransformProperty::ScaleY)
					.prefix("Y", AXIS_Y_COLOR)
					.scale(100.0)
					.suffix("%")
					.step(0.005)
					.precision(1),
				link,
			]
			.spacing(SPACE_2)
			.align_y(Alignment::Center)
			.into()
		}
		PropertyGroup::Opacity => {
			let opacity = value(TransformProperty::Opacity);
			row![
				slider(0.0_f32..=1.0, opacity, |value| InspectorMessage::SetChannel {
					property: TransformProperty::Opacity,
					value,
				})
				.step(0.01_f32)
				.width(Length::Fill)
				.style(widgets::slider),
				field(TransformProperty::Opacity)
					.scale(100.0)
					.suffix("%")
					.step(0.005)
					.precision(0)
					.width(58),
			]
			.spacing(SPACE_3 + 2.0)
			.align_y(Alignment::Center)
			.into()
		}
	};

	row![
		key_cluster(layer, group),
		ui_label(group.label(), FONT_UI)
			.color(TEXT_SECONDARY_COLOR)
			.width(LABEL_WIDTH),
		fields,
	]
	.spacing(SPACE_2)
	.align_y(Alignment::Center)
	.height(ROW_HEIGHT)
	.into()
}
