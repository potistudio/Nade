//! タイムラインのトラックコントロール用アイコン描画

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Point, Rectangle};

const VIEWBOX: f32 = 16.0;

fn map_point(rect: Rectangle, x: f32, y: f32) -> Point {
	let scale = ((rect.width - 4.0) / VIEWBOX).min((rect.height - 4.0) / VIEWBOX);
	let offset_x = rect.x + (rect.width - VIEWBOX * scale) * 0.5;
	let offset_y = rect.y + (rect.height - VIEWBOX * scale) * 0.5;
	Point::new(offset_x + x * scale, offset_y + y * scale)
}

fn map_radius(rect: Rectangle, radius: f32) -> f32 {
	let scale = ((rect.width - 4.0) / VIEWBOX).min((rect.height - 4.0) / VIEWBOX);
	radius * scale
}

/// 可視トグル用の目アイコン
pub fn draw_eye_icon(frame: &mut canvas::Frame, rect: Rectangle, color: Color) {
	let p = |x, y| map_point(rect, x, y);
	let stroke_width = map_radius(rect, 1.0).max(0.8);

	let eye = Path::new(|builder| {
		builder.move_to(p(2.5, 8.0));
		builder.quadratic_curve_to(p(8.0, 3.0), p(13.5, 8.0));
		builder.quadratic_curve_to(p(8.0, 13.0), p(2.5, 8.0));
		builder.close();
	});
	frame.stroke(&eye, Stroke::default().with_color(color).with_width(stroke_width));

	let pupil = Path::circle(p(8.0, 8.0), map_radius(rect, 1.8));
	frame.fill(&pupil, color);
}

/// ソロトグル用のターゲットアイコン
pub fn draw_solo_icon(frame: &mut canvas::Frame, rect: Rectangle, color: Color, active: bool) {
	let center = map_point(rect, 8.0, 8.0);
	let stroke_width = map_radius(rect, 1.0).max(0.8);
	let outer_radius = map_radius(rect, 3.5);

	if active {
		let dot = Path::circle(center, outer_radius);
		frame.fill(&dot, color);
	} else {
		let ring = Path::circle(center, outer_radius);
		frame.stroke(&ring, Stroke::default().with_color(color).with_width(stroke_width));
	}
}
