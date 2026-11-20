//! Properties panel (After Effects "Effect Controls" / Cavalry "Attribute
//! Editor"): layer header, collapsible sections and keyframe-aware rows.

use constants::icons::{Icon, icon, icon_transformed};
use constants::style::{
	AXIS_X_COLOR, AXIS_Y_COLOR, FONT_TINY, FONT_TITLE, FONT_UI, KEYFRAME_COLOR, LABEL_COLORS, MONOSPACE_FONT, SPACE_1,
	SPACE_2, SPACE_3, SPACE_4, SPACE_5, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body_center, mono_label, ui_label};
use core::{BlendMode, CompositionId, InstanceId};
use domain::{
	EffectKind, Instance, InstanceContent, LayerEffect, MediaKind, MediaSource, ParamKind, TransformProperty,
};
use iced::time::{Duration, Instant};
use iced::widget::{Space, button, column, container, pick_list, row, scrollable, slider, text_input, tooltip};
use iced::{Alignment, Animation, Border, Color, Element, Length};
use inspector_panel::{EffectEdit, InspectorMessage, PropertyGroup, Section};
use panel_system::PanelSystemMessage;
use std::collections::HashSet;

use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::widgets::draggable_number::{KeyState, number_field};
use core::CompositionSummary;

const LABEL_WIDTH: f32 = 56.0;
/// Effect parameter names are longer than transform labels.
const EFFECT_LABEL_WIDTH: f32 = 92.0;
/// Width of the `[‹][◆][›]` key cluster.
const KEY_CLUSTER_WIDTH: f32 = 42.0;
const ROW_HEIGHT: f32 = constants::style::ROW_HEIGHT;

/// Panel-local UI state owned by the app (section disclosure, links, drafts).
#[derive(Debug)]
pub struct InspectorUi {
	layer_open: Animation<bool>,
	transform_open: Animation<bool>,
	text_open: Animation<bool>,
	solid_open: Animation<bool>,
	footage_open: Animation<bool>,
	effects_open: Animation<bool>,
	/// Effects whose controls are folded, per layer.
	collapsed_effects: HashSet<(InstanceId, u32)>,
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
			layer_open: disclosure(),
			transform_open: disclosure(),
			text_open: disclosure(),
			solid_open: disclosure(),
			footage_open: disclosure(),
			effects_open: disclosure(),
			collapsed_effects: HashSet::new(),
			scale_linked: true,
			hex_draft: None,
		}
	}
}

impl InspectorUi {
	fn disclosure(&self, section: Section) -> &Animation<bool> {
		match section {
			Section::Layer => &self.layer_open,
			Section::Transform => &self.transform_open,
			Section::Text => &self.text_open,
			Section::Solid => &self.solid_open,
			Section::Footage => &self.footage_open,
			Section::Effects => &self.effects_open,
		}
	}

	pub fn toggle(&mut self, section: Section, now: Instant) {
		let animation = match section {
			Section::Layer => &mut self.layer_open,
			Section::Transform => &mut self.transform_open,
			Section::Text => &mut self.text_open,
			Section::Solid => &mut self.solid_open,
			Section::Footage => &mut self.footage_open,
			Section::Effects => &mut self.effects_open,
		};
		let open = animation.value();
		animation.go_mut(!open, now);
	}

	pub fn is_open(&self, section: Section) -> bool {
		self.disclosure(section).value()
	}
	/// Fold / unfold the controls of one effect.
	pub fn toggle_effect(&mut self, layer: InstanceId, effect: u32) {
		if !self.collapsed_effects.remove(&(layer, effect)) {
			self.collapsed_effects.insert((layer, effect));
		}
	}

	/// Unfold an effect (e.g. right after adding it).
	pub fn expand_effect(&mut self, layer: InstanceId, effect: u32) {
		self.collapsed_effects.remove(&(layer, effect));
	}

	fn is_effect_open(&self, layer: InstanceId, effect: u32) -> bool {
		!self.collapsed_effects.contains(&(layer, effect))
	}

	pub fn is_animating(&self, now: Instant) -> bool {
		[
			&self.layer_open,
			&self.transform_open,
			&self.text_open,
			&self.solid_open,
			&self.footage_open,
			&self.effects_open,
		]
		.iter()
		.any(|animation| animation.is_animating(now))
	}
}

pub struct LayerTarget<'a> {
	pub instance: &'a Instance,
	pub time: f32,
	pub label: Color,
	pub fps: f32,
	/// Layers this one may be parented to (no cycles), plus "None".
	pub parents: Vec<ParentChoice>,
	/// Name of the composition a comp layer plays.
	pub nested: Option<&'a str>,
}

/// An entry of the Parent picker.
#[derive(Debug, Clone, PartialEq)]
pub struct ParentChoice {
	pub id: Option<InstanceId>,
	pub name: String,
}

impl ParentChoice {
	pub fn none() -> Self {
		Self {
			id: None,
			name: "None".to_string(),
		}
	}
}

/// An entry of the "Add to" composition picker.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionChoice {
	pub id: CompositionId,
	pub name: String,
}

impl std::fmt::Display for CompositionChoice {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.name)
	}
}

impl std::fmt::Display for ParentChoice {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.name)
	}
}

pub enum InspectorTarget<'a> {
	Empty,
	Composition {
		summary: CompositionSummary<'a>,
		layers: usize,
		background: [f32; 4],
		/// Compositions this one can be nested into (no loops).
		nest_targets: Vec<CompositionChoice>,
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
		InspectorTarget::Composition {
			summary,
			layers,
			background,
			nest_targets,
		} => composition_view(summary, layers, background, nest_targets),
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

fn composition_view<'a>(
	summary: CompositionSummary<'a>,
	layers: usize,
	background: [f32; 4],
	nest_targets: Vec<CompositionChoice>,
) -> Element<'a, InspectorMessage> {
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
			fact("Layers", layers.to_string()),
		]
		.spacing(SPACE_1),
	)
	.padding([SPACE_3, SPACE_4 + 2.0])
	.style(widgets::card);

	let duration = row![
		number_field(summary.duration, InspectorMessage::SetCompositionDuration)
			.default_value(10.0)
			.suffix(" s")
			.step(0.05)
			.precision(2),
		mono_label(core::timecode::format(summary.duration, summary.fps), FONT_UI).color(TEXT_MUTED_COLOR),
		widgets::tip(
			button(button_body_center(icon(Icon::Fit, 12.0, TEXT_SECONDARY_COLOR)))
				.padding(0)
				.width(24)
				.height(ROW_HEIGHT)
				.style(widgets::button_ghost)
				.on_press(InspectorMessage::FitCompositionToLayers),
			"Fit Duration to Layers",
			tooltip::Position::Left,
		),
	]
	.spacing(SPACE_2)
	.align_y(Alignment::Center);

	let swatches = [
		Color::BLACK,
		Color::from_rgb(0.18, 0.18, 0.18),
		Color::from_rgb(0.5, 0.5, 0.5),
		Color::WHITE,
	];
	let backgrounds = row(swatches.into_iter().chain(LABEL_COLORS).map(|well| {
		let selected = (well.r - background[0]).abs() < 0.01
			&& (well.g - background[1]).abs() < 0.01
			&& (well.b - background[2]).abs() < 0.01;
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
						Color::from_rgba(1.0, 1.0, 1.0, 0.18)
					},
					width: if selected { 2.0 } else { 1.0 },
					radius: 7.0.into(),
				},
				..button::Style::default()
			})
			.on_press(InspectorMessage::SetCompositionBackground([well.r, well.g, well.b]))
			.into()
	}))
	.spacing(SPACE_2)
	.wrap()
	.vertical_spacing(SPACE_2);

	let setting_label = |label: &'a str| ui_label(label, FONT_UI).color(TEXT_SECONDARY_COLOR).width(76);
	let settings = container(
		column![
			row![setting_label("Duration"), duration]
				.spacing(SPACE_2)
				.align_y(Alignment::Center)
				.height(ROW_HEIGHT),
			row![setting_label("Background"), backgrounds]
				.spacing(SPACE_2)
				.align_y(Alignment::Center),
			row![
				setting_label("Add to"),
				pick_list(nest_targets, None::<CompositionChoice>, |choice: CompositionChoice| {
					InspectorMessage::NestInto(choice.id)
				})
				.placeholder("Nest in composition…")
				.text_size(FONT_UI)
				.padding([2, 8])
				.width(Length::Fill)
				.style(widgets::pick_list)
				.menu_style(widgets::menu),
			]
			.spacing(SPACE_2)
			.align_y(Alignment::Center)
			.height(ROW_HEIGHT),
		]
		.spacing(SPACE_2),
	)
	.padding([SPACE_3, SPACE_4 + 2.0])
	.style(widgets::card);

	scrollable(
		column![header, facts, settings]
			.spacing(SPACE_5)
			.padding([SPACE_5, SPACE_4 + 2.0]),
	)
	.style(widgets::scrollable)
	.into()
}

fn layer_view<'a>(layer: LayerTarget<'a>, ui: &'a InspectorUi, now: Instant) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;

	let mut sections = column![layer_header(&layer)].spacing(SPACE_4 + 2.0);

	let open = layer.nested.map(|_| open_comp_button());
	sections = sections.push(section(
		Section::Layer,
		Icon::PanelLayers,
		"Layer",
		open,
		ui,
		now,
		layer_rows(&layer),
	));

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

	if let Some((size, color)) = instance.content().as_solid() {
		sections = sections.push(section(
			Section::Solid,
			Icon::LayerSolid,
			"Solid",
			None,
			ui,
			now,
			solid_rows(size, color, ui),
		));
	}

	if let Some(source) = instance.content().as_media() {
		let fit = (source.kind != MediaKind::Audio).then(fit_button);
		sections = sections.push(section(
			Section::Footage,
			match source.kind {
				MediaKind::Image => Icon::Image,
				MediaKind::Video => Icon::Video,
				MediaKind::Audio => Icon::Audio,
			},
			"Footage",
			fit,
			ui,
			now,
			footage_rows(source, instance, layer.fps),
		));
	}

	if accepts_effects(instance.content()) {
		let clear = (!instance.effects.is_empty()).then(remove_all_effects_button);
		sections = sections.push(section(
			Section::Effects,
			Icon::Sparkle,
			"Effects",
			clear,
			ui,
			now,
			effect_rows(&layer, ui),
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
		.height(ROW_HEIGHT)
		.center_y(ROW_HEIGHT)
		.center_x(30)
		.style(widgets::chip_tinted(color))
		.into()
}

fn layer_header<'a>(layer: &LayerTarget<'a>) -> Element<'a, InspectorMessage> {
	let instance = layer.instance;
	let kind = instance.content().kind_label();
	let glyph = match instance.content() {
		InstanceContent::Empty => Icon::LayerNull,
		InstanceContent::Text { .. } => Icon::LayerText,
		InstanceContent::Solid { .. } => Icon::LayerSolid,
		InstanceContent::Media(source) => match source.kind {
			MediaKind::Image => Icon::Image,
			MediaKind::Video => Icon::Video,
			MediaKind::Audio => Icon::Audio,
		},
		InstanceContent::Composition(_) => Icon::Comp,
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
	.height(ROW_HEIGHT)
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

fn fit_button<'a>() -> Element<'a, InspectorMessage> {
	widgets::tip(
		button(button_body_center(icon(Icon::Fit, 12.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press(InspectorMessage::FitToComposition),
		"Fit to Composition",
		tooltip::Position::Left,
	)
}

fn reset_button<'a>() -> Element<'a, InspectorMessage> {
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
						constants::style::ACCENT_BRIGHT_COLOR
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

fn labelled<'a>(label: &'a str, content: Element<'a, InspectorMessage>) -> Element<'a, InspectorMessage> {
	row![
		Space::new().width(42),
		ui_label(label, FONT_UI).color(TEXT_SECONDARY_COLOR).width(LABEL_WIDTH),
		content,
	]
	.spacing(SPACE_2)
	.align_y(Alignment::Center)
	.height(ROW_HEIGHT)
	.into()
}

fn text_rows<'a>(
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

fn solid_rows<'a>(size: [f32; 2], color: [f32; 4], ui: &'a InspectorUi) -> Element<'a, InspectorMessage> {
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

fn open_comp_button<'a>() -> Element<'a, InspectorMessage> {
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
fn layer_rows<'a>(layer: &LayerTarget<'a>) -> Element<'a, InspectorMessage> {
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

fn footage_rows<'a>(source: &'a MediaSource, instance: &Instance, fps: f32) -> Element<'a, InspectorMessage> {
	let fact = |label: &'a str, value: String| -> Element<'a, InspectorMessage> {
		labelled(label, mono_label(value, FONT_UI).color(TEXT_PRIMARY_COLOR).into())
	};
	let file = source
		.path
		.file_name()
		.map(|name| name.to_string_lossy().into_owned())
		.unwrap_or_default();

	let mut rows = column![labelled(
		"File",
		ui_label(file, FONT_UI)
			.color(TEXT_PRIMARY_COLOR)
			.wrapping(iced::widget::text::Wrapping::None)
			.into()
	)]
	.spacing(SPACE_1 + 1.0);
	if source.kind != MediaKind::Audio {
		rows = rows.push(fact("Size", format!("{} × {}", source.info.width, source.info.height)));
	}
	if let Some(rate) = source.info.fps.filter(|_| source.kind == MediaKind::Video) {
		rows = rows.push(fact("Rate", format!("{rate:.2} fps")));
	}
	if let Some(duration) = source.info.duration.filter(|_| source.kind != MediaKind::Image) {
		rows = rows.push(fact("Length", core::timecode::format(duration, fps)));
		rows = rows.push(fact("In", core::timecode::format(instance.source_offset, fps)));
	}
	if source.has_audio() {
		rows = rows.push(fact("Audio", "Yes".to_string()));
	}
	rows.into()
}

fn fill_rows<'a>(fill: [f32; 4], ui: &'a InspectorUi) -> Vec<Element<'a, InspectorMessage>> {
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

// -----------------------------------------------------------------------------
// Effects
// -----------------------------------------------------------------------------

/// Whether the layer draws pixels effects could change.
fn accepts_effects(content: &InstanceContent) -> bool {
	match content {
		InstanceContent::Empty => false,
		InstanceContent::Media(source) => source.kind != MediaKind::Audio,
		_ => true,
	}
}

fn remove_all_effects_button<'a>() -> Element<'a, InspectorMessage> {
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
fn effect_rows<'a>(layer: &LayerTarget<'a>, ui: &'a InspectorUi) -> Element<'a, InspectorMessage> {
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

fn effect_message(id: u32, edit: EffectEdit) -> InspectorMessage {
	InspectorMessage::Effect { id, edit }
}

fn small_button<'a>(
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
fn effect_block<'a>(
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

fn effect_key_state(effect: &LayerEffect, slot: usize, time: f32) -> KeyState {
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
fn effect_key_cluster<'a>(effect: &LayerEffect, param: usize, time: f32) -> Element<'a, InspectorMessage> {
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

fn effect_param_row<'a>(
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
						constants::style::ACCENT_BRIGHT_COLOR
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

#[cfg(test)]
mod tests {
	use super::to_hex;

	#[test]
	fn formats_fill_as_hex() {
		assert_eq!(to_hex([1.0, 0.0, 0.5, 1.0]), "#FF0080");
		assert_eq!(to_hex([2.0, -1.0, 0.0, 1.0]), "#FF0000");
	}
}
