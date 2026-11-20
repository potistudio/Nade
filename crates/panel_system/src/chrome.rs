//! Squircle panel background and outline drawn behind area content.

use iced::border::Radius;
use iced::mouse;
use iced::widget::canvas::{Frame, Geometry, Program, Stroke};
use iced::widget::{Space, column, container, stack};
use iced::{Border, Color, Element, Length, Point, Rectangle, Theme};

use nade_constants::squircle::EXTENT;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PanelChrome {
	pub(crate) header_height: f32,
	pub(crate) outline_width: f32,
	/// Corner extent of the outer outline.
	pub(crate) outer_radius: f32,
	/// Corner extent of the header/body fill inside the outline.
	pub(crate) inner_radius: f32,
	pub(crate) panel_color: Color,
	pub(crate) header_color: Color,
	pub(crate) outline_color: Color,
	/// Hairline between header and body.
	pub(crate) separator_color: Color,
	pub(crate) hover_color: Color,
	pub(crate) hovered: bool,
}

pub(crate) fn squircle_panel_stack<'a, Message: 'a>(
	chrome: PanelChrome,
	content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
	let PanelChrome {
		header_height,
		outline_width,
		outer_radius,
		inner_radius,
		panel_color,
		header_color,
		outline_color,
		separator_color,
		hover_color,
		hovered,
	} = chrome;

	// The quad shader turns `border.radius` into a continuous corner with a
	// 2.25x extent. Dividing the extent tokens keeps the panel geometry as it
	// was before the shader change.
	let header_radius = Radius {
		top_left: inner_radius / EXTENT,
		top_right: inner_radius / EXTENT,
		bottom_right: 0.0,
		bottom_left: 0.0,
	};
	let body_radius = Radius {
		top_left: 0.0,
		top_right: 0.0,
		bottom_right: inner_radius / EXTENT,
		bottom_left: inner_radius / EXTENT,
	};

	let header = container(Space::new())
		.width(Length::Fill)
		.height(header_height)
		.style(move |_| container::Style {
			background: Some(header_color.into()),
			border: Border {
				radius: header_radius,
				..Border::default()
			},
			..container::Style::default()
		});

	let body = container(Space::new())
		.width(Length::Fill)
		.height(Length::Fill)
		.style(move |_| container::Style {
			background: Some(panel_color.into()),
			border: Border {
				radius: body_radius,
				..Border::default()
			},
			..container::Style::default()
		});

	let separator = container(Space::new())
		.width(Length::Fill)
		.height(1)
		.style(move |_| container::Style {
			background: Some(separator_color.into()),
			..container::Style::default()
		});

	// Keep the separator on the body's top edge instead of giving it a row of
	// its own, so the content still starts directly under the header.
	let separator_layer = column![Space::new().height(header_height), separator]
		.width(Length::Fill)
		.height(Length::Fill);

	let background = stack![
		column![header, body].width(Length::Fill).height(Length::Fill),
		separator_layer,
	]
	.width(Length::Fill)
	.height(Length::Fill);

	let outline_color = if hovered { hover_color } else { outline_color };

	container(
		stack![background, content.into()]
			.width(Length::Fill)
			.height(Length::Fill),
	)
	.width(Length::Fill)
	.height(Length::Fill)
	.padding(outline_width)
	.style(move |_| container::Style {
		border: Border {
			color: outline_color,
			width: outline_width,
			radius: Radius::new(outer_radius / EXTENT),
		},
		..container::Style::default()
	})
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
