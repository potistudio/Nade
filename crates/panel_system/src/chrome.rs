//! Squircle panel background and outline drawn behind area content.

use iced::mouse;
use iced::widget::canvas::{Canvas, Frame, Geometry, Program, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Theme};

use crate::consts::PANEL_FOCUS_OUTLINE_WIDTH;
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
	/// Hairline between header and body.
	pub(crate) separator_color: Color,
	/// Focus ring color, blended in by `focus`.
	pub(crate) focus_color: Color,
	/// Focus transition progress (`0.0` idle → `1.0` focused).
	pub(crate) focus: f32,
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
		let inner = Rectangle {
			x: PANEL_FOCUS_OUTLINE_WIDTH,
			y: PANEL_FOCUS_OUTLINE_WIDTH,
			width: (bounds.width - PANEL_FOCUS_OUTLINE_WIDTH * 2.0).max(0.0),
			height: (bounds.height - PANEL_FOCUS_OUTLINE_WIDTH * 2.0).max(0.0),
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

		// Draw the separator on the body side so it does not consume header padding.
		if header_rect.height > 0.0 && body_rect.height > 0.0 {
			frame.fill_rectangle(
				Point::new(body_rect.x, body_rect.y),
				iced::Size::new(header_rect.width, 1.0),
				self.separator_color,
			);
		}

		let focus = self.focus.clamp(0.0, 1.0);
		let outline_color = constants::style::mix(self.outline_color, self.focus_color, focus);
		let outline_width = self.outline_width + (PANEL_FOCUS_OUTLINE_WIDTH - self.outline_width) * focus;
		let inset = outline_width * 0.5;
		// Canvas strokes extend to both sides of the path; keep the outer edge inside its bounds.
		let outline_bounds = Rectangle {
			x: inset,
			y: inset,
			width: (bounds.width - outline_width).max(0.0),
			height: (bounds.height - outline_width).max(0.0),
		};
		let outline = squircle_rounded_rectangle(
			outline_bounds,
			(self.outer_radius - inset).max(0.0),
			SquircleCorners::ALL,
			self.exponent,
		);
		frame.stroke(
			&outline,
			Stroke::default().with_width(outline_width).with_color(outline_color),
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

/// macOS-style diagonal resize grip drawn in an area's bottom-right corner.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CornerGrip {
	pub(crate) color: Color,
}

impl<Message> Program<Message> for CornerGrip {
	type State = ();

	fn draw(
		&self,
		_state: &(),
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = Frame::new(renderer, bounds.size());
		let right = bounds.width - 3.0;
		let bottom = bounds.height - 3.0;
		for offset in [4.0, 8.0] {
			let line = iced::widget::canvas::Path::line(
				Point::new(right - offset, bottom),
				Point::new(right, bottom - offset),
			);
			frame.stroke(
				&line,
				Stroke::default()
					.with_width(1.25)
					.with_color(self.color)
					.with_line_cap(iced::widget::canvas::LineCap::Round),
			);
		}
		vec![frame.into_geometry()]
	}

	fn mouse_interaction(&self, _state: &(), bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		if cursor.is_over(bounds) {
			mouse::Interaction::Crosshair
		} else {
			mouse::Interaction::None
		}
	}
}
