//! A layer's page: header followed by its sections.

use iced::Element;
use iced::time::Instant;
use iced::widget::{column, scrollable};
use nade_constants::icons::Icon;
use nade_constants::style::{SPACE_1, SPACE_4};
use nade_constants::widgets;
use nade_domain::MediaKind;

use crate::state::InspectorUi;
use crate::target::LayerTarget;
use crate::view::header::layer_header;
use crate::view::sections::effects::{accepts_effects, effect_rows, remove_all_effects_button};
use crate::view::sections::footage::{fit_button, footage_rows};
use crate::view::sections::layer::{layer_rows, open_comp_button};
use crate::view::sections::section;
use crate::view::sections::solid::solid_rows;
use crate::view::sections::text::text_rows;
use crate::view::sections::transform::{property_row, reset_button};
use crate::{InspectorMessage, PropertyGroup, Section};

pub(crate) fn layer_view<'a>(
	layer: LayerTarget<'a>,
	ui: &'a InspectorUi,
	now: Instant,
) -> Element<'a, InspectorMessage> {
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

	scrollable(sections.padding([SPACE_4, SPACE_4]))
		.style(widgets::scrollable)
		.into()
}
