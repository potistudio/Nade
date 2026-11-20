//! Time / value grid with labels.

use crate::helpers::{format_value, grid_step};
use crate::widget::CurveEditorWidget;
use iced::{
	Color, Point, Size,
	widget::canvas::{self, Path, Stroke, Text},
};
use nade_constants::style::{MONOSPACE_FONT, TEXT_MUTED_COLOR};

impl CurveEditorWidget<'_> {
	pub(super) fn draw_grid(&self, frame: &mut canvas::Frame, size: Size) {
		let time_step = grid_step(self.state.x_scale, 80.0);
		let value_step = grid_step(self.state.y_scale, 48.0);
		let grid = Stroke::default()
			.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.04))
			.with_width(1.0);

		let min_time = self.state.x_to_time(0.0).floor();
		let max_time = self.state.x_to_time(size.width).ceil();
		let mut time = (min_time / time_step).floor() * time_step;
		while time <= max_time {
			let x = self.state.time_to_x(time);
			frame.stroke(&Path::line(Point::new(x, 0.0), Point::new(x, size.height)), grid);
			frame.fill_text(Text {
				content: format!("{time:.1}s"),
				position: Point::new(x + 4.0, 4.0),
				color: TEXT_MUTED_COLOR,
				size: 9.5.into(),
				font: MONOSPACE_FONT,
				..Text::default()
			});
			time += time_step;
		}

		let top_value = self.state.screen_to_curve(Point::new(0.0, 0.0))[1];
		let bottom_value = self.state.screen_to_curve(Point::new(0.0, size.height))[1];
		let mut value = (bottom_value / value_step).floor() * value_step;
		while value <= top_value {
			let y = self.state.curve_to_screen(0.0, value).y;
			let stroke = if value.abs() < value_step * 0.01 {
				Stroke::default()
					.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.12))
					.with_width(1.0)
			} else {
				grid
			};
			frame.stroke(&Path::line(Point::new(0.0, y), Point::new(size.width, y)), stroke);
			frame.fill_text(Text {
				content: format_value(value, value_step),
				position: Point::new(4.0, y - 13.0),
				color: TEXT_MUTED_COLOR,
				size: 9.5.into(),
				font: MONOSPACE_FONT,
				..Text::default()
			});
			value += value_step;
		}
	}
}
