//! Status bar: playback state, render rate and comp summary.

use constants::style::{
	FONT_TINY, FONT_UI, SPACE_2, SPACE_4, SPACE_5, SUCCESS_COLOR, TEXT_MUTED_COLOR, TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, mono_label, ui_label};
use iced::widget::{Space, container, row};
use iced::{Alignment, Border, Color, Element, Length};

use crate::message::Message;
use crate::theme::{SHELL_GUTTER, STATUS_BAR_HEIGHT};
use crate::viewer::CompositionSummary;

pub struct StatusInfo<'a> {
	pub playing: bool,
	/// Breathing phase of the playback dot (`0.0..=1.0`).
	pub pulse: f32,
	pub render_fps: f32,
	pub composition: Option<CompositionSummary<'a>>,
}

pub fn view<'a>(info: StatusInfo<'a>) -> Element<'a, Message> {
	let dot_color = if info.playing {
		Color {
			a: 0.55 + 0.45 * info.pulse,
			..SUCCESS_COLOR
		}
	} else {
		TEXT_MUTED_COLOR
	};

	let dot = container(Space::new().width(7).height(7)).style(move |_| container::Style {
		background: Some(dot_color.into()),
		border: Border {
			radius: 3.5.into(),
			..Default::default()
		},
		..Default::default()
	});

	let mut left = row![dot].spacing(SPACE_2 + 2.0).align_y(Alignment::Center);
	// 停止中はシーク時にしか描画しないため、実測レートは再生中のみ意味がある
	if info.playing {
		left = left
			.push(widgets::vertical_separator(12.0))
			.push(mono_label(format!("{:.1}", info.render_fps), FONT_TINY + 1.0).color(TEXT_SECONDARY_COLOR))
			.push(ui_label("fps rendered", FONT_TINY + 1.0).color(TEXT_MUTED_COLOR));
	}

	let right: Element<'a, Message> = match info.composition {
		Some(comp) => row![
			ui_label(comp.name, FONT_UI).color(TEXT_SECONDARY_COLOR),
			chip(format!("{}×{}", comp.width, comp.height)),
			chip(format!("{:.0} fps", comp.fps)),
			chip(core::timecode::format(comp.duration, comp.fps)),
		]
		.spacing(SPACE_2)
		.align_y(Alignment::Center)
		.into(),
		None => Space::new().into(),
	};

	container(
		row![left, Space::new().width(Length::Fill), right]
			.spacing(SPACE_5)
			.align_y(Alignment::Center),
	)
	.width(Length::Fill)
	.height(STATUS_BAR_HEIGHT)
	.padding([0.0, SHELL_GUTTER + SPACE_4])
	.center_y(STATUS_BAR_HEIGHT)
	.into()
}

fn chip<'a>(label: String) -> Element<'a, Message> {
	container(mono_label(label, FONT_TINY + 1.0).color(TEXT_SECONDARY_COLOR))
		.padding([2, 6])
		.style(widgets::chip)
		.into()
}
