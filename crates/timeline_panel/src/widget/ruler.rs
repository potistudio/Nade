//! Time ruler, work area and composition end.

use crate::interaction::TimelineInteraction;
use crate::widget::TimelineWidget;
use constants::style::MONOSPACE_FONT;
use constants::timeline::*;
use iced::{
	Color, Point, Rectangle, Size,
	widget::canvas::{self, Text},
};

/// Minimum spacing between labelled ruler ticks (px).
pub(super) const MIN_LABEL_SPACING: f32 = 76.0;

/// Subdivisions between labelled ticks.
pub(super) const RULER_SUBDIVISIONS: u32 = 5;

impl TimelineWidget<'_> {
	/// Ruler tick spacing in seconds: the smallest "nice" step whose labels
	/// stay at least [`MIN_LABEL_SPACING`] apart.
	pub(super) fn label_step(&self) -> f32 {
		let pixels_per_second = PIXELS_PER_SECOND * self.state.time_scale;
		let frame = 1.0 / self.fps.max(1.0);
		[
			frame,
			frame * 2.0,
			frame * 5.0,
			frame * 10.0,
			0.5,
			1.0,
			2.0,
			5.0,
			10.0,
			30.0,
			60.0,
		]
		.into_iter()
		.find(|step| step * pixels_per_second >= MIN_LABEL_SPACING)
		.unwrap_or(120.0)
	}

	pub(super) fn draw_ruler(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		rect: Rectangle,
		lanes: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RULER_BG);
		self.draw_horizontal_line(frame, rect.x, rect.x + rect.width, rect.y + rect.height, colors::BORDER);

		let step = self.label_step();
		let minor = step / RULER_SUBDIVISIONS as f32;
		let visible_start = state.x_to_time(rect.x, rect.x).max(0.0);
		let visible_end = state.x_to_time(rect.x + rect.width, rect.x);
		let first = (visible_start / minor).floor() as i64;
		let last = (visible_end / minor).ceil() as i64;

		for index in first.max(0)..=last {
			let time = index as f32 * minor;
			let x = state.time_to_x(time, rect.x);
			if x < rect.x || x > rect.x + rect.width {
				continue;
			}

			let is_major = index % RULER_SUBDIVISIONS as i64 == 0;
			let (tick_height, tick_color) = if is_major {
				(8.0, colors::TICK_MAJOR)
			} else {
				(4.0, colors::TICK_MINOR)
			};
			self.draw_vertical_line(
				frame,
				x,
				rect.y + rect.height - tick_height,
				rect.y + rect.height,
				tick_color,
			);

			if is_major {
				self.draw_vertical_line(frame, x, lanes.y, lanes.y + lanes.height, colors::GRID_LINE);
				frame.fill_text(Text {
					content: core::timecode::format_ruler(time, self.fps),
					position: Point::new(x + 4.0, rect.y + 4.0),
					color: colors::TEXT_SECONDARY,
					size: 9.5.into(),
					font: MONOSPACE_FONT,
					..Text::default()
				});
			}
		}
	}

	/// ワークエリア: ルーラー下端のバーと、範囲外の薄い陰
	pub(super) fn draw_work_area(&self, frame: &mut canvas::Frame, ruler: Rectangle, lanes: Rectangle) {
		let Some([start, end]) = self.work_area else {
			return;
		};
		let left = self.state.time_to_x(start, lanes.x).max(lanes.x);
		let right = self.state.time_to_x(end, lanes.x).min(lanes.x + lanes.width);
		if right <= left {
			return;
		}
		let full = self
			.duration
			.is_none_or(|duration| start <= 1e-4 && end >= duration - 1e-4);
		if !full {
			let shade = Color::from_rgba(0.0, 0.0, 0.0, 0.18);
			let top = ruler.y + ruler.height;
			let bottom = lanes.y + lanes.height;
			if left > lanes.x {
				frame.fill_rectangle(Point::new(lanes.x, top), Size::new(left - lanes.x, bottom - top), shade);
			}
			let lanes_right = lanes.x + lanes.width;
			if right < lanes_right {
				frame.fill_rectangle(
					Point::new(right, top),
					Size::new(lanes_right - right, bottom - top),
					shade,
				);
			}
		}
		let accent = constants::style::ACCENT_COLOR;
		frame.fill_rectangle(
			Point::new(left, ruler.y + ruler.height - 4.0),
			Size::new(right - left, 3.0),
			Color { a: 0.75, ..accent },
		);
	}

	/// コンポジション終端より後ろを暗くし、終端に線を引く
	pub(super) fn draw_composition_end(&self, frame: &mut canvas::Frame, ruler: Rectangle, lanes: Rectangle) {
		let Some(duration) = self.duration else {
			return;
		};
		let x = self.state.time_to_x(duration, lanes.x);
		if x >= lanes.x + lanes.width {
			return;
		}
		let start = x.max(lanes.x);
		frame.fill_rectangle(
			Point::new(start, ruler.y),
			Size::new(lanes.x + lanes.width - start, lanes.y + lanes.height - ruler.y),
			Color::from_rgba(0.0, 0.0, 0.0, 0.32),
		);
		if x >= lanes.x {
			self.draw_vertical_line(
				frame,
				x,
				ruler.y,
				lanes.y + lanes.height,
				Color::from_rgba(1.0, 1.0, 1.0, 0.18),
			);
		}
	}
}
