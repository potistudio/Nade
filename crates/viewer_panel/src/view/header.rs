//! Viewer header: composition title, guides and zoom controls.

use constants::icons::{Icon, icon};
use constants::style::{
	FONT_TINY, FONT_UI, ICON_SIZE, SPACE_1, SPACE_2, SPACE_4, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR,
	TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body_center, mono_label, ui_label};
use iced::widget::{Space, button, container, row, tooltip};
use iced::{Alignment, Element, Length, Size, Vector};

use crate::message::PreviewMessage;
use crate::view::{BAR_HEIGHT, ViewerProps};
use crate::widgets::viewport::Viewport;

pub(super) fn header<'a>(props: &ViewerProps<'a>) -> Element<'a, PreviewMessage> {
	let title: Element<'a, PreviewMessage> = match &props.composition {
		Some(comp) => row![
			icon(Icon::Comp, ICON_SIZE, TEXT_SECONDARY_COLOR),
			ui_label(comp.name, FONT_UI).color(TEXT_PRIMARY_COLOR),
			container(mono_label(format!("{}×{}", comp.width, comp.height), FONT_TINY).color(TEXT_MUTED_COLOR))
				.padding([2, 5])
				.style(widgets::chip),
		]
		.spacing(SPACE_2 + 2.0)
		.align_y(Alignment::Center)
		.into(),
		None => ui_label("No composition", FONT_UI).color(TEXT_MUTED_COLOR).into(),
	};

	let magnification = props.composition.as_ref().map(|comp| {
		Viewport::new(
			props.viewport_size,
			Size::new(comp.width as f32, comp.height as f32),
			props.preview.view_zoom,
			Vector::ZERO,
		)
		.magnification()
	});
	let zoom_label = magnification.map_or_else(|| "—".to_string(), |m| format!("{:.0}%", m * 100.0));

	let tools = row![
		toggle(
			Icon::Grid,
			props.settings.show_grid,
			"Rule-of-Thirds Grid",
			PreviewMessage::ToggleGrid
		),
		toggle(
			Icon::SafeArea,
			props.settings.show_safe_areas,
			"Title/Action Safe",
			PreviewMessage::ToggleSafeAreas
		),
		widgets::vertical_separator(14.0),
		ghost_icon(Icon::Minus, "Zoom Out", PreviewMessage::ZoomBy(1.0 / 1.25)),
		widgets::tip(
			button(button_body_center(mono_label(zoom_label, FONT_TINY + 1.0)))
				.padding(0)
				.width(46)
				.height(BAR_HEIGHT)
				.style(widgets::button_ghost)
				.on_press(PreviewMessage::ActualPixels),
			"Actual Pixels (100%)",
			tooltip::Position::Bottom,
		),
		ghost_icon(Icon::Plus, "Zoom In", PreviewMessage::ZoomBy(1.25)),
		ghost_icon(Icon::Fit, "Fit in Window", PreviewMessage::FitView),
	]
	.spacing(SPACE_1)
	.align_y(Alignment::Center);

	container(
		row![title, Space::new().width(Length::Fill), tools]
			.align_y(Alignment::Center)
			.height(Length::Fill),
	)
	.width(Length::Fill)
	.height(BAR_HEIGHT)
	.padding([0.0, SPACE_4])
	.into()
}

fn toggle<'a>(glyph: Icon, active: bool, label: &'a str, message: PreviewMessage) -> Element<'a, PreviewMessage> {
	let color = if active {
		constants::style::ACCENT_BRIGHT_COLOR
	} else {
		TEXT_SECONDARY_COLOR
	};
	widgets::tip(
		button(button_body_center(icon(glyph, ICON_SIZE, color)))
			.padding(0)
			.width(26)
			.height(BAR_HEIGHT)
			.style(widgets::button_toggle(active))
			.on_press(message),
		label,
		tooltip::Position::Bottom,
	)
}

fn ghost_icon<'a>(glyph: Icon, label: &'a str, message: PreviewMessage) -> Element<'a, PreviewMessage> {
	widgets::tip(
		button(button_body_center(icon(
			glyph,
			ICON_SIZE_SMALL_PLUS,
			TEXT_SECONDARY_COLOR,
		)))
		.padding(0)
		.width(24)
		.height(BAR_HEIGHT)
		.style(widgets::button_ghost)
		.on_press(message),
		label,
		tooltip::Position::Bottom,
	)
}

const ICON_SIZE_SMALL_PLUS: f32 = 13.0;
