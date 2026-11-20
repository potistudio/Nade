//! Playhead line.

use crate::widget::CurveEditorWidget;
use iced::{
	Point, Rectangle, Size,
	widget::canvas::{self, Path, Stroke},
};
use nade_constants::style::ACCENT_COLOR;

impl CurveEditorWidget<'_> {
	pub(super) fn draw_playhead(&self, frame: &mut canvas::Frame, height: f32) {
		let x = self.state.time_to_x(self.current_time);
		frame.stroke(
			&Path::line(Point::new(x, 0.0), Point::new(x, height)),
			Stroke::default().with_color(ACCENT_COLOR).with_width(1.5),
		);
		let head = nade_constants::squircle::path(Rectangle::new(Point::new(x - 5.0, 0.0), Size::new(10.0, 12.0)), 3.0);
		frame.fill(&head, ACCENT_COLOR);
	}
}
