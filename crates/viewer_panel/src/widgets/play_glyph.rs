//! Play ⇄ pause glyph that morphs between the two shapes.
//!
//! Both shapes are described as two quads on a 16×16 grid: the play triangle
//! is split into a left and right half, the pause glyph is its two bars. The
//! glyph interpolates vertex-wise, like the iOS media controls.

use iced::widget::canvas::{self, Frame, Geometry, LineJoin, Path, Program, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme, mouse};

type Quad = [[f32; 2]; 4];

const PLAY: [Quad; 2] = [
	[[5.0, 3.6], [8.7, 5.8], [8.7, 10.2], [5.0, 12.4]],
	[[8.7, 5.8], [12.4, 8.0], [12.4, 8.0], [8.7, 10.2]],
];

const PAUSE: [Quad; 2] = [
	[[4.2, 3.2], [6.9, 3.2], [6.9, 12.8], [4.2, 12.8]],
	[[9.1, 3.2], [11.8, 3.2], [11.8, 12.8], [9.1, 12.8]],
];

/// Draws the glyph; `pause_amount` is `0.0` for ▶ and `1.0` for ❚❚.
#[derive(Debug, Clone, Copy)]
pub struct PlayGlyph {
	pub pause_amount: f32,
	pub color: Color,
}

/// Add the morphing glyph to `frame`, fitted into `rect`.
pub fn draw_play_glyph(frame: &mut Frame, rect: Rectangle, pause_amount: f32, color: Color) {
	let t = pause_amount.clamp(0.0, 1.0);
	let scale = rect.width.min(rect.height) / 16.0;
	let origin = Point::new(
		rect.x + (rect.width - 16.0 * scale) * 0.5,
		rect.y + (rect.height - 16.0 * scale) * 0.5,
	);

	for (play, pause) in PLAY.iter().zip(PAUSE.iter()) {
		let path = Path::new(|builder| {
			for (index, (a, b)) in play.iter().zip(pause.iter()).enumerate() {
				let point = Point::new(
					origin.x + (a[0] + (b[0] - a[0]) * t) * scale,
					origin.y + (a[1] + (b[1] - a[1]) * t) * scale,
				);
				if index == 0 {
					builder.move_to(point);
				} else {
					builder.line_to(point);
				}
			}
			builder.close();
		});
		frame.fill(&path, color);
		// 丸い角に見せるため、同色の細いストロークで縁取る
		frame.stroke(
			&path,
			Stroke::default()
				.with_color(color)
				.with_width(scale * 1.1)
				.with_line_join(LineJoin::Round),
		);
	}
}

impl<Message> Program<Message> for PlayGlyph {
	type State = ();

	fn draw(
		&self,
		_state: &(),
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		draw_play_glyph(
			&mut frame,
			Rectangle::new(Point::ORIGIN, bounds.size()),
			self.pause_amount,
			self.color,
		);
		vec![frame.into_geometry()]
	}

	fn mouse_interaction(&self, _state: &(), _bounds: Rectangle, _cursor: mouse::Cursor) -> mouse::Interaction {
		mouse::Interaction::None
	}
}
