//! Range slider for the visible time span.

use crate::widget::{TimelineWidget, rounded_rect};
use crate::{TimelineModel, interaction::TimelineInteraction};
use constants::timeline::*;
use iced::{
	Point, Rectangle,
	widget::canvas::{self, Path, Stroke},
};

impl TimelineWidget<'_> {
	pub(super) fn draw_range_slider(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RANGE_SLIDER_BG);

		let padding = 3.0_f32;
		let track_rect = Rectangle {
			x: rect.x + padding,
			y: rect.y + padding,
			width: (rect.width - padding * 2.0).max(0.0),
			height: rect.height - padding * 2.0,
		};
		frame.fill(&rounded_rect(track_rect, track_rect.height), colors::RANGE_SLIDER_TRACK);

		let total_duration = state.total_timeline_duration(model);
		if total_duration <= 0.0 || track_rect.width <= 0.0 {
			return;
		}

		let norm_start = (state.visible_start_time() / total_duration).clamp(0.0, 1.0);
		let norm_end = (state.visible_end_time() / total_duration).clamp(0.0, 1.0);
		let handle_l = track_rect.x + norm_start * track_rect.width;
		let handle_r = (track_rect.x + norm_end * track_rect.width).max(handle_l + 4.0);

		let handle = rounded_rect(
			Rectangle {
				x: handle_l,
				width: handle_r - handle_l,
				..track_rect
			},
			track_rect.height,
		);
		frame.fill(&handle, colors::RANGE_SLIDER_HANDLE);
		frame.stroke(
			&handle,
			Stroke::default()
				.with_color(colors::RANGE_SLIDER_HANDLE_BORDER)
				.with_width(1.0),
		);

		// Keep both resize grips inside the handle, even at its minimum width.
		let inset = ((handle_r - handle_l) * 0.25).min(4.0);
		let grip_radius = 2.0_f32.min(inset);
		let grip_y = track_rect.y + track_rect.height * 0.5;
		for x in [handle_l + inset, handle_r - inset] {
			frame.fill(
				&Path::circle(Point::new(x, grip_y), grip_radius),
				colors::RANGE_SLIDER_GRIP,
			);
		}
	}
}
