//! View, selection and drag state, and the screen / curve mapping.

use crate::HIT_RADIUS;
use crate::helpers::distance;
use crate::message::{CurveCanvasEvent, CurveEditorMessage, CurveEditorUpdate};
use core::{AnimationCurve, HandleSide, Transform};
use domain::{TransformAnimation, TransformProperty};
use iced::{Point, Size, Vector, mouse};

#[derive(Debug, Clone)]
pub(crate) enum DragState {
	Key(u64),
	Handle(u64, HandleSide),
	Playhead,
	Pan(Point),
}

/// View and selection state. Animation data remains in the domain model.
#[derive(Debug, Clone)]
pub struct CurveEditorState {
	pub selected_property: TransformProperty,
	pub selected_key: Option<u64>,
	pub(crate) x_scale: f32,
	pub(crate) y_scale: f32,
	offset: Vector,
	viewport: Size,
	drag: Option<DragState>,
}

impl Default for CurveEditorState {
	fn default() -> Self {
		Self {
			selected_property: TransformProperty::PositionX,
			selected_key: None,
			x_scale: 100.0,
			y_scale: 1.0,
			offset: Vector::new(20.0, 0.0),
			viewport: Size::new(800.0, 360.0),
			drag: None,
		}
	}
}

impl CurveEditorState {
	pub fn apply_message(
		&mut self,
		animation: &mut TransformAnimation,
		base: &Transform,
		message: CurveEditorMessage,
		current_time: f32,
	) -> CurveEditorUpdate {
		let mut update = CurveEditorUpdate::default();
		match message {
			CurveEditorMessage::SelectProperty(property) => {
				self.selected_property = property;
				self.selected_key = None;
			}
			CurveEditorMessage::AddKey => {
				let curve = animation.curve_mut(self.selected_property);
				let value = curve.evaluate(self.selected_property.value(base), current_time);
				self.selected_key = Some(curve.add_key(current_time, value));
				update.curve_changed = true;
			}
			CurveEditorMessage::DeleteSelected => {
				if let Some(id) = self.selected_key.take() {
					update.curve_changed = animation.curve_mut(self.selected_property).remove_key(id);
				}
			}
			CurveEditorMessage::SetInterpolation(interpolation) => {
				if let Some(id) = self.selected_key {
					update.curve_changed = animation
						.curve_mut(self.selected_property)
						.set_interpolation(id, interpolation);
				}
			}
			CurveEditorMessage::FrameAll => self.frame_all(animation),
			CurveEditorMessage::CanvasEvent(event) => {
				update = self.apply_canvas_event(animation, event, current_time);
			}
		}
		update
	}

	fn apply_canvas_event(
		&mut self,
		animation: &mut TransformAnimation,
		event: CurveCanvasEvent,
		current_time: f32,
	) -> CurveEditorUpdate {
		let mut update = CurveEditorUpdate::default();
		match event {
			CurveCanvasEvent::MousePressed {
				position,
				button,
				bounds,
			} => {
				self.viewport = bounds;
				match button {
					mouse::Button::Middle => self.drag = Some(DragState::Pan(position)),
					mouse::Button::Left => {
						let curve = animation.curve(self.selected_property);
						if let Some((id, side)) = self.hit_handle(curve, position) {
							self.selected_key = Some(id);
							self.drag = Some(DragState::Handle(id, side));
						} else if let Some(id) = self.hit_key(curve, position) {
							self.selected_key = Some(id);
							self.drag = Some(DragState::Key(id));
						} else if (self.time_to_x(current_time) - position.x).abs() <= HIT_RADIUS {
							self.drag = Some(DragState::Playhead);
						} else {
							self.selected_key = None;
							let time = self.x_to_time(position.x).max(0.0);
							self.drag = Some(DragState::Playhead);
							update.playhead_time = Some(time);
						}
					}
					_ => {}
				}
			}
			CurveCanvasEvent::MouseReleased(button) => {
				let ends_drag = matches!(
					(button, &self.drag),
					(
						mouse::Button::Left,
						Some(DragState::Key(_) | DragState::Handle(_, _) | DragState::Playhead)
					) | (mouse::Button::Middle, Some(DragState::Pan(_)))
				);
				if ends_drag {
					self.drag = None;
				}
			}
			CurveCanvasEvent::MouseMoved { position, bounds } => {
				self.viewport = bounds;
				match self.drag.clone() {
					Some(DragState::Key(id)) => {
						let [time, value] = self.screen_to_curve(position);
						update.curve_changed =
							animation
								.curve_mut(self.selected_property)
								.set_key(id, time.max(0.0), value);
					}
					Some(DragState::Handle(id, side)) => {
						let [time, value] = self.screen_to_curve(position);
						update.curve_changed = animation
							.curve_mut(self.selected_property)
							.set_handle(id, side, time, value);
					}
					Some(DragState::Playhead) => {
						update.playhead_time = Some(self.x_to_time(position.x).max(0.0));
					}
					Some(DragState::Pan(previous)) => {
						self.offset.x += position.x - previous.x;
						self.offset.y += position.y - previous.y;
						self.drag = Some(DragState::Pan(position));
					}
					None => {}
				}
			}
			CurveCanvasEvent::MouseWheelScrolled {
				position,
				delta,
				bounds,
			} => {
				self.viewport = bounds;
				let amount = match delta {
					mouse::ScrollDelta::Lines { y, .. } => y,
					mouse::ScrollDelta::Pixels { y, .. } => y / 30.0,
				};
				self.zoom_at(position, amount);
			}
			CurveCanvasEvent::DeletePressed => {
				if let Some(id) = self.selected_key.take() {
					update.curve_changed = animation.curve_mut(self.selected_property).remove_key(id);
				}
			}
		}
		update
	}

	fn frame_all(&mut self, animation: &TransformAnimation) {
		let mut bounds: Option<([f32; 2], [f32; 2])> = None;
		for property in TransformProperty::ALL {
			if let Some((min, max)) = animation.curve(property).bounds() {
				bounds = Some(match bounds {
					Some((all_min, all_max)) => (
						[all_min[0].min(min[0]), all_min[1].min(min[1])],
						[all_max[0].max(max[0]), all_max[1].max(max[1])],
					),
					None => (min, max),
				});
			}
		}
		let Some((min, max)) = bounds else {
			return;
		};
		let width = (self.viewport.width - 40.0).max(100.0);
		let height = (self.viewport.height - 40.0).max(100.0);
		self.x_scale = (width / (max[0] - min[0]).max(1.0)).clamp(10.0, 1000.0);
		self.y_scale = (height / (max[1] - min[1]).max(1.0)).clamp(0.01, 1000.0);
		self.offset.x = 20.0 - min[0] * self.x_scale;
		self.offset.y = (min[1] + max[1]) * 0.5 * self.y_scale;
	}

	fn zoom_at(&mut self, cursor: Point, amount: f32) {
		let before = self.screen_to_curve(cursor);
		let factor = (1.0 + amount * 0.12).clamp(0.25, 4.0);
		self.x_scale = (self.x_scale * factor).clamp(5.0, 5000.0);
		self.y_scale = (self.y_scale * factor).clamp(0.005, 5000.0);
		let after = self.curve_to_screen(before[0], before[1]);
		self.offset.x += cursor.x - after.x;
		self.offset.y += cursor.y - after.y;
	}

	pub(crate) fn time_to_x(&self, time: f32) -> f32 {
		time * self.x_scale + self.offset.x
	}

	pub(crate) fn x_to_time(&self, x: f32) -> f32 {
		(x - self.offset.x) / self.x_scale
	}

	pub(crate) fn curve_to_screen(&self, time: f32, value: f32) -> Point {
		Point::new(
			self.time_to_x(time),
			self.viewport.height * 0.5 + self.offset.y - value * self.y_scale,
		)
	}

	pub(crate) fn screen_to_curve(&self, point: Point) -> [f32; 2] {
		[
			self.x_to_time(point.x),
			(self.viewport.height * 0.5 + self.offset.y - point.y) / self.y_scale,
		]
	}

	fn hit_key(&self, curve: &AnimationCurve, point: Point) -> Option<u64> {
		curve
			.keys()
			.iter()
			.find(|key| distance(self.curve_to_screen(key.time, key.value), point) <= HIT_RADIUS)
			.map(|key| key.id)
	}

	fn hit_handle(&self, curve: &AnimationCurve, point: Point) -> Option<(u64, HandleSide)> {
		let id = self.selected_key?;
		let key = curve.key(id)?;
		[HandleSide::Left, HandleSide::Right].into_iter().find_map(|side| {
			let handle = key.handle_position(side);
			(distance(self.curve_to_screen(handle[0], handle[1]), point) <= HIT_RADIUS).then_some((id, side))
		})
	}
}
