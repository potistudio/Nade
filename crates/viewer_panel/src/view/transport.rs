//! Transport bar: timecode, playback controls and resolution.

use constants::icons::{Icon, icon};
use constants::style::{
	FONT_TINY, FONT_UI, ICON_SIZE, SPACE_1, SPACE_2, SPACE_4, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR,
	TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body_center, mono_label, ui_label};
use iced::widget::{Canvas, Space, button, container, pick_list, row, stack, tooltip};
use iced::{Alignment, Element, Length};

use crate::message::{PreviewMessage, TransportMessage};
use crate::settings::Resolution;
use crate::view::{TRANSPORT_HEIGHT, ViewerProps};
use crate::widgets::play_glyph::PlayGlyph;

pub(super) fn transport<'a>(props: &ViewerProps<'a>) -> Element<'a, PreviewMessage> {
	let fps = props.composition.as_ref().map_or(30.0, |comp| comp.fps);
	let duration = props.composition.as_ref().map_or(0.0, |comp| comp.duration);
	let playing = props.preview.is_playing;

	let timecode = row![
		mono_label(core::timecode::format(props.preview.time, fps), FONT_UI).color(TEXT_PRIMARY_COLOR),
		mono_label(format!("/ {}", core::timecode::format(duration, fps)), FONT_TINY + 1.0).color(TEXT_MUTED_COLOR),
	]
	.spacing(SPACE_2 + 2.0)
	.align_y(Alignment::Center);

	let play = widgets::tip(
		button(button_body_center(
			Canvas::new(PlayGlyph {
				pause_amount: if playing { 1.0 } else { 0.0 },
				color: TEXT_PRIMARY_COLOR,
			})
			.width(18)
			.height(18),
		))
		.padding(0)
		.width(40)
		.height(TRANSPORT_HEIGHT)
		.style(widgets::button_tool)
		.on_press(PreviewMessage::Transport(TransportMessage::TogglePlay)),
		if playing { "Pause  Space" } else { "Play  Space" },
		tooltip::Position::Top,
	);

	let controls = row![
		transport_button(Icon::SkipStart, "Go to Start  Home", TransportMessage::GoToStart),
		transport_button(Icon::StepBack, "Previous Frame  ←", TransportMessage::StepBackward),
		play,
		transport_button(Icon::StepForward, "Next Frame  →", TransportMessage::StepForward),
		transport_button(Icon::SkipEnd, "Go to End  End", TransportMessage::GoToEnd),
		Space::new().width(SPACE_2),
		widgets::tip(
			button(button_body_center(icon(
				Icon::Loop,
				ICON_SIZE,
				if props.loop_playback {
					constants::style::ACCENT_BRIGHT_COLOR
				} else {
					TEXT_SECONDARY_COLOR
				},
			)))
			.padding(0)
			.width(30)
			.height(TRANSPORT_HEIGHT)
			.style(widgets::button_toggle(props.loop_playback))
			.on_press(PreviewMessage::Transport(TransportMessage::ToggleLoop)),
			"Loop Playback",
			tooltip::Position::Top,
		),
	]
	.spacing(SPACE_1)
	.align_y(Alignment::Center);

	let resolution = row![
		ui_label("Resolution", FONT_TINY + 1.0).color(TEXT_MUTED_COLOR),
		pick_list(
			Resolution::ALL,
			Some(props.settings.resolution),
			PreviewMessage::SetResolution
		)
		.text_size(FONT_UI)
		.padding([2, 8])
		.width(86)
		.style(widgets::pick_list)
		.menu_style(widgets::menu),
	]
	.spacing(SPACE_2 + 2.0)
	.align_y(Alignment::Center);

	container(
		stack![
			row![timecode, Space::new().width(Length::Fill), resolution]
				.align_y(Alignment::Center)
				.height(Length::Fill),
			container(controls)
				.width(Length::Fill)
				.height(Length::Fill)
				.center_x(Length::Fill)
				.center_y(Length::Fill),
		]
		.width(Length::Fill)
		.height(Length::Fill),
	)
	.width(Length::Fill)
	.height(TRANSPORT_HEIGHT)
	.padding([0.0, SPACE_4 + 2.0])
	.into()
}

fn transport_button<'a>(glyph: Icon, label: &'a str, message: TransportMessage) -> Element<'a, PreviewMessage> {
	widgets::tip(
		button(button_body_center(icon(glyph, ICON_SIZE, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(30)
			.height(TRANSPORT_HEIGHT)
			.style(widgets::button_ghost)
			.on_press(PreviewMessage::Transport(message)),
		label,
		tooltip::Position::Top,
	)
}
