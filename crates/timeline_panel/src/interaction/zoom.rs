//! Time scale (zoom) changes.

use crate::interaction::TimelineInteraction;
use nade_constants::timeline::*;

impl TimelineInteraction {
	/// Set the timeline zoom scale
	/// and clamp to the minimum and maximum scale
	pub fn set_zoom_scale(&mut self, scale: f32) {
		self.time_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
	}

	/// ズームをカーソル位置中心で行う
	///
	/// `factor` = 1.0 より大きければズームイン、小さければズームアウト。
	/// カーソル下の時間位置が画面上で動かないようにスクロールオフセットを調整する。
	pub fn zoom_at_cursor_x(&mut self, factor: f32) {
		let new_scale = (self.time_scale * factor).clamp(MIN_SCALE, MAX_SCALE);
		if (self.time_scale - new_scale).abs() < f32::EPSILON {
			return;
		}

		let cursor = self.cursor_abs_x;
		let tl = self.timeline_left_abs;

		let time_at_cursor = (cursor - tl - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale);
		self.time_scale = new_scale;
		self.scroll_offset.x = cursor - tl - time_at_cursor * PIXELS_PER_SECOND * new_scale;
		self.clamp_scroll_offset();
	}

	/// Set the timeline zoom scale, keeping the playhead centered
	pub fn set_zoom_scale_centered(&mut self, scale: f32, current_time: f32) {
		let new_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
		if (self.time_scale - new_scale).abs() < f32::EPSILON {
			return;
		}

		self.time_scale = new_scale;

		let content_center_x = (self.viewport_width - TRACK_LABEL_WIDTH) / 2.0;
		let new_offset_x = content_center_x - (current_time * PIXELS_PER_SECOND * self.time_scale);

		self.scroll_offset.x = new_offset_x;
		self.clamp_scroll_offset();
	}

	/// Zoom in the timeline by 1.2x
	/// and clamp to the maximum scale
	pub fn zoom_in(&mut self, current_time: f32) {
		self.set_zoom_scale_centered(self.time_scale * 1.2, current_time);
	}

	/// Zoom out the timeline by 1.2x
	/// and clamp to the minimum scale
	pub fn zoom_out(&mut self, current_time: f32) {
		self.set_zoom_scale_centered(self.time_scale / 1.2, current_time);
	}

	/// Reset the timeline zoom scale to 1.0
	pub fn reset_zoom(&mut self) {
		self.set_zoom_scale(1.0);
	}
}
