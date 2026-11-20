//! Status bar: playback state, render rate and comp summary.

use constants::style::{
	FONT_TINY, FONT_UI, SPACE_2, SPACE_4, SPACE_5, SUCCESS_COLOR, TEXT_MUTED_COLOR, TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, mono_label, ui_label};
use iced::widget::{Space, container, row};
use iced::{Alignment, Border, Color, Element, Length};

use crate::message::Message;
use crate::theme::{SHELL_GUTTER, STATUS_BAR_HEIGHT};
use core::CompositionSummary;

pub struct StatusInfo<'a> {
	pub playing: bool,
	/// Progress of a running export (`0.0..=1.0`).
	pub export: Option<f32>,
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

	if let Some(progress) = info.export {
		let track = container(
			container(
				Space::new()
					.width(Length::Fixed(120.0 * progress.clamp(0.0, 1.0)))
					.height(4),
			)
			.style(|_| container::Style {
				background: Some(constants::style::ACCENT_COLOR.into()),
				border: Border {
					radius: 2.0.into(),
					..Default::default()
				},
				..Default::default()
			}),
		)
		.width(120)
		.height(4)
		.style(|_| container::Style {
			background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.1).into()),
			border: Border {
				radius: 2.0.into(),
				..Default::default()
			},
			..Default::default()
		});
		left = left
			.push(widgets::vertical_separator(12.0))
			.push(ui_label("Exporting", FONT_TINY + 1.0).color(TEXT_SECONDARY_COLOR))
			.push(track)
			.push(mono_label(format!("{:.0}%", progress * 100.0), FONT_TINY + 1.0).color(TEXT_SECONDARY_COLOR))
			.push(
				iced::widget::button(ui_label("Cancel", FONT_TINY + 1.0).color(TEXT_SECONDARY_COLOR))
					.padding([0, 6])
					.style(widgets::button_ghost)
					.on_press(Message::Shell(crate::message::ShellMessage::CancelExport)),
			);
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
