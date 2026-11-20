//! Layer bars, keyframe diamonds and resize handles.

use crate::widget::{DropPreview, KEYFRAME_RADIUS, TimelineWidget, squircle_rect};
use crate::{TimelineClip, TimelineModel, interaction::TimelineInteraction};
use iced::{
	Color, Point, Rectangle, Size,
	widget::canvas::{self, Path, Stroke},
};
use nade_constants::timeline::{
	colors::{CLIP_HOVERED_OUTLINE, CLIP_RESIZE_HANDLE, CLIP_SELECTED_OUTLINE, CLIP_SHRINKING_OUTLINE},
	*,
};

impl TimelineWidget<'_> {
	pub(super) fn draw_timeline_content(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TIMELINE_BG);

		let reorder_preview = self.reorder_preview;
		let insert_before_idx =
			reorder_preview.map(|(from_idx, to_idx)| if to_idx < from_idx { to_idx } else { to_idx + 1 });

		{
			for (track_index, track) in model.tracks.iter().enumerate() {
				let y_offset = match insert_before_idx {
					Some(insert_idx) if track_index >= insert_idx => TRACK_HEIGHT,
					_ => 0.0,
				};
				let y = rect.y + (track_index as f32 * TRACK_HEIGHT) + state.scroll_offset.y + y_offset;

				if !state.is_track_visible(y, rect) {
					continue;
				}

				let bg_color = if track_index % 2 == 0 {
					colors::TIMELINE_BG_ALT
				} else {
					colors::TIMELINE_BG_ALT2
				};
				frame.fill_rectangle(Point::new(rect.x, y), Size::new(rect.width, TRACK_HEIGHT), bg_color);

				for clip in &track.clips {
					self.draw_clip(
						frame,
						state,
						rect,
						clip,
						y,
						track_index,
						model.is_track_rendered(track_index),
					);
				}

				self.draw_horizontal_line(
					frame,
					rect.x,
					rect.x + rect.width,
					y + TRACK_HEIGHT,
					colors::BORDER_DARK,
				);
			}

			if let Some(preview) = self.drop_preview {
				self.draw_drop_preview(frame, state, rect, preview);
			}

			if let Some(insert_idx) = insert_before_idx {
				let gap_y = rect.y + (insert_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
				frame.fill_rectangle(
					Point::new(rect.x, gap_y),
					Size::new(rect.width, TRACK_HEIGHT),
					Color {
						a: 0.08,
						..colors::SELECTION
					},
				);
			}
		}
	}

	/// Ghost of a dropped-in clip; a new track shows as an insertion line at
	/// the lane's top edge with the ghost straddling it.
	fn draw_drop_preview(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		rect: Rectangle,
		preview: DropPreview,
	) {
		let accent = nade_constants::style::ACCENT_COLOR;
		let lane_y = rect.y + preview.lane as f32 * TRACK_HEIGHT + state.scroll_offset.y;
		let track_y = if preview.insert {
			// 差し込み位置（トラックの境目）にまたがって置く
			let line = Rectangle::new(Point::new(rect.x, lane_y - 1.0), Size::new(rect.width, 2.0));
			frame.fill_rectangle(line.position(), line.size(), accent);
			lane_y - TRACK_HEIGHT * 0.5
		} else {
			lane_y
		};

		let x_start = state.time_to_x(preview.start, rect.x);
		let x_end = state.time_to_x(preview.start + preview.duration, rect.x);
		let ghost = squircle_rect(
			Rectangle {
				x: x_start,
				y: track_y + TRACK_PADDING,
				width: (x_end - x_start).max(1.0),
				height: TRACK_HEIGHT - TRACK_PADDING * 2.0,
			},
			CLIP_CORNER_RADIUS,
		);
		frame.fill(&ghost, Color { a: 0.35, ..accent });
		frame.stroke(&ghost, Stroke::default().with_color(accent).with_width(1.0));
	}

	#[allow(clippy::too_many_arguments)]
	pub(super) fn draw_clip(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		timeline_rect: Rectangle,
		clip: &TimelineClip,
		track_y: f32,
		track_index: usize,
		track_rendered: bool,
	) {
		let x_start = state.time_to_x(clip.start_time, timeline_rect.x);
		let x_end = state.time_to_x(clip.end_time(), timeline_rect.x);

		if x_end < timeline_rect.x || x_start > timeline_rect.x + timeline_rect.width {
			return;
		}

		let clip_rect = Rectangle {
			x: x_start,
			y: track_y + TRACK_PADDING,
			width: (x_end - x_start).max(1.0),
			height: TRACK_HEIGHT - TRACK_PADDING * 2.0,
		};

		let is_selected = state.is_clip_selected(track_index, clip.id);
		let is_hovered = state.is_clip_hovered(track_index, clip.id);
		let is_shrinking = state.alt_shrinking_clip == Some((track_index, clip.id));
		// Duplicate clips (id >= 10000) are shown semi-transparent during drag
		let is_duplicate = clip.id >= 10000;
		let label = Self::clip_base_color(track_index, clip.id);
		let dim = if track_rendered { 1.0 } else { 0.4 };
		let dim = if is_duplicate { dim * 0.5 } else { dim };

		let body = squircle_rect(clip_rect, CLIP_CORNER_RADIUS);
		frame.fill(
			&body,
			Color {
				a: dim,
				..nade_constants::style::mix(label, Color::BLACK, 0.15)
			},
		);

		let outline = if is_selected {
			Stroke::default().with_color(CLIP_SELECTED_OUTLINE).with_width(1.0)
		} else if is_hovered {
			Stroke::default().with_color(CLIP_HOVERED_OUTLINE).with_width(1.0)
		} else {
			Stroke::default()
				.with_color(Color { a: 0.55 * dim, ..label })
				.with_width(1.0)
		};
		frame.stroke(&body, outline);

		if is_shrinking {
			frame.stroke(
				&body,
				Stroke::default().with_color(CLIP_SHRINKING_OUTLINE).with_width(2.0),
			);
		}

		self.draw_keyframes(frame, state, timeline_rect, clip, clip_rect, dim);

		if is_hovered && !is_duplicate {
			self.draw_resize_handles(frame, clip_rect);
		}
	}

	/// クリップ上にキーフレームの菱形を描く
	pub(super) fn draw_keyframes(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		timeline_rect: Rectangle,
		clip: &TimelineClip,
		clip_rect: Rectangle,
		dim: f32,
	) {
		let Some(times) = self.keyframes.get(&clip.id) else {
			return;
		};
		let center_y = clip_rect.y + clip_rect.height * 0.5;
		for &time in times {
			let outside_clip = time < clip.start_time - f32::EPSILON || time > clip.end_time() + f32::EPSILON;
			let dim = if outside_clip { dim * 0.3 } else { dim };
			let x = state.time_to_x(time, timeline_rect.x);
			let on_playhead = (time - self.current_time).abs() < 0.5 / self.fps.max(1.0);
			let r = if on_playhead {
				KEYFRAME_RADIUS + 1.0
			} else {
				KEYFRAME_RADIUS
			};
			let diamond = Path::new(|builder| {
				builder.move_to(Point::new(x, center_y - r));
				builder.line_to(Point::new(x + r, center_y));
				builder.line_to(Point::new(x, center_y + r));
				builder.line_to(Point::new(x - r, center_y));
				builder.close();
			});
			frame.fill(
				&diamond,
				Color {
					a: dim,
					..colors::KEYFRAME
				},
			);
			frame.stroke(
				&diamond,
				Stroke::default()
					.with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.55 * dim))
					.with_width(1.0),
			);
		}
	}

	pub(super) fn draw_resize_handles(&self, frame: &mut canvas::Frame, clip_rect: Rectangle) {
		let height = (clip_rect.height - 8.0).max(4.0);
		for x in [clip_rect.x + 4.0, clip_rect.x + clip_rect.width - 6.0] {
			let handle = squircle_rect(
				Rectangle::new(Point::new(x, clip_rect.y + 4.0), Size::new(2.0, height)),
				1.0,
			);
			frame.fill(&handle, CLIP_RESIZE_HANDLE);
		}
	}

	#[inline]
	pub(super) fn clip_base_color(track_index: usize, _clip_id: usize) -> Color {
		colors::CLIP_PALETTE[track_index % colors::CLIP_PALETTE.len()]
	}
}
