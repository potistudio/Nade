//! Layer header: kind badge, editable name and span.

use iced::widget::{column, container, row, text_input};
use iced::{Alignment, Color, Element};
use nade_constants::icons::{Icon, icon};
use nade_constants::style::{FONT_TINY, FONT_TITLE, SPACE_1, SPACE_3, TEXT_MUTED_COLOR};
use nade_constants::widgets::{self, ui_label};
use nade_domain::{InstanceContent, MediaKind};

use crate::InspectorMessage;
use crate::target::LayerTarget;
use crate::view::ROW_HEIGHT;

pub(crate) fn badge<'a>(glyph: Icon, color: Color) -> Element<'a, InspectorMessage> {
	container(icon(glyph, 15.0, color))
		.width(30)
		.height(ROW_HEIGHT)
		.center_y(ROW_HEIGHT)
		.center_x(30)
		.style(widgets::chip_tinted(color))
		.into()
}

pub(crate) fn layer_header<'a>(layer: &LayerTarget<'a>) -> Element<'a, InspectorMessage> {
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
