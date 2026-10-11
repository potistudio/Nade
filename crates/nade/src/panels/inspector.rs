//! Properties panel (After Effects "Effect Controls" / Cavalry "Attribute
//! Editor"): layer header, collapsible sections and keyframe-aware rows.

use constants::icons::{Icon, icon, icon_transformed};
use constants::style::{
	AXIS_X_COLOR, AXIS_Y_COLOR, FONT_TINY, FONT_TITLE, FONT_UI, KEYFRAME_COLOR, LABEL_COLORS, SPACE_1, SPACE_2,
	SPACE_3, SPACE_4, SPACE_5, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body_center, mono_label, ui_label};
use domain::{Instance, InstanceContent, TransformProperty};
use iced::time::{Duration, Instant};
use iced::widget::{Space, button, column, container, row, scrollable, slider, text_input, tooltip};
use iced::{Alignment, Animation, Border, Color, Element, Length};
use inspector_panel::{InspectorMessage, PropertyGroup, Section};
use panel_system::PanelSystemMessage;

use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::viewer::CompositionSummary;
use crate::widgets::draggable_number::{KeyState, number_field};

const LABEL_WIDTH: f32 = 56.0;
const ROW_HEIGHT: f32 = 24.0;

/// Panel-local UI state owned by the app (section disclosure, links, drafts).
#[derive(Debug)]
pub struct InspectorUi {
	transform_open: Animation<bool>,
	text_open: Animation<bool>,
	pub scale_linked: bool,
	pub hex_draft: Option<String>,
}

impl Default for InspectorUi {
	fn default() -> Self {
		let disclosure = || {
			Animation::new(true)
				.duration(Duration::from_millis(constants::style::MOTION_FAST_MS + 40))
				.easing(iced::animation::Easing::EaseOutCubic)
		};
		Self {
			transform_open: disclosure(),
			text_open: disclosure(),
			scale_linked: true,
			hex_draft: None,
		}
	}
}

impl InspectorUi {
	fn disclosure(&self, section: Section) -> &Animation<bool> {
		match section {
			Section::Transform => &self.transform_open,
			Section::Text => &self.text_open,
		}
	}

	pub fn toggle(&mut self, section: Section, now: Instant) {
		let animation = match section {
			Section::Transform => &mut self.transform_open,
			Section::Text => &mut self.text_open,
		};
		let open = animation.value();
		animation.go_mut(!open, now);
	}

	fn is_open(&self, section: Section) -> bool {
		self.disclosure(section).value()
	}

	pub fn is_animating(&self, now: Instant) -> bool {
		self.transform_open.is_animating(now) || self.text_open.is_animating(now)
	}
}

pub struct LayerTarget<'a> {
	pub instance: &'a Instance,
	pub time: f32,
	pub label: Color,
}

pub enum InspectorTarget<'a> {
	Empty,
	Composition {
		summary: CompositionSummary<'a>,
		layers: usize,
	},
	Layer(LayerTarget<'a>),
}

pub fn view<'a>(
	target: InspectorTarget<'a>,
	ui: &'a InspectorUi,
	now: Instant,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content: Element<'a, InspectorMessage> = match target {
		InspectorTarget::Empty => empty_state(),
		InspectorTarget::Composition { summary, layers } => composition_view(summary, layers),
		InspectorTarget::Layer(layer) => layer_view(layer, ui, now),
	};

	let panel: Element<'a, InspectorMessage> = container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(widgets::panel_surface)
		.into();
	panel.map(|message| PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(message)))
}

// -----------------------------------------------------------------------------
// States
// -----------------------------------------------------------------------------

fn empty_state<'a>() -> Element<'a, InspectorMessage> {
	container(
		column![
			icon(Icon::PanelInspector, 28.0, TEXT_MUTED_COLOR),
			ui_label("No Layer Selected", FONT_TITLE).color(TEXT_SECONDARY_COLOR),
			container(
				ui_label(
					"Select a layer in the viewer, timeline or layers panel to edit its properties.",
					FONT_UI
				)
				.color(TEXT_MUTED_COLOR)
				.align_x(Alignment::Center)
				.line_height(1.4),
			)
			.max_width(220),
		]
		.spacing(SPACE_3 + 2.0)
		.align_x(Alignment::Center),
	)
	.center(Length::Fill)
	.padding(SPACE_5)
	.into()
}

fn composition_view<'a>(summary: CompositionSummary<'a>, layers: usize) -> Element<'a, InspectorMessage> {
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
			fact("Duration", core::timecode::format(summary.duration, summary.fps)),
			fact("Layers", layers.to_string()),
		]
		.spacing(SPACE_1),
	)
	.padding([SPACE_3, SPACE_4 + 2.0])
	.style(widgets::card);

	scrollable(
		column![header, facts]
			.spacing(SPACE_5)
			.padding([SPACE_5, SPACE_4 + 2.0]),
	)
	.style(widgets::scrollable)
	.into()
}

fn layer_view<'a>(layer: LayerTarget<'a>, ui: &'a InspectorUi, now: Instant) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;
	let is_text = matches!(instance.content(), InstanceContent::Text { .. });

	let mut sections = column![layer_header(&layer, is_text)].spacing(SPACE_4 + 2.0);

	let transform_rows = PropertyGroup::ALL
		.into_iter()
		.map(|group| property_row(&layer, group, ui))
		.collect::<Vec<_>>();
	sections = sections.push(section(
		Section::Transform,
		Icon::Transform,
		"Transform",
		Some(reset_button()),
		ui,
		now,
		column(transform_rows).spacing(SPACE_1 + 1.0).into(),
	));

	if let Some((text, _font, font_size, spacing, fill)) = instance.content().as_text() {
		sections = sections.push(section(
			Section::Text,
			Icon::ToolText,
			"Text",
			None,
			ui,
			now,
			text_rows(text, font_size, spacing, fill, ui),
		));
	}

	scrollable(sections.padding([SPACE_4 + 2.0, SPACE_4 + 2.0]))
		.style(widgets::scrollable)
		.into()
}

// -----------------------------------------------------------------------------
// Header
// -----------------------------------------------------------------------------

fn badge<'a>(glyph: Icon, color: Color) -> Element<'a, InspectorMessage> {
	container(icon(glyph, 15.0, color))
		.width(30)
		.height(30)
		.center(30)
		.style(widgets::chip_tinted(color))
		.into()
}

fn layer_header<'a>(layer: &LayerTarget<'a>, is_text: bool) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;
	let (glyph, kind) = if is_text {
		(Icon::LayerText, "Text Layer")
	} else {
		(Icon::LayerNull, "Null Object")
	};

	let span = format!("{kind}  ·  {:.2}s", instance.duration());

	row![
		badge(glyph, layer.label),
		column![
			text_input("Layer name", instance.name())
				.on_input(InspectorMessage::Rename)
				.size(FONT_TITLE + 1.0)
				.padding([2, 4])
				.style(widgets::title_input),
			container(ui_label(span, FONT_TINY + 0.5).color(TEXT_MUTED_COLOR)).padding([0, 4]),
		]
		.spacing(SPACE_1),
	]
	.spacing(SPACE_3 + 2.0)
	.align_y(Alignment::Center)
	.into()
}

// -----------------------------------------------------------------------------
// Sections
// -----------------------------------------------------------------------------

fn section<'a>(
	kind: Section,
	glyph: Icon,
	title: &'a str,
	trailing: Option<Element<'a, InspectorMessage>>,
	ui: &InspectorUi,
	now: Instant,
	body: Element<'a, InspectorMessage>,
) -> Element<'a, InspectorMessage> {
	let openness = ui.disclosure(kind).interpolate(0.0, 1.0, now);

	let toggle = button(widgets::button_body(
		row![
			icon_transformed(
				Icon::ChevronRight,
				10.0,
				TEXT_SECONDARY_COLOR,
				openness * std::f32::consts::FRAC_PI_2,
				1.0,
			),
			icon(glyph, 13.0, TEXT_SECONDARY_COLOR),
			ui_label(title, FONT_UI + 0.5).color(TEXT_PRIMARY_COLOR),
		]
		.spacing(SPACE_3 + 1.0)
		.align_y(Alignment::Center),
	))
	.padding([0.0, SPACE_2])
	.width(Length::Fill)
	.height(26)
	.style(widgets::button_ghost)
	.on_press(InspectorMessage::ToggleSection(kind));

	let mut header = row![toggle].spacing(SPACE_1).align_y(Alignment::Center);
	if let Some(trailing) = trailing {
		header = header.push(trailing);
	}

	let mut content = column![header].spacing(SPACE_2);
	if ui.is_open(kind) {
		content = content.push(container(body).padding([SPACE_1, SPACE_2]));
	}

	container(content)
		.padding([SPACE_2, SPACE_2])
		.width(Length::Fill)
		.style(widgets::card)
		.into()
}

fn reset_button<'a>() -> Element<'a, InspectorMessage> {
	widgets::tip(
		button(button_body_center(icon(Icon::Reset, 12.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(24)
			.style(widgets::button_ghost)
			.on_press(InspectorMessage::ResetTransform),
		"Reset Transform",
		tooltip::Position::Left,
	)
}

// -----------------------------------------------------------------------------
// Transform rows
// -----------------------------------------------------------------------------

fn key_state(instance: &Instance, property: TransformProperty, time: f32) -> KeyState {
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
fn key_cluster<'a>(layer: &LayerTarget<'a>, group: PropertyGroup) -> Element<'a, InspectorMessage> {
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
			.height(ROW_HEIGHT - 2.0)
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
			.height(ROW_HEIGHT - 2.0)
			.style(widgets::button_ghost)
			.on_press_maybe(exists.then_some(InspectorMessage::JumpToKey { group, forward }))
			.into()
	};

	row![navigate(false), diamond, navigate(true)]
		.spacing(0)
		.align_y(Alignment::Center)
		.into()
}

fn property_row<'a>(layer: &LayerTarget<'a>, group: PropertyGroup, ui: &InspectorUi) -> Element<'a, InspectorMessage> {
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
						constants::style::ACCENT_BRIGHT_COLOR
					} else {
						TEXT_MUTED_COLOR
					},
				)))
				.padding(0)
				.width(20)
				.height(ROW_HEIGHT - 2.0)
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

// -----------------------------------------------------------------------------
// Text rows
// -----------------------------------------------------------------------------

fn to_hex(color: [f32; 4]) -> String {
	let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
	format!(
		"#{:02X}{:02X}{:02X}",
		channel(color[0]),
		channel(color[1]),
		channel(color[2])
	)
}

fn text_rows<'a>(
	text: &'a str,
	font_size: f32,
	spacing: f32,
	fill: [f32; 4],
	ui: &'a InspectorUi,
) -> Element<'a, InspectorMessage> {
	let labelled = |label: &'a str, content: Element<'a, InspectorMessage>| -> Element<'a, InspectorMessage> {
		row![
			Space::new().width(42),
			ui_label(label, FONT_UI).color(TEXT_SECONDARY_COLOR).width(LABEL_WIDTH),
			content,
		]
		.spacing(SPACE_2)
		.align_y(Alignment::Center)
		.height(ROW_HEIGHT)
		.into()
	};

	let content = text_input("Text", text)
		.on_input(InspectorMessage::SetText)
		.size(FONT_UI)
		.padding([3, 7])
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
			.font(iced::Font::MONOSPACE)
			.size(FONT_UI)
			.padding([3, 6])
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

	column![
		labelled("Content", content.into()),
		labelled("Font", metrics.into()),
		labelled("Fill", fill_row.into()),
		row![Space::new().width(42.0 + LABEL_WIDTH + SPACE_2 * 2.0), wells],
	]
	.spacing(SPACE_1 + 1.0)
	.into()
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
