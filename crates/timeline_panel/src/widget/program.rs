//! Canvas program: input handling and keyframe dragging.

use crate::widget::{KEYFRAME_RADIUS, TimelineWidget};
use crate::{
	interaction::TimelineLayout,
	message::{TimelineCanvasEvent, TimelineMessage},
};
use constants::timeline::*;
use iced::{
	Event, Point, Rectangle, Size, Theme,
	keyboard::{self},
	mouse,
	widget::canvas::{self, Geometry},
};

/// Canvas-local input state.
#[derive(Debug, Default)]
pub struct CanvasState {
	modifiers: keyboard::Modifiers,
	key_drag: Option<KeyDrag>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct KeyDrag {
	clip_id: usize,
	origin: f32,
	last: f32,
}

impl TimelineWidget<'_> {
	/// Keyframe diamond under `point` (canvas-local): `(clip id, key time)`.
	pub(super) fn keyframe_at(&self, point: Point, size: Size) -> Option<(usize, f32)> {
		let layout = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, size));
		let lanes = layout.content;
		if !lanes.contains(point) {
			return None;
		}
		let reach = KEYFRAME_RADIUS + 2.0;
		self.model.tracks.iter().enumerate().find_map(|(track_index, track)| {
			let center_y =
				lanes.y + track_index as f32 * TRACK_HEIGHT + self.state.scroll_offset.y + TRACK_HEIGHT * 0.5;
			if (point.y - center_y).abs() > reach + 1.0 {
				return None;
			}
			track.clips.iter().find_map(|clip| {
				let times = self.keyframes.get(&clip.id)?;
				times
					.iter()
					.copied()
					.filter(|time| (self.state.time_to_x(*time, lanes.x) - point.x).abs() <= reach)
					.min_by(|a, b| {
						let distance = |time: f32| (self.state.time_to_x(time, lanes.x) - point.x).abs();
						distance(*a).total_cmp(&distance(*b))
					})
					.map(|time| (clip.id, time))
			})
		})
	}

	/// Pointer time snapped to whole frames (canvas-local x).
	pub(super) fn snapped_time(&self, x: f32, size: Size) -> f32 {
		let layout = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, size));
		let fps = self.fps.max(1.0);
		(self.state.x_to_time(x, layout.content.x) * fps).round().max(0.0) / fps
	}
}

impl canvas::Program<TimelineMessage> for TimelineWidget<'_> {
	type State = CanvasState;

	fn draw(
		&self,
		_canvas_state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		self.draw_timeline(&mut frame, bounds.size());
		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		canvas_state: &mut Self::State,
		event: &Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Option<canvas::Action<TimelineMessage>> {
		let cursor_pos = cursor
			.position_in(bounds)
			.map(|pos| Point::new(bounds.x + pos.x, bounds.y + pos.y));

		// キーフレームの菱形をドラッグして時間を変える（クリップのドラッグより優先）
		if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
			&& let Some(local) = cursor.position_in(bounds)
			&& let Some((clip_id, time)) = self.keyframe_at(local, bounds.size())
		{
			canvas_state.key_drag = Some(KeyDrag {
				clip_id,
				origin: time,
				last: time,
			});
			return Some(canvas::Action::capture());
		}
		if let Some(drag) = canvas_state.key_drag {
			match event {
				Event::Mouse(mouse::Event::CursorMoved { position }) => {
					let time = self.snapped_time(position.x - bounds.x, bounds.size());
					if (time - drag.last).abs() < 1e-5 {
						return Some(canvas::Action::capture());
					}
					canvas_state.key_drag = Some(KeyDrag { last: time, ..drag });
					return Some(
						canvas::Action::publish(TimelineMessage::KeyframeDragged {
							clip_id: drag.clip_id,
							origin: drag.origin,
							time,
						})
						.and_capture(),
					);
				}
				Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
					canvas_state.key_drag = None;
					return Some(canvas::Action::capture());
				}
				_ => {}
			}
		}

		match event {
			Event::Mouse(mouse_event) => match mouse_event {
				mouse::Event::ButtonPressed(mouse::Button::Left) => cursor_pos.map(|position| {
					canvas::Action::publish(TimelineMessage::CanvasEvent(TimelineCanvasEvent::MousePressed {
						position,
						bounds,
						modifiers: canvas_state.modifiers,
					}))
				}),
				mouse::Event::ButtonReleased(mouse::Button::Left) => Some(canvas::Action::publish(
					TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseReleased),
				)),
				mouse::Event::ButtonPressed(mouse::Button::Right) => cursor_pos
					.and_then(|position| self.state.find_clip_at(self.model, position, bounds))
					.map(|(track_index, clip_id)| {
						canvas::Action::publish(TimelineMessage::ContextClip { track_index, clip_id })
					}),
				mouse::Event::CursorMoved { .. } => cursor_pos.map(|position| {
					canvas::Action::publish(TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseMoved {
						position,
						bounds,
					}))
				}),
				mouse::Event::WheelScrolled { delta } if cursor_pos.is_some() => Some(canvas::Action::publish(
					TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseWheelScrolled { delta: *delta, bounds }),
				)),
				_ => None,
			},
			Event::Keyboard(key_event) => match key_event {
				keyboard::Event::ModifiersChanged(modifiers) => {
					canvas_state.modifiers = *modifiers;
					None
				}
				keyboard::Event::KeyPressed { key, modifiers, .. } => {
					canvas_state.modifiers = *modifiers;
					Some(canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::KeyPressed {
							key: key.clone(),
							modifiers: *modifiers,
						},
					)))
				}
				keyboard::Event::KeyReleased { key, modifiers, .. } => {
					canvas_state.modifiers = *modifiers;
					Some(canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::KeyReleased {
							key: key.clone(),
							modifiers: *modifiers,
						},
					)))
				}
			},
			_ => None,
		}
	}

	fn mouse_interaction(
		&self,
		canvas_state: &Self::State,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> mouse::Interaction {
		if canvas_state.key_drag.is_some() {
			return mouse::Interaction::ResizingHorizontally;
		}
		if cursor
			.position_in(bounds)
			.is_some_and(|local| self.keyframe_at(local, bounds.size()).is_some())
		{
			return mouse::Interaction::Pointer;
		}
		cursor
			.position_in(bounds)
			.map(|pos| {
				self.state
					.get_mouse_interaction(self.model, Point::new(bounds.x + pos.x, bounds.y + pos.y), bounds)
			})
			.unwrap_or_default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		TimelineClip, TimelineModel,
		interaction::{TimelineInteraction, TimelineLayout},
	};
	use iced::{Point, Rectangle, Size};
	use std::collections::HashMap;

	use crate::TimelineTrack;
	use crate::widget::TimelineWidget;

	#[test]
	fn finds_keyframe_diamonds_under_the_pointer() {
		let state = TimelineInteraction::new();
		let mut model = TimelineModel::new();
		let mut track = TimelineTrack::new("Layer");
		track.add_clip(TimelineClip::new(7, "Layer", 0.0, 5.0));
		model.add_track(track);
		let widget = TimelineWidget::new(&state, &model)
			.frame_rate(10.0)
			.keyframes(HashMap::from([(7, vec![1.0, 2.0])]));

		let size = Size::new(900.0, 300.0);
		let lanes = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, size)).content;
		let y = lanes.y + TRACK_HEIGHT * 0.5;
		let x = state.time_to_x(2.0, lanes.x);
		assert_eq!(widget.keyframe_at(Point::new(x + 1.0, y), size), Some((7, 2.0)));
		assert_eq!(widget.keyframe_at(Point::new(x + 30.0, y), size), None);
		assert_eq!(widget.keyframe_at(Point::new(x, y + TRACK_HEIGHT), size), None);
		// フレーム単位にスナップ
		let snapped = widget.snapped_time(state.time_to_x(1.23, lanes.x), size);
		assert!((snapped - 1.2).abs() < 1e-5, "{snapped}");
	}
}
