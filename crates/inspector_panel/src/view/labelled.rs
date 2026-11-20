//! A label column followed by a control, aligned with the transform rows.

use constants::style::{FONT_UI, SPACE_2, TEXT_SECONDARY_COLOR};
use constants::widgets::ui_label;
use iced::widget::{Space, row};
use iced::{Alignment, Element};

use crate::InspectorMessage;
use crate::view::{LABEL_WIDTH, ROW_HEIGHT};

pub(crate) fn labelled<'a>(label: &'a str, content: Element<'a, InspectorMessage>) -> Element<'a, InspectorMessage> {
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
