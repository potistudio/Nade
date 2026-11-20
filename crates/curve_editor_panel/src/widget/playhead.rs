//! Playhead line.

use crate::widget::CurveEditorWidget;
use constants::style::ACCENT_COLOR;
use iced::{
	Point, Size,
	widget::canvas::{self, Path, Stroke},
};

impl CurveEditorWidget<'_> {
	pub(super) fn draw_playhead(&self, frame: &mut canvas::Frame, height: f32) {
		let x = self.state.time_to_x(self.current_time);
		frame.stroke(
			&Path::line(Point::new(x, 0.0), Point::new(x, height)),
			Stroke::default().with_color(ACCENT_COLOR).with_width(1.5),
		);
		let head = Path::rounded_rectangle(Point::new(x - 5.0, 0.0), Size::new(10.0, 12.0), 3.0.into());
		frame.fill(&head, ACCENT_COLOR);
	}
}
