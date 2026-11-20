//! Placeholder shown when nothing is selected.

use constants::icons::{Icon, icon};
use constants::style::{FONT_TITLE, FONT_UI, SPACE_3, SPACE_5, TEXT_MUTED_COLOR, TEXT_SECONDARY_COLOR};
use constants::widgets::ui_label;
use iced::widget::{column, container};
use iced::{Alignment, Element, Length};

use crate::InspectorMessage;

pub(crate) fn empty_state<'a>() -> Element<'a, InspectorMessage> {
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
