//! Clip and track hit testing.

use crate::interaction::TimelineInteraction;
use constants::timeline::*;
use core::{TimelineClip, TimelineModel};
use iced::{Point, Rectangle};

impl TimelineInteraction {
	pub(super) fn find_clips_in_rect(
		&self,
		model: &TimelineModel,
		sel_rect: Rectangle,
		bounds: Rectangle,
	) -> Vec<(usize, usize)> {
		let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
		let timeline_top = bounds.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT;
		let mut result = Vec::new();

		for (track_index, track) in model.tracks.iter().enumerate() {
			let track_y = timeline_top + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;
			let clip_top = track_y + TRACK_PADDING;
			let clip_bottom = track_y + TRACK_HEIGHT - TRACK_PADDING;

			if sel_rect.y + sel_rect.height < clip_top || sel_rect.y > clip_bottom {
				continue;
			}

			for clip in &track.clips {
				let x_start = self.time_to_x(clip.start_time, timeline_left);
				let x_end = self.time_to_x(clip.end_time(), timeline_left);

				if sel_rect.x < x_end && sel_rect.x + sel_rect.width > x_start {
					result.push((track_index, clip.id));
				}
			}
		}

		result
	}

	pub(crate) fn find_clip_at(&self, model: &TimelineModel, pos: Point, bounds: Rectangle) -> Option<(usize, usize)> {
		let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
		let timeline_top = bounds.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT;

		for (track_index, track) in model.tracks.iter().enumerate() {
			let track_y = timeline_top + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

			if pos.y < track_y + TRACK_PADDING || pos.y > track_y + TRACK_HEIGHT - TRACK_PADDING {
				continue;
			}

			for clip in &track.clips {
				let x_start = self.time_to_x(clip.start_time, timeline_left);
				let x_end = self.time_to_x(clip.end_time(), timeline_left);

				if pos.x >= x_start && pos.x <= x_end {
					return Some((track_index, clip.id));
				}
			}
		}
		None
	}

	pub(crate) fn find_clip(model: &TimelineModel, track_id: usize, clip_id: usize) -> Option<&TimelineClip> {
		model
			.tracks
			.get(track_id)
			.and_then(|track| track.clips.iter().find(|clip| clip.id == clip_id))
	}

	pub(crate) fn find_clip_mut(
		model: &mut TimelineModel,
		track_id: usize,
		clip_id: usize,
	) -> Option<&mut TimelineClip> {
		model
			.tracks
			.get_mut(track_id)
			.and_then(|track| track.clips.iter_mut().find(|clip| clip.id == clip_id))
	}

	pub(crate) fn is_clip_selected(&self, track_index: usize, clip_id: usize) -> bool {
		self.selected_clip == Some((track_index, clip_id)) || self.selected_clips.contains(&(track_index, clip_id))
	}

	pub(crate) fn is_clip_hovered(&self, track_index: usize, clip_id: usize) -> bool {
		self.hovered_clip == Some((track_index, clip_id))
	}

	#[inline]
	pub(crate) fn is_track_visible(&self, track_y: f32, rect: Rectangle) -> bool {
		track_y + TRACK_HEIGHT >= rect.y && track_y <= rect.y + rect.height
	}
}
