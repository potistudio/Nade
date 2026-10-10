//! Superellipse (squircle) paths for panel chrome.

use std::f32::consts::{FRAC_PI_2, PI};

use iced::Point;
use iced::widget::canvas;
use iced::widget::canvas::path::Builder;

/// Which corners of a rounded rectangle use squircle curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquircleCorners {
	pub top_left: bool,
	pub top_right: bool,
	pub bottom_right: bool,
	pub bottom_left: bool,
}

impl SquircleCorners {
	pub const ALL: Self = Self {
		top_left: true,
		top_right: true,
		bottom_right: true,
		bottom_left: true,
	};

	pub const TOP: Self = Self {
		top_left: true,
		top_right: true,
		bottom_right: false,
		bottom_left: false,
	};

	pub const BOTTOM: Self = Self {
		top_left: false,
		top_right: false,
		bottom_right: true,
		bottom_left: true,
	};
}

/// Builds a squircle rounded rectangle path.
pub fn squircle_rounded_rectangle(
	rect: iced::Rectangle,
	radius: f32,
	corners: SquircleCorners,
	exponent: f32,
) -> canvas::Path {
	canvas::Path::new(|path| {
		append_squircle_rounded_rectangle(path, rect, radius, corners, exponent);
	})
}

fn append_squircle_rounded_rectangle(
	path: &mut Builder,
	rect: iced::Rectangle,
	radius: f32,
	corners: SquircleCorners,
	exponent: f32,
) {
	let radius = radius.min(rect.width / 2.0).min(rect.height / 2.0);
	if radius <= 0.0 {
		path.rectangle(Point::new(rect.x, rect.y), rect.size());
		return;
	}

	let left = rect.x;
	let right = rect.x + rect.width;
	let top = rect.y;
	let bottom = rect.y + rect.height;

	let start = if corners.top_left {
		Point::new(left + radius, top)
	} else {
		Point::new(left, top)
	};
	path.move_to(start);

	if corners.top_right {
		path.line_to(Point::new(right - radius, top));
		append_squircle_corner(
			path,
			Point::new(right - radius, top + radius),
			radius,
			-FRAC_PI_2,
			0.0,
			exponent,
		);
	} else {
		path.line_to(Point::new(right, top));
	}

	if corners.bottom_right {
		path.line_to(Point::new(right, bottom - radius));
		append_squircle_corner(
			path,
			Point::new(right - radius, bottom - radius),
			radius,
			0.0,
			FRAC_PI_2,
			exponent,
		);
	} else {
		path.line_to(Point::new(right, bottom));
	}

	if corners.bottom_left {
		path.line_to(Point::new(left + radius, bottom));
		append_squircle_corner(
			path,
			Point::new(left + radius, bottom - radius),
			radius,
			FRAC_PI_2,
			PI,
			exponent,
		);
	} else {
		path.line_to(Point::new(left, bottom));
	}

	if corners.top_left {
		path.line_to(Point::new(left, top + radius));
		append_squircle_corner(
			path,
			Point::new(left + radius, top + radius),
			radius,
			PI,
			PI + FRAC_PI_2,
			exponent,
		);
	} else {
		path.line_to(Point::new(left, top));
	}

	path.close();
}

fn append_squircle_corner(
	path: &mut Builder,
	center: Point,
	radius: f32,
	start_angle: f32,
	end_angle: f32,
	exponent: f32,
) {
	const SEGMENTS: usize = 16;

	for step in 1..=SEGMENTS {
		let t = start_angle + (end_angle - start_angle) * (step as f32 / SEGMENTS as f32);
		let point = squircle_point(center, radius, t, exponent);
		path.line_to(point);
	}
}

fn squircle_point(center: Point, radius: f32, angle: f32, exponent: f32) -> Point {
	let cos_t = angle.cos();
	let sin_t = angle.sin();
	let power = 2.0 / exponent;

	Point::new(
		center.x + radius * cos_t.signum() * cos_t.abs().powf(power),
		center.y + radius * sin_t.signum() * sin_t.abs().powf(power),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use iced::Size;

	#[test]
	fn builds_squircle_paths() {
		let rect = iced::Rectangle::new(Point::ORIGIN, Size::new(120.0, 80.0));
		let _ = squircle_rounded_rectangle(rect, 6.0, SquircleCorners::ALL, 4.0);
		let _ = squircle_rounded_rectangle(rect, 5.0, SquircleCorners::TOP, 4.0);
		let _ = squircle_rounded_rectangle(rect, 5.0, SquircleCorners::BOTTOM, 4.0);
	}
}
