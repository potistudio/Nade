//! Dispatch of widget messages, canvas events, keyboard input and cursor shape.

use crate::interaction::{DragState, TimelineInteraction, TimelineLayout};
use crate::message::{TimelineCanvasEvent, TimelineMessage, TimelineUpdate};
use constants::timeline::*;
use core::TimelineModel;
use iced::{
	Point, Rectangle, Vector,
	keyboard::{self, Key},
	mouse,
};

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
				track_reordered: None,
				visibility_changed: false,
			},
			TimelineMessage::ClipModified => TimelineUpdate {
				playhead_time: None,
				clip_modified: true,
				track_reordered: None,
				visibility_changed: false,
			},
			TimelineMessage::CanvasEvent(event) => self.handle_canvas_event(model, event, current_time),
			// アプリ側でキーフレームを動かす
			TimelineMessage::KeyframeDragged { .. } => TimelineUpdate::default(),
			TimelineMessage::ContextClip { track_index, clip_id } => {
				if !self.is_clip_selected(track_index, clip_id) {
					self.select_clip(track_index, clip_id);
				}
				TimelineUpdate::default()
			}
			TimelineMessage::ReorderTrack { .. } => {
				// Handled via pending_reorder in handle_mouse_release
				TimelineUpdate::default()
			}
			TimelineMessage::ToggleVisible(track_index) => TimelineUpdate {
				playhead_time: None,
				clip_modified: false,
				track_reordered: None,
				visibility_changed: model.toggle_visible(track_index),
			},
			TimelineMessage::ToggleSolo(track_index) => TimelineUpdate {
				playhead_time: None,
				clip_modified: false,
				track_reordered: None,
				visibility_changed: model.toggle_solo(track_index),
			},
		}
	}

	pub(super) fn handle_canvas_event(
		&mut self,
		model: &mut TimelineModel,
		event: TimelineCanvasEvent,
		current_time: f32,
	) -> TimelineUpdate {
		match event {
			TimelineCanvasEvent::MousePressed {
				position,
				bounds,
				modifiers,
			} => {
				self.update_viewport_width(bounds.width);
				self.ctrl_pressed = modifiers.command();
				self.shift_pressed = modifiers.shift();
				self.alt_pressed = modifiers.alt();
				let (_handled, playhead_time, visibility_changed) = self.handle_mouse_press(model, position, bounds);
				TimelineUpdate {
					playhead_time,
					clip_modified: false,
					track_reordered: None,
					visibility_changed,
				}
			}
			TimelineCanvasEvent::MouseReleased => {
				let was_dragging_clip = self.is_dragging_clip();
				let was_duplicating = self.is_duplicating_clip();
				let (_handled, playhead_time) = self.handle_mouse_release(model);
				TimelineUpdate {
					playhead_time,
					clip_modified: was_dragging_clip || was_duplicating,
					track_reordered: None,
					visibility_changed: false,
				}
			}
			TimelineCanvasEvent::MouseMoved { position, bounds } => {
				self.update_viewport_width(bounds.width);
				let timeline_left = TimelineLayout::from_bounds_absolute(bounds).timeline_left();
				let (handled, playhead_time) = self.handle_mouse_move(model, position, timeline_left, bounds);
				TimelineUpdate {
					playhead_time,
					clip_modified: handled && self.is_dragging_clip(),
					track_reordered: None,
					visibility_changed: false,
				}
			}
			TimelineCanvasEvent::MouseWheelScrolled { delta, bounds } => {
				self.scroll_bounds = Some(bounds);
				self.update_viewport_width(bounds.width);
				let layout = TimelineLayout::from_bounds_absolute(bounds);
				let duration = model
					.tracks
					.iter()
					.flat_map(|track| &track.clips)
					.map(|clip| clip.end_time())
					.fold(30.0_f32, f32::max);
				self.scroll_min = Vector::new(
					(layout.content.width - duration * PIXELS_PER_SECOND * self.time_scale).min(0.0),
					(layout.content.height - model.tracks.len() as f32 * TRACK_HEIGHT).min(0.0),
				);
				if self.scroll_input_is_momentum {
					let filter_axis = |position: f32, target: f32, min: f32, delta: f32| {
						if position < min || position > 0.0 || target < min || target > 0.0 {
							0.0
						} else {
							delta
						}
					};
					let delta = match delta {
						mouse::ScrollDelta::Lines { x, y } => mouse::ScrollDelta::Lines {
							x: filter_axis(self.scroll_offset.x, self.scroll_target.x, self.scroll_min.x, x),
							y: filter_axis(self.scroll_offset.y, self.scroll_target.y, self.scroll_min.y, y),
						},
						mouse::ScrollDelta::Pixels { x, y } => mouse::ScrollDelta::Pixels {
							x: filter_axis(self.scroll_offset.x, self.scroll_target.x, self.scroll_min.x, x),
							y: filter_axis(self.scroll_offset.y, self.scroll_target.y, self.scroll_min.y, y),
						},
					};
					self.handle_scroll(delta, current_time);
					return TimelineUpdate::default();
				}
				self.handle_scroll(delta, current_time);
				TimelineUpdate::default()
			}
			TimelineCanvasEvent::KeyPressed { key, modifiers } => {
				let is_alt_key = matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Alt));
				self.update_modifiers(modifiers);
				self.shift_pressed = modifiers.shift();
				if is_alt_key {
					self.alt_pressed = true;
				}
				self.handle_keyboard(key, modifiers, current_time);
				TimelineUpdate::default()
			}
			TimelineCanvasEvent::KeyReleased { key, modifiers } => {
				let is_alt_key = matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Alt));
				if is_alt_key {
					self.alt_pressed = false;
				}
				self.shift_pressed = modifiers.shift();
				self.update_modifiers(modifiers);
				TimelineUpdate::default()
			}
		}
	}

	pub(crate) fn update_modifiers(&mut self, modifiers: keyboard::Modifiers) {
		self.ctrl_pressed = modifiers.command();
		self.alt_pressed = modifiers.alt();
	}

	pub(crate) fn handle_keyboard(&mut self, key: Key, modifiers: keyboard::Modifiers, current_time: f32) -> bool {
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
			DragState::Clip { .. } => {
				return mouse::Interaction::Grabbing;
			}
			DragState::RangeSelect { .. } => {
				return mouse::Interaction::Crosshair;
			}
			DragState::RangeSliderLeft { .. } | DragState::RangeSliderRight { .. } => {
				return mouse::Interaction::ResizingHorizontally;
			}
			DragState::RangeSliderMiddle { .. } => {
				return mouse::Interaction::Grabbing;
			}
			_ => {}
		}

		let layout = TimelineLayout::from_bounds_absolute(bounds);

		if layout.range_slider.contains(pos) {
			return mouse::Interaction::Grab;
		}

		if layout.track_labels.contains(pos) {
			let track_labels_top = layout.track_labels.y;
			let track_index = ((pos.y - track_labels_top - self.scroll_offset.y) / TRACK_HEIGHT).floor() as usize;

			if track_index < model.tracks.len() {
				let track_left = bounds.x;
				let track_y = track_labels_top + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

				if track_visible_btn_rect(track_left, track_y).contains(pos)
					|| track_solo_btn_rect(track_left, track_y).contains(pos)
				{
					return mouse::Interaction::Pointer;
				}
			}
		}

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
