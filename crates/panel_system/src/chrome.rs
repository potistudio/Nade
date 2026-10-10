//! Squircle panel background and outline drawn behind area content.

use iced::mouse;
use iced::widget::canvas::{Canvas, Frame, Geometry, Program, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Theme};

use crate::squircle::{SquircleCorners, squircle_rounded_rectangle};

#[derive(Debug, Clone, Copy)]
pub(crate) struct PanelChrome {
	pub(crate) header_height: f32,
	pub(crate) outline_width: f32,
	pub(crate) outer_radius: f32,
	pub(crate) inner_radius: f32,
	pub(crate) exponent: f32,
	pub(crate) panel_color: Color,
	pub(crate) header_color: Color,
	pub(crate) outline_color: Color,
}

impl<Message> Program<Message> for PanelChrome {
	type State = ();

	fn draw(
		&self,
		_state: &(),
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		if bounds.width < 1.0 || bounds.height < 1.0 {
			return Vec::new();
		}

		let mut frame = Frame::new(renderer, bounds.size());
		let outer = Rectangle::new(Point::ORIGIN, bounds.size());
		let inner = Rectangle {
			x: self.outline_width,
			y: self.outline_width,
			width: (bounds.width - self.outline_width * 2.0).max(0.0),
			height: (bounds.height - self.outline_width * 2.0).max(0.0),
		};

		let header_height = self.header_height.min(inner.height);
		let header_rect = Rectangle {
			x: inner.x,
			y: inner.y,
			width: inner.width,
			height: header_height,
		};
		let body_rect = Rectangle {
			x: inner.x,
			y: inner.y + header_height,
			width: inner.width,
			height: (inner.height - header_height).max(0.0),
		};

		if body_rect.height > 0.0 {
			let body = squircle_rounded_rectangle(body_rect, self.inner_radius, SquircleCorners::BOTTOM, self.exponent);
			frame.fill(&body, self.panel_color);
		}

		if header_rect.height > 0.0 {
			let header =
				squircle_rounded_rectangle(header_rect, self.inner_radius, SquircleCorners::TOP, self.exponent);
			frame.fill(&header, self.header_color);
		}

		let outline = squircle_rounded_rectangle(outer, self.outer_radius, SquircleCorners::ALL, self.exponent);
		frame.stroke(
			&outline,
			Stroke::default()
				.with_width(self.outline_width)
				.with_color(self.outline_color),
		);

		vec![frame.into_geometry()]
	}

	fn mouse_interaction(&self, _state: &(), _bounds: Rectangle, _cursor: mouse::Cursor) -> mouse::Interaction {
		mouse::Interaction::None
	}
}

pub(crate) fn squircle_panel_stack<'a, Message: 'a>(
	chrome: PanelChrome,
	content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
	iced::widget::stack![
		Canvas::new(chrome).width(Length::Fill).height(Length::Fill),
		content.into(),
	]
	.into()
}
