//! Curve segments, keys and Bezier handles.

use crate::widget::CurveEditorWidget;
use crate::{CURVE_SAMPLES, HANDLE_RADIUS, KEY_RADIUS};
use constants::style::ACCENT_COLOR;
use core::{AnimationCurve, HandleSide, Interpolation};
use iced::{
	Color, Point,
	widget::canvas::{self, LineCap, LineJoin, Path, Stroke},
};

impl CurveEditorWidget<'_> {
	pub(super) fn draw_curve(&self, frame: &mut canvas::Frame, curve: &AnimationCurve, color: Color, selected: bool) {
		if curve.keys().is_empty() {
			return;
		}
		if curve.keys().len() > 1 {
			let path = Path::new(|builder| {
				let first = curve.keys()[0];
				builder.move_to(self.state.curve_to_screen(first.time, first.value));
				for pair in curve.keys().windows(2) {
					let left = pair[0];
					let right = pair[1];
					match left.interpolation {
						Interpolation::Hold => {
							builder.line_to(self.state.curve_to_screen(right.time, left.value));
							builder.line_to(self.state.curve_to_screen(right.time, right.value));
						}
						Interpolation::Linear => builder.line_to(self.state.curve_to_screen(right.time, right.value)),
						Interpolation::Bezier => {
							for sample in 1..=CURVE_SAMPLES {
								let amount = sample as f32 / CURVE_SAMPLES as f32;
								let time = left.time + (right.time - left.time) * amount;
								builder.line_to(self.state.curve_to_screen(time, curve.evaluate(left.value, time)));
							}
						}
					}
				}
			});
			frame.stroke(
				&path,
				Stroke::default()
					.with_color(Color {
						a: if selected { 1.0 } else { 0.32 },
						..color
					})
					.with_width(if selected { 2.0 } else { 1.25 })
					.with_line_cap(LineCap::Round)
					.with_line_join(LineJoin::Round),
			);
		}

		// 選択キーのハンドル（キーより先に描いて下に潜らせる）
		if selected
			&& let Some(id) = self.state.selected_key
			&& let Some(key) = curve.key(id)
		{
			let anchor = self.state.curve_to_screen(key.time, key.value);
			for side in [HandleSide::Left, HandleSide::Right] {
				let handle = key.handle_position(side);
				let handle_position = self.state.curve_to_screen(handle[0], handle[1]);
				frame.stroke(
					&Path::line(anchor, handle_position),
					Stroke::default()
						.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.45))
						.with_width(1.0),
				);
				let knob = Path::circle(handle_position, HANDLE_RADIUS);
				frame.fill(&knob, Color::WHITE);
				frame.stroke(&knob, Stroke::default().with_color(ACCENT_COLOR).with_width(1.0));
			}
		}

		for key in curve.keys() {
			let position = self.state.curve_to_screen(key.time, key.value);
			let is_active = selected && self.state.selected_key == Some(key.id);
			let radius = if is_active { KEY_RADIUS + 1.5 } else { KEY_RADIUS };
			let diamond = Path::new(|builder| {
				builder.move_to(Point::new(position.x, position.y - radius));
				builder.line_to(Point::new(position.x + radius, position.y));
				builder.line_to(Point::new(position.x, position.y + radius));
				builder.line_to(Point::new(position.x - radius, position.y));
				builder.close();
			});
			let fill = if is_active {
				Color::WHITE
			} else if selected {
				color
			} else {
				Color { a: 0.45, ..color }
			};
			frame.fill(&diamond, fill);
			frame.stroke(
				&diamond,
				Stroke::default()
					.with_color(if is_active {
						ACCENT_COLOR
					} else {
						Color::from_rgba(0.0, 0.0, 0.0, 0.55)
					})
					.with_width(if is_active { 1.5 } else { 1.0 }),
			);
		}
	}
}
