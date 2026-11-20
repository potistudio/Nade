//! Canvas program: drawing order and pointer / keyboard input.

use crate::helpers::property_color;
use crate::message::{CurveCanvasEvent, CurveEditorMessage};
use crate::widget::CurveEditorWidget;
use iced::{
	Event, Point, Rectangle, Theme, keyboard, mouse,
	widget::canvas::{self, Geometry},
};
use nade_constants::style::WELL_COLOR;
use nade_domain::TransformProperty;

impl canvas::Program<CurveEditorMessage> for CurveEditorWidget<'_> {
	type State = ();

	fn draw(
		&self,
		_state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		frame.fill_rectangle(Point::ORIGIN, bounds.size(), WELL_COLOR);
		self.draw_grid(&mut frame, bounds.size());
		for property in TransformProperty::ALL {
			self.draw_curve(
				&mut frame,
				self.animation.curve(property),
				property_color(property),
				property == self.state.selected_property,
			);
		}
		self.draw_playhead(&mut frame, bounds.height);
		if self.animation.curve(self.state.selected_property).is_empty() {
			self.draw_empty_hint(&mut frame, bounds.size());
		}
		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		_state: &mut Self::State,
		event: &Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Option<canvas::Action<CurveEditorMessage>> {
		let position = cursor.position_in(bounds)?;
		match event {
			Event::Mouse(mouse::Event::ButtonPressed(button)) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MousePressed {
					position,
					button: *button,
					bounds: bounds.size(),
				}),
			)),
			Event::Mouse(mouse::Event::ButtonReleased(button)) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MouseReleased(*button)),
			)),
			Event::Mouse(mouse::Event::CursorMoved { .. }) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MouseMoved {
					position,
					bounds: bounds.size(),
				}),
			)),
			Event::Mouse(mouse::Event::WheelScrolled { delta }) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MouseWheelScrolled {
					position,
					delta: *delta,
					bounds: bounds.size(),
				}),
			)),
			Event::Keyboard(keyboard::Event::KeyPressed {
				key: keyboard::Key::Named(keyboard::key::Named::Delete | keyboard::key::Named::Backspace),
				..
			}) => Some(canvas::Action::publish(CurveEditorMessage::CanvasEvent(
				CurveCanvasEvent::DeletePressed,
			))),
			_ => None,
		}
	}

	fn mouse_interaction(&self, _state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.map(|_| mouse::Interaction::Crosshair)
			.unwrap_or_default()
	}
}
