//! Editable properties, their colors and grid / formatting math.

use constants::style::{AXIS_X_COLOR, AXIS_Y_COLOR, AXIS_Z_COLOR, LABEL_COLORS, TEXT_SECONDARY_COLOR};
use domain::TransformProperty;
use iced::{Color, Point};

/// Channels editable on 2D layers, in panel order.
pub(crate) const EDITABLE_PROPERTIES: [TransformProperty; 6] = [
	TransformProperty::PositionX,
	TransformProperty::PositionY,
	TransformProperty::RotationZ,
	TransformProperty::ScaleX,
	TransformProperty::ScaleY,
	TransformProperty::Opacity,
];

pub(crate) fn format_value(value: f32, step: f32) -> String {
	if step >= 1.0 {
		format!("{value:.0}")
	} else if step >= 0.1 {
		format!("{value:.1}")
	} else {
		format!("{value:.2}")
	}
}

pub(crate) fn property_color(property: TransformProperty) -> Color {
	match property {
		TransformProperty::PositionX => AXIS_X_COLOR,
		TransformProperty::PositionY => AXIS_Y_COLOR,
		TransformProperty::PositionZ => AXIS_Z_COLOR,
		TransformProperty::RotationX | TransformProperty::RotationY => TEXT_SECONDARY_COLOR,
		TransformProperty::RotationZ => LABEL_COLORS[1],
		TransformProperty::ScaleX => LABEL_COLORS[3],
		TransformProperty::ScaleY => LABEL_COLORS[5],
		TransformProperty::ScaleZ => TEXT_SECONDARY_COLOR,
		TransformProperty::Opacity => Color::from_rgb8(220, 220, 228),
		TransformProperty::AnchorX => Color::from_rgb8(232, 150, 96),
		TransformProperty::AnchorY => Color::from_rgb8(150, 200, 120),
	}
}

pub(crate) fn grid_step(scale: f32, target_pixels: f32) -> f32 {
	let raw = target_pixels / scale.max(0.0001);
	let power = 10.0_f32.powf(raw.log10().floor());
	let normalized = raw / power;
	let factor = if normalized <= 1.0 {
		1.0
	} else if normalized <= 2.0 {
		2.0
	} else if normalized <= 5.0 {
		5.0
	} else {
		10.0
	};
	factor * power
}

pub(crate) fn distance(a: Point, b: Point) -> f32 {
	((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}
