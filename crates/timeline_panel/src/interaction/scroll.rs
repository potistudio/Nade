//! Scroll offset, elastic overscroll and the momentum spring.

use crate::interaction::TimelineInteraction;
use constants::timeline::*;
use core::TimelineModel;
use iced::{Rectangle, Vector, mouse};

impl TimelineInteraction {
	/// Clamp the scroll offset to valid bounds
	pub fn clamp_scroll_offset(&mut self) {
		self.scroll_offset.x = self.scroll_offset.x.min(0.0);
		self.scroll_offset.y = self.scroll_offset.y.min(0.0);
		self.scroll_target = self.scroll_offset;
		self.scroll_velocity = Vector::ZERO;
		self.last_scroll = None;
		self.last_spring_tick = None;
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

	/// 現在のビューポートで見えている先頭の時間
	pub fn visible_start_time(&self) -> f32 {
		(-self.scroll_offset.x / (PIXELS_PER_SECOND * self.time_scale)).max(0.0)
	}

	/// 現在のビューポートで見えている末尾の時間
	pub fn visible_end_time(&self) -> f32 {
		let content_w = (self.viewport_width - TRACK_LABEL_WIDTH).max(0.0);
		self.visible_start_time() + content_w / (PIXELS_PER_SECOND * self.time_scale)
	}

	/// タイムライン全体の長さ（クリップの末尾と表示末尾の大きい方、最低30秒）
	pub fn total_timeline_duration(&self, model: &TimelineModel) -> f32 {
		let max_clip = model
			.tracks
			.iter()
			.flat_map(|t| t.clips.iter())
			.map(|c| c.end_time())
			.fold(0.0f32, f32::max);
		max_clip.max(self.visible_end_time()).max(30.0)
	}
}

impl TimelineInteraction {
	pub(super) fn apply_elastic_scroll(&mut self, x: f32, y: f32) {
		fn advance(position: f32, delta: f32, min: f32) -> f32 {
			let proposed = position + delta;
			let edge = proposed.clamp(min, 0.0);
			if proposed == edge {
				proposed
			} else {
				let previous = position - position.clamp(min, 0.0);
				let excess = proposed - edge;
				let resisted = if previous * delta > 0.0 || previous == 0.0 {
					// Integrate resistance that approaches zero at an 80px excursion.
					// The offset approaches the limit smoothly instead of being clamped.
					const SPRING_REACH: f32 = 80.0;
					let distance = previous.abs();
					let outward = (excess.abs() - distance).max(0.0);
					let resistance = (1.0 - distance / SPRING_REACH).powi(3);
					let travel = (SPRING_REACH - distance) * -(-outward * resistance / SPRING_REACH).exp_m1();
					excess.signum() * (distance + travel)
				} else {
					excess
				};
				edge + resisted
			}
		}
		let now = std::time::Instant::now();
		if self.last_scroll.is_none() {
			self.scroll_target = self.scroll_offset;
			self.last_spring_tick = Some(now);
		}
		self.scroll_target.x = advance(self.scroll_target.x, x, self.scroll_min.x);
		self.scroll_target.y = advance(self.scroll_target.y, y, self.scroll_min.y);
		// Inside the bounds, retain direct input. Outside, only the spring writes
		// the displayed offset; wheel events modify its target, never its position.
		fn direct(position: &mut f32, target: f32, velocity: &mut f32, min: f32) {
			if *position >= min && *position <= 0.0 && target >= min && target <= 0.0 {
				*position = target;
				*velocity = 0.0;
			}
		}
		direct(
			&mut self.scroll_offset.x,
			self.scroll_target.x,
			&mut self.scroll_velocity.x,
			self.scroll_min.x,
		);
		direct(
			&mut self.scroll_offset.y,
			self.scroll_target.y,
			&mut self.scroll_velocity.y,
			self.scroll_min.y,
		);
		if self.scroll_gesture_active {
			self.scroll_offset = self.scroll_target;
			self.scroll_velocity = Vector::ZERO;
		}
		self.last_scroll = Some(now);
	}

	pub fn set_scroll_gesture_active(&mut self, active: bool) {
		if active == self.scroll_gesture_active {
			return;
		}
		self.scroll_gesture_active = active;
		self.scroll_velocity = Vector::ZERO;
		self.last_spring_tick = Some(std::time::Instant::now());
		if active && self.last_scroll.is_some() {
			self.scroll_offset = self.scroll_target;
		}
	}

	pub fn set_scroll_input_is_momentum(&mut self, momentum: bool) {
		self.scroll_input_is_momentum = momentum;
	}

	pub fn scroll_bounds(&self) -> Option<Rectangle> {
		self.scroll_bounds
	}

	pub fn is_scroll_animating(&self) -> bool {
		!self.scroll_gesture_active
			&& self.last_scroll.is_some()
			&& (self.scroll_offset.x < self.scroll_min.x
				|| self.scroll_offset.x > 0.0
				|| self.scroll_offset.y < self.scroll_min.y
				|| self.scroll_offset.y > 0.0
				|| (self.scroll_target.x - self.scroll_offset.x).abs() > 0.001
				|| (self.scroll_target.y - self.scroll_offset.y).abs() > 0.001
				|| self.scroll_velocity.x.abs() > 0.1
				|| self.scroll_velocity.y.abs() > 0.1)
	}

	pub fn tick_scroll_spring(&mut self, now: std::time::Instant) {
		if self.scroll_gesture_active || self.last_scroll.is_none() {
			return;
		}
		let previous = self.last_spring_tick.replace(now).unwrap_or(now);
		let dt = now.saturating_duration_since(previous).as_secs_f32().min(0.032);
		fn step(position: &mut f32, target: &mut f32, velocity: &mut f32, min: f32, dt: f32) {
			let edge = target.clamp(min, 0.0);
			// Consume input once as a velocity impulse, not a moving spring target.
			// Only the spring integrates the displayed position.
			if *target != edge {
				*velocity += (*target - *position) * 24.0;
			}
			// Analytic critically damped spring aimed directly at the edge.
			let displacement = *position - edge;
			let coefficient = *velocity + 24.0 * displacement;
			let decay = (-24.0 * dt).exp();
			*position = edge + (displacement + coefficient * dt) * decay;
			*velocity = (*velocity - 24.0 * coefficient * dt) * decay;
			if (*position - edge).abs() < 0.01 && velocity.abs() < 0.1 {
				*position = edge;
				*velocity = 0.0;
			}
			*target = *position;
		}
		step(
			&mut self.scroll_offset.x,
			&mut self.scroll_target.x,
			&mut self.scroll_velocity.x,
			self.scroll_min.x,
			dt,
		);
		step(
			&mut self.scroll_offset.y,
			&mut self.scroll_target.y,
			&mut self.scroll_velocity.y,
			self.scroll_min.y,
			dt,
		);
		if !self.is_scroll_animating() {
			self.last_scroll = None;
		}
	}

	pub(crate) fn handle_scroll(&mut self, delta: mouse::ScrollDelta, current_time: f32) -> bool {
		match delta {
			_ if self.ctrl_pressed => {
				let dy = match delta {
					mouse::ScrollDelta::Lines { y, .. } => y * SCROLL_MULTIPLIER,
					mouse::ScrollDelta::Pixels { y, .. } => y,
				};
				if dy > 0.0 {
					self.zoom_in(current_time);
				} else if dy < 0.0 {
					self.zoom_out(current_time);
				}
			}
			mouse::ScrollDelta::Lines { x, y } => {
				self.apply_elastic_scroll(x * SCROLL_MULTIPLIER, y * SCROLL_MULTIPLIER);
			}
			mouse::ScrollDelta::Pixels { x, y } => {
				self.apply_elastic_scroll(x, y);
			}
		}
		true
	}
}
