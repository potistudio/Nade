//! Playhead line and head.

use crate::interaction::TimelineInteraction;
use crate::widget::{TimelineWidget, rounded_rect};
use iced::{
	Color, Point, Rectangle,
	widget::canvas::{self, LineCap, Path, Stroke},
};
use nade_constants::timeline::*;

impl TimelineWidget<'_> {
	pub(super) fn draw_playhead(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		ruler_rect: Rectangle,
		timeline_rect: Rectangle,
	) {
		let x = state.time_to_x(self.current_time, timeline_rect.x);

		if x < timeline_rect.x || x > timeline_rect.x + timeline_rect.width {
			return;
		}

		// 線
		let line = Path::line(
			Point::new(x, ruler_rect.y + ruler_rect.height - 4.0),
			Point::new(x, timeline_rect.y + timeline_rect.height),
		);
		frame.stroke(
			&line,
			Stroke::default()
				.with_color(colors::PLAYHEAD)
				.with_width(1.5)
				.with_line_cap(LineCap::Round),
		);

		// ルーラー上のカプセル型ハンドル
		let head = Rectangle {
			x: x - 5.5,
			y: ruler_rect.y + 3.0,
			width: 11.0,
			height: ruler_rect.height - 6.0,
		};
		let head_path = rounded_rect(head, 4.0);
		frame.fill(&head_path, colors::PLAYHEAD);
		frame.stroke(
			&head_path,
			Stroke::default()
				.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.35))
				.with_width(1.0),
		);
		let grip = Path::line(Point::new(x, head.y + 4.0), Point::new(x, head.y + head.height - 4.0));
		frame.stroke(
			&grip,
			Stroke::default()
				.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.85))
				.with_width(1.0)
				.with_line_cap(LineCap::Round),
		);
	}
}
