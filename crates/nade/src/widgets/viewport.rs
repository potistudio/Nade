//! Viewer coordinate math shared by the video shader and the overlay canvas.
//!
//! Three spaces are involved:
//! - **screen**: pixels inside the viewer widget (origin top-left),
//! - **frame**: composition pixels (origin top-left of the comp),
//! - **comp**: composition pixels relative to the comp center (layer positions).

use iced::{Point, Rectangle, Size, Vector};

pub const MIN_ZOOM: f32 = 0.05;
pub const MAX_ZOOM: f32 = 32.0;

/// Pan/zoom state of the viewer. `zoom = 1.0` fits the comp to the viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
	pub size: Size,
	pub frame: Size,
	pub zoom: f32,
	pub offset: Vector,
}

impl Viewport {
	pub fn new(size: Size, frame: Size, zoom: f32, offset: Vector) -> Self {
		Self {
			size,
			frame: Size::new(frame.width.max(1.0), frame.height.max(1.0)),
			zoom: if zoom > 0.0 { zoom } else { 1.0 },
			offset,
		}
	}

	/// Scale at which the comp fits the viewport.
	pub fn fit_scale(&self) -> f32 {
		(self.size.width / self.frame.width).min(self.size.height / self.frame.height)
	}

	/// Screen pixels per comp pixel.
	pub fn display_scale(&self) -> f32 {
		self.fit_scale() * self.zoom
	}

	/// Effective magnification relative to 100% (1 comp px = 1 screen px).
	pub fn magnification(&self) -> f32 {
		self.display_scale()
	}

	/// Screen rectangle covered by the comp frame.
	pub fn frame_rect(&self) -> Rectangle {
		let scale = self.display_scale();
		let size = Size::new(self.frame.width * scale, self.frame.height * scale);
		Rectangle::new(
			Point::new(
				(self.size.width - size.width) * 0.5 + self.offset.x,
				(self.size.height - size.height) * 0.5 + self.offset.y,
			),
			size,
		)
	}

	/// Comp (center-relative) → screen.
	pub fn comp_to_screen(&self, point: [f32; 2]) -> Point {
		let rect = self.frame_rect();
		let scale = self.display_scale();
		Point::new(
			rect.x + (point[0] + self.frame.width * 0.5) * scale,
			rect.y + (point[1] + self.frame.height * 0.5) * scale,
		)
	}

	/// Screen → comp (center-relative).
	pub fn screen_to_comp(&self, point: Point) -> [f32; 2] {
		let rect = self.frame_rect();
		let scale = self.display_scale();
		[
			(point.x - rect.x) / scale - self.frame.width * 0.5,
			(point.y - rect.y) / scale - self.frame.height * 0.5,
		]
	}

	/// New `(zoom, offset)` after multiplying zoom by `factor` around `anchor`
	/// (screen space) so the point under the anchor stays put.
	pub fn zoomed_at(&self, factor: f32, anchor: Point) -> (f32, Vector) {
		let comp = self.screen_to_comp(anchor);
		let zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
		let next = Self { zoom, ..*self };
		let centered = Self {
			offset: Vector::ZERO,
			..next
		};
		let target = centered.comp_to_screen(comp);
		(zoom, Vector::new(anchor.x - target.x, anchor.y - target.y))
	}

	/// `(zoom, offset)` for an exact magnification (`1.0` = 100%), keeping the
	/// viewport center fixed.
	pub fn with_magnification(&self, magnification: f32) -> (f32, Vector) {
		let zoom = (magnification / self.fit_scale().max(f32::EPSILON)).clamp(MIN_ZOOM, MAX_ZOOM);
		let factor = zoom / self.zoom;
		(zoom, Vector::new(self.offset.x * factor, self.offset.y * factor))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn viewport(zoom: f32, offset: Vector) -> Viewport {
		Viewport::new(Size::new(800.0, 450.0), Size::new(1920.0, 1080.0), zoom, offset)
	}

	#[test]
	fn comp_center_maps_to_viewport_center() {
		let view = viewport(1.0, Vector::ZERO);
		let center = view.comp_to_screen([0.0, 0.0]);
		assert!((center.x - 400.0).abs() < 1e-3 && (center.y - 225.0).abs() < 1e-3);
	}

	#[test]
	fn screen_and_comp_round_trip() {
		let view = viewport(2.3, Vector::new(37.0, -12.0));
		let comp = [-311.0, 145.5];
		let back = view.screen_to_comp(view.comp_to_screen(comp));
		assert!((back[0] - comp[0]).abs() < 1e-2 && (back[1] - comp[1]).abs() < 1e-2);
	}

	#[test]
	fn zoom_keeps_anchor_fixed() {
		let view = viewport(1.0, Vector::new(10.0, 5.0));
		let anchor = Point::new(612.0, 140.0);
		let before = view.screen_to_comp(anchor);
		let (zoom, offset) = view.zoomed_at(1.8, anchor);
		let after = Viewport { zoom, offset, ..view }.screen_to_comp(anchor);
		assert!((before[0] - after[0]).abs() < 1e-2 && (before[1] - after[1]).abs() < 1e-2);
	}

	#[test]
	fn magnification_targets_actual_pixels() {
		let view = viewport(1.0, Vector::ZERO);
		let (zoom, _) = view.with_magnification(1.0);
		let actual = Viewport { zoom, ..view };
		assert!((actual.magnification() - 1.0).abs() < 1e-4);
	}
}
