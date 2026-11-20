//! Zoom buttons and scale readout below the timeline.

use crate::message::TimelineMessage;
use crate::widget::TimelineWidget;
use iced::{
	Alignment, Element, Length,
	widget::{Space, button, container, row},
};

impl TimelineWidget<'_> {
	pub(super) fn zoom_control_view(time_scale: f32) -> Element<'static, TimelineMessage> {
		use constants::icons::{Icon, icon};
		use constants::style::{FONT_TINY, SPACE_1, SPACE_4, TEXT_SECONDARY_COLOR, TOOLBAR_HEIGHT};
		use constants::widgets;

		let icon_button = |glyph: Icon, label: &'static str, message: TimelineMessage| {
			widgets::tip(
				button(widgets::button_body_center(icon(glyph, 13.0, TEXT_SECONDARY_COLOR)))
					.padding(0)
					.width(TOOLBAR_HEIGHT)
					.height(TOOLBAR_HEIGHT)
					.style(widgets::button_ghost)
					.on_press(message),
				label,
				iced::widget::tooltip::Position::Top,
			)
		};

		let zoom_label = widgets::mono_label(format!("{:.0}%", time_scale * 100.0), FONT_TINY + 1.0)
			.color(TEXT_SECONDARY_COLOR)
			.width(40);

		let controls = row![
			Space::new().width(Length::Fill),
			icon_button(Icon::Minus, "Zoom Out  −", TimelineMessage::ZoomOut),
			icon_button(Icon::Plus, "Zoom In  +", TimelineMessage::ZoomIn),
			zoom_label,
			icon_button(Icon::Fit, "Reset Zoom", TimelineMessage::ResetZoom),
		]
		.spacing(SPACE_1 + 2.0)
		.align_y(Alignment::Center);

		container(controls)
			.padding([0.0, SPACE_4])
			.width(Length::Fill)
			.height(TOOLBAR_HEIGHT)
			.center_y(TOOLBAR_HEIGHT)
			.style(widgets::toolbar)
			.into()
	}
}
