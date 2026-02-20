//! タイムラインインタラクション状態の定義

use iced::{
	Point, Rectangle, Vector,
	keyboard::{self, Key},
	mouse,
};

use constants::timeline::*;
use nade_core::{TimelineClip, TimelineModel};

use crate::widget::{TimelineCanvasEvent, TimelineMessage, TimelineUpdate};

#[derive(Clone, Debug, Default)]
pub(crate) enum DragState {
	#[default]
	None,
	Playhead,
	Clip {
		track_id: usize,
		clip_id: usize,
		offset: f32,
	},
	ClipResizeLeft {
		track_id: usize,
		clip_id: usize,
		original_start: f32,
		original_duration: f32,
	},
	ClipResizeRight {
		track_id: usize,
		clip_id: usize,
	},
	Panning {
		start_offset: Vector,
		start_cursor: Point,
	},
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TimelineLayout {
	pub ruler: Rectangle,
	pub track_labels: Rectangle,
	pub content: Rectangle,
}

impl TimelineLayout {
	pub fn from_bounds(bounds: Rectangle) -> Self {
		Self {
			ruler: Rectangle {
				x: TRACK_LABEL_WIDTH,
				y: 0.0,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RULER_HEIGHT,
			},
			track_labels: Rectangle {
				x: 0.0,
				y: RULER_HEIGHT,
				width: TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
			content: Rectangle {
				x: TRACK_LABEL_WIDTH,
				y: RULER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
		}
	}

	pub fn from_bounds_absolute(bounds: Rectangle) -> Self {
		Self {
			ruler: Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RULER_HEIGHT,
			},
			track_labels: Rectangle {
				x: bounds.x,
				y: bounds.y + RULER_HEIGHT,
				width: TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
			content: Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y + RULER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
		}
	}

	pub fn timeline_left(&self) -> f32 {
		self.content.x
	}
}

/// タイムラインのUIインタラクション状態
#[derive(Debug)]
pub struct TimelineInteraction {
	pub scroll_offset: Vector,
	pub time_scale: f32,
	pub selected_clip: Option<(usize, usize)>,
	pub(crate) drag_state: DragState,
	pub(crate) ctrl_pressed: bool,
	pub(crate) viewport_width: f32,
}

impl Default for TimelineInteraction {
	fn default() -> Self {
		Self {
			scroll_offset: Vector::ZERO,
			time_scale: 1.0,
			selected_clip: None,
			drag_state: DragState::None,
			ctrl_pressed: false,
			viewport_width: 800.0,
		}
	}
}

impl TimelineInteraction {
	/// Is the playhead being dragged?
	#[inline]
	pub fn is_dragging_playhead(&self) -> bool {
		matches!(self.drag_state, DragState::Playhead)
	}

	/// 新しいタイムライン状態を作成（空）
	pub fn new() -> Self {
		Self::default()
	}

	/// 外部モデル更新後の選択状態をリセット
	pub fn clear_selection(&mut self) {
		self.selected_clip = None;
		self.drag_state = DragState::None;
	}

	/// Convert a time value to an x coordinate.
	#[inline]
	pub(crate) fn time_to_x(&self, time: f32, timeline_left: f32) -> f32 {
		timeline_left + (time * PIXELS_PER_SECOND * self.time_scale) + self.scroll_offset.x
	}

	/// Convert an x coordinate to a time value.
	#[inline]
	pub(crate) fn x_to_time(&self, x: f32, timeline_left: f32) -> f32 {
		((x - timeline_left) - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale)
	}

	pub(crate) fn find_clip_at(
		&self,
		model: &TimelineModel,
		pos: Point,
		bounds: Rectangle,
	) -> Option<(usize, usize)> {
		let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
		let timeline_top = bounds.y + RULER_HEIGHT;

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

	pub(crate) fn find_clip(
		model: &TimelineModel,
		track_id: usize,
		clip_id: usize,
	) -> Option<&TimelineClip> {
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
		self.selected_clip == Some((track_index, clip_id))
	}

	/// Clamp the scroll offset to valid bounds
	pub fn clamp_scroll_offset(&mut self) {
		self.scroll_offset.x = self.scroll_offset.x.min(0.0);
		self.scroll_offset.y = self.scroll_offset.y.min(0.0);
	}

	/// Set the position of the timeline offset
	pub fn set_scroll_position(&mut self, x: f32, y: f32) {
		self.scroll_offset.x = x;
		self.scroll_offset.y = y;
		self.clamp_scroll_offset();
	}

	/// Apply a scroll delta to the timeline offset
	/// and clamp to the minimum values
	pub fn apply_scroll_delta(&mut self, x: f32, y: f32) {
		self.scroll_offset.x += x;
		self.scroll_offset.y += y;
		self.clamp_scroll_offset();
	}

	/// Update the viewport width
	pub fn update_viewport_width(&mut self, width: f32) {
		self.viewport_width = width;
	}

	/// Set the timeline zoom scale
	/// and clamp to the minimum and maximum scale
	pub fn set_zoom_scale(&mut self, scale: f32) {
		self.time_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
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

	#[inline]
	pub(crate) fn is_track_visible(&self, track_y: f32, rect: Rectangle) -> bool {
		track_y + TRACK_HEIGHT >= rect.y && track_y <= rect.y + rect.height
	}

	pub(crate) fn calculate_tick_interval(&self) -> f32 {
		match self.time_scale {
			s if s < 0.3 => 5.0,
			s if s < 1.0 => 2.0,
			s if s < 3.0 => 1.0,
			_ => 0.5,
		}
	}
}

impl TimelineInteraction {
	/// タイムラインメッセージを状態に適用
	pub fn apply_message(
		&mut self,
		model: &mut TimelineModel,
		message: TimelineMessage,
		current_time: f32,
	) -> TimelineUpdate {
		match message {
			TimelineMessage::ZoomIn => {
				self.zoom_in(current_time);
				TimelineUpdate::default()
			}
			TimelineMessage::ZoomOut => {
				self.zoom_out(current_time);
				TimelineUpdate::default()
			}
			TimelineMessage::ZoomChanged(scale) => {
				self.set_zoom_scale_centered(scale, current_time);
				TimelineUpdate::default()
			}
			TimelineMessage::ResetZoom => {
				self.reset_zoom();
				TimelineUpdate::default()
			}
			TimelineMessage::PlayheadChanged(time) => TimelineUpdate {
				playhead_time: Some(time),
				clip_modified: false,
			},
			TimelineMessage::ClipModified => TimelineUpdate {
				playhead_time: None,
				clip_modified: true,
			},
			TimelineMessage::CanvasEvent(event) => {
				self.handle_canvas_event(model, event, current_time)
			}
		}
	}

	fn handle_canvas_event(
		&mut self,
		model: &mut TimelineModel,
		event: TimelineCanvasEvent,
		current_time: f32,
	) -> TimelineUpdate {
		match event {
			TimelineCanvasEvent::MousePressed { position, bounds } => {
				self.update_viewport_width(bounds.width);
				let (_handled, playhead_time) = self.handle_mouse_press(model, position, bounds);
				TimelineUpdate {
					playhead_time,
					clip_modified: false,
				}
			}
			TimelineCanvasEvent::MouseReleased => {
				let was_dragging_clip = self.is_dragging_clip();
				let (_handled, playhead_time) = self.handle_mouse_release();
				TimelineUpdate {
					playhead_time,
					clip_modified: was_dragging_clip,
				}
			}
			TimelineCanvasEvent::MouseMoved { position, bounds } => {
				self.update_viewport_width(bounds.width);
				let timeline_left = TimelineLayout::from_bounds_absolute(bounds).timeline_left();
				let (handled, playhead_time) =
					self.handle_mouse_move(model, position, timeline_left);
				TimelineUpdate {
					playhead_time,
					clip_modified: handled && self.is_dragging_clip(),
				}
			}
			TimelineCanvasEvent::MouseWheelScrolled { delta, bounds } => {
				self.update_viewport_width(bounds.width);
				self.handle_scroll(delta, current_time);
				TimelineUpdate::default()
			}
			TimelineCanvasEvent::KeyPressed { key, modifiers } => {
				self.update_modifiers(modifiers);
				self.handle_keyboard(key, modifiers, current_time);
				TimelineUpdate::default()
			}
			TimelineCanvasEvent::KeyReleased { modifiers } => {
				self.update_modifiers(modifiers);
				TimelineUpdate::default()
			}
		}
	}

	/// Is a clip being dragged?
	#[inline]
	pub fn is_dragging_clip(&self) -> bool {
		matches!(
			self.drag_state,
			DragState::Clip { .. }
				| DragState::ClipResizeLeft { .. }
				| DragState::ClipResizeRight { .. }
		)
	}

	/// マウスプレス時の処理
	///
	/// プレイヘッドドラッグ開始時は開始時間を返す
	/// Returns (handled, playhead_time)
	pub(crate) fn handle_mouse_press(
		&mut self,
		model: &TimelineModel,
		pos: Point,
		bounds: Rectangle,
	) -> (bool, Option<f32>) {
		let layout = TimelineLayout::from_bounds_absolute(bounds);

		if layout.ruler.contains(pos) {
			let time = self.x_to_time(pos.x, layout.timeline_left()).max(0.0);
			self.drag_state = DragState::Playhead;
			return (true, Some(time));
		}

		if layout.content.contains(pos) {
			if let Some((track_id, clip_id)) = self.find_clip_at(model, pos, bounds) {
				self.selected_clip = Some((track_id, clip_id));
				self.start_clip_drag(model, pos, track_id, clip_id, layout.timeline_left());
				return (true, None);
			}

			self.selected_clip = None;
			self.drag_state = DragState::Panning {
				start_offset: self.scroll_offset,
				start_cursor: pos,
			};
			return (true, None);
		}

		(false, None)
	}

	fn start_clip_drag(
		&mut self,
		model: &TimelineModel,
		pos: Point,
		track_id: usize,
		clip_id: usize,
		timeline_left: f32,
	) {
		if let Some(clip) = Self::find_clip(model, track_id, clip_id) {
			let x_start = self.time_to_x(clip.start_time, timeline_left);
			let x_end = self.time_to_x(clip.end_time(), timeline_left);

			self.drag_state = if pos.x < x_start + RESIZE_HANDLE_WIDTH {
				DragState::ClipResizeLeft {
					track_id,
					clip_id,
					original_start: clip.start_time,
					original_duration: clip.duration,
				}
			} else if pos.x > x_end - RESIZE_HANDLE_WIDTH {
				DragState::ClipResizeRight { track_id, clip_id }
			} else {
				let time = self.x_to_time(pos.x, timeline_left);
				DragState::Clip {
					track_id,
					clip_id,
					offset: clip.start_time - time,
				}
			};
		}
	}

	pub(crate) fn handle_mouse_move(
		&mut self,
		model: &mut TimelineModel,
		pos: Point,
		timeline_left: f32,
	) -> (bool, Option<f32>) {
		match self.drag_state.clone() {
			DragState::Playhead => {
				let time = self.x_to_time(pos.x, timeline_left).max(0.0);
				(true, Some(time))
			}
			DragState::Clip {
				track_id,
				clip_id,
				offset,
			} => {
				let time = self.x_to_time(pos.x, timeline_left) + offset;
				if let Some(clip) = Self::find_clip_mut(model, track_id, clip_id) {
					clip.start_time = time.max(0.0);
				}
				(true, None)
			}
			DragState::ClipResizeLeft {
				track_id,
				clip_id,
				original_start,
				original_duration,
			} => {
				let time = self.x_to_time(pos.x, timeline_left).max(0.0);
				if let Some(clip) = Self::find_clip_mut(model, track_id, clip_id) {
					let delta = time - original_start;
					let new_duration = (original_duration - delta).max(MIN_CLIP_DURATION);
					clip.start_time = original_start + original_duration - new_duration;
					clip.duration = new_duration;
				}
				(true, None)
			}
			DragState::ClipResizeRight { track_id, clip_id } => {
				let end_time = self.x_to_time(pos.x, timeline_left);
				if let Some(clip) = Self::find_clip_mut(model, track_id, clip_id) {
					clip.duration = (end_time - clip.start_time).max(MIN_CLIP_DURATION);
				}
				(true, None)
			}
			DragState::Panning {
				start_offset,
				start_cursor,
			} => {
				let delta = pos - start_cursor;
				self.scroll_offset = start_offset + delta;
				self.clamp_scroll_offset();
				(true, None)
			}
			DragState::None => (false, None),
		}
	}

	/// マウスリリース時の処理
	///
	/// プレイヘッドドラッグ終了時は最終時間を返す
	pub(crate) fn handle_mouse_release(&mut self) -> (bool, Option<f32>) {
		if !matches!(self.drag_state, DragState::None) {
			self.drag_state = DragState::None;
			return (true, None);
		}
		(false, None)
	}

	pub(crate) fn handle_scroll(&mut self, delta: mouse::ScrollDelta, current_time: f32) -> bool {
		if self.ctrl_pressed {
			let dy = match delta {
				mouse::ScrollDelta::Lines { y, .. } => y * SCROLL_MULTIPLIER,
				mouse::ScrollDelta::Pixels { y, .. } => y,
			};

			if dy > 0.0 {
				self.zoom_in(current_time);
			} else if dy < 0.0 {
				self.zoom_out(current_time);
			}
		} else {
			match delta {
				mouse::ScrollDelta::Lines { x, y } => {
					self.apply_scroll_delta(x * SCROLL_MULTIPLIER, y * SCROLL_MULTIPLIER);
				}
				mouse::ScrollDelta::Pixels { x, y } => {
					self.apply_scroll_delta(x, y);
				}
			}
		}
		true
	}

	pub(crate) fn update_modifiers(&mut self, modifiers: keyboard::Modifiers) {
		self.ctrl_pressed = modifiers.command();
	}

	pub(crate) fn handle_keyboard(
		&mut self,
		key: Key,
		modifiers: keyboard::Modifiers,
		current_time: f32,
	) -> bool {
		if modifiers.command() {
			match key {
				Key::Character(ref c) if c == "=" || c == "+" => {
					self.zoom_in(current_time);
					return true;
				}
				Key::Character(ref c) if c == "-" => {
					self.zoom_out(current_time);
					return true;
				}
				_ => {}
			}
		}
		false
	}

	pub(crate) fn get_mouse_interaction(
		&self,
		model: &TimelineModel,
		pos: Point,
		bounds: Rectangle,
	) -> mouse::Interaction {
		match &self.drag_state {
			DragState::ClipResizeLeft { .. } | DragState::ClipResizeRight { .. } => {
				return mouse::Interaction::ResizingHorizontally;
			}
			DragState::Panning { .. } | DragState::Clip { .. } => {
				return mouse::Interaction::Grabbing;
			}
			_ => {}
		}

		let layout = TimelineLayout::from_bounds_absolute(bounds);

		if layout.content.contains(pos)
			&& let Some((track_id, clip_id)) = self.find_clip_at(model, pos, bounds)
			&& let Some(clip) = Self::find_clip(model, track_id, clip_id)
		{
			let x_start = self.time_to_x(clip.start_time, layout.timeline_left());
			let x_end = self.time_to_x(clip.end_time(), layout.timeline_left());

			if pos.x < x_start + RESIZE_HANDLE_WIDTH || pos.x > x_end - RESIZE_HANDLE_WIDTH {
				return mouse::Interaction::ResizingHorizontally;
			}
			return mouse::Interaction::Grab;
		}

		mouse::Interaction::default()
	}
}
