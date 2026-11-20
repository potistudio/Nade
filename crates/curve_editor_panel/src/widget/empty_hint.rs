//! Hint shown when the selected property has no keys.

use crate::widget::CurveEditorWidget;
use constants::style::TEXT_MUTED_COLOR;
use iced::{
	Point, Size,
	widget::canvas::{self, Text},
};

impl CurveEditorWidget<'_> {
	pub(super) fn draw_empty_hint(&self, frame: &mut canvas::Frame, size: Size) {
		frame.fill_text(Text {
			content: "No keyframes on this channel — press ◇ to add one at the playhead".into(),
			position: Point::new(size.width * 0.5, size.height * 0.5),
			color: TEXT_MUTED_COLOR,
			size: 11.0.into(),
			align_x: iced::advanced::text::Alignment::Center,
			align_y: iced::alignment::Vertical::Center,
			..Text::default()
		});
	}
}
