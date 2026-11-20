//! Footage section: source file facts.

use iced::Element;
use iced::widget::{button, column, tooltip};
use nade_constants::icons::{Icon, icon};
use nade_constants::style::{FONT_UI, SPACE_1, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use nade_constants::widgets::{self, button_body_center, mono_label, ui_label};
use nade_domain::{Instance, MediaKind, MediaSource};

use crate::InspectorMessage;
use crate::view::{ROW_HEIGHT, labelled};

pub(crate) fn fit_button<'a>() -> Element<'a, InspectorMessage> {
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

pub(crate) fn footage_rows<'a>(
	source: &'a MediaSource,
	instance: &Instance,
	fps: f32,
) -> Element<'a, InspectorMessage> {
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
		rows = rows.push(fact("Length", nade_core::timecode::format(duration, fps)));
		rows = rows.push(fact("In", nade_core::timecode::format(instance.source_offset, fps)));
	}
	if source.has_audio() {
		rows = rows.push(fact("Audio", "Yes".to_string()));
	}
	rows.into()
}
