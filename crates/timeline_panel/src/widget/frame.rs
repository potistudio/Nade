//! Overall layout of the canvas, the corner timecode, the range-select rectangle and line helpers.

use crate::interaction::TimelineLayout;
use crate::widget::{TimelineWidget, rounded_rect};
use constants::style::MONOSPACE_FONT;
use constants::timeline::*;
use iced::{
	Color, Point, Rectangle, Size,
	alignment::Vertical,
	widget::canvas::{self, Path, Stroke, Text},
};

impl TimelineWidget<'_> {
	pub(super) fn draw_timeline(&self, frame: &mut canvas::Frame, bounds: Size) {
		let layout = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, bounds));

		frame.fill_rectangle(Point::ORIGIN, bounds, colors::BACKGROUND);

		self.draw_timeline_content(frame, self.state, self.model, layout.content);
		self.draw_composition_end(frame, layout.ruler, layout.content);
		self.draw_ruler(frame, self.state, layout.ruler, layout.content);
		self.draw_work_area(frame, layout.ruler, layout.content);
		self.draw_range_slider(frame, self.state, self.model, layout.range_slider);
		self.draw_track_labels(frame, self.state, self.model, layout.track_labels);
		self.draw_corner(
			frame,
			Rectangle::new(Point::ORIGIN, Size::new(TRACK_LABEL_WIDTH, layout.content.y)),
		);
		self.draw_playhead(frame, self.state, layout.ruler, layout.content);
		self.draw_selection_rect(frame);
	}

	/// 左上: 現在時刻のタイムコード（AE のタイムコード表示に相当）
	pub(super) fn draw_corner(&self, frame: &mut canvas::Frame, rect: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RULER_BG);
		frame.fill_text(Text {
			content: core::timecode::format(self.current_time, self.fps),
			position: Point::new(rect.x + 10.0, rect.y + rect.height * 0.5),
			color: constants::style::ACCENT_BRIGHT_COLOR,
			size: 14.0.into(),
			font: MONOSPACE_FONT,
			align_y: Vertical::Center,
			..Text::default()
		});
		frame.fill_text(Text {
			content: format!("{:.0} fps", self.fps),
			position: Point::new(rect.x + rect.width - 10.0, rect.y + rect.height * 0.5),
			color: colors::TEXT_MUTED,
			size: 9.0.into(),
			align_x: iced::advanced::text::Alignment::Right,
			align_y: Vertical::Center,
			..Text::default()
		});
		self.draw_vertical_line(frame, rect.x + rect.width, rect.y, rect.y + rect.height, colors::BORDER);
		self.draw_horizontal_line(frame, rect.x, rect.x + rect.width, rect.y + rect.height, colors::BORDER);
	}

	pub(super) fn draw_selection_rect(&self, frame: &mut canvas::Frame) {
		let Some(sel_rect) = self.state.selection_rect else {
			return;
		};

		let origin = self.state.canvas_origin;
		let local_rect = Rectangle {
			x: sel_rect.x - origin.x,
			y: sel_rect.y - origin.y,
			width: sel_rect.width,
			height: sel_rect.height,
		};

		let path = rounded_rect(local_rect, 3.0);
		frame.fill(&path, colors::SELECTION_FILL);
		frame.stroke(
			&path,
			Stroke::default().with_color(colors::SELECTION_STROKE).with_width(1.0),
		);
	}

	pub(super) fn draw_horizontal_line(&self, frame: &mut canvas::Frame, x1: f32, x2: f32, y: f32, color: Color) {
		let line = Path::line(Point::new(x1, y), Point::new(x2, y));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(1.0));
	}

	pub(super) fn draw_vertical_line(&self, frame: &mut canvas::Frame, x: f32, y1: f32, y2: f32, color: Color) {
		let line = Path::line(Point::new(x, y1), Point::new(x, y2));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(1.0));
	}
}
