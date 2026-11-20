//! 2D affine transforms and layer blending modes.

use serde::{Deserialize, Serialize};

/// 2D affine map: `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty` (screen y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
	pub a: f32,
	pub b: f32,
	pub c: f32,
	pub d: f32,
	pub tx: f32,
	pub ty: f32,
}

impl Default for Affine {
	fn default() -> Self {
		Self::IDENTITY
	}
}

impl Affine {
	pub const IDENTITY: Self = Self {
		a: 1.0,
		b: 0.0,
		c: 0.0,
		d: 1.0,
		tx: 0.0,
		ty: 0.0,
	};

	pub fn translate(x: f32, y: f32) -> Self {
		Self {
			tx: x,
			ty: y,
			..Self::IDENTITY
		}
	}

	pub fn scale(x: f32, y: f32) -> Self {
		Self {
			a: x,
			d: y,
			..Self::IDENTITY
		}
	}

	/// Clockwise rotation on screen, in degrees.
	pub fn rotate(degrees: f32) -> Self {
		let (sin, cos) = degrees.to_radians().sin_cos();
		Self {
			a: cos,
			b: sin,
			c: -sin,
			d: cos,
			..Self::IDENTITY
		}
	}

	/// `self ∘ inner`: apply `inner` first, then `self`.
	pub fn then_apply(self, inner: Self) -> Self {
		Self {
			a: self.a * inner.a + self.c * inner.b,
			b: self.b * inner.a + self.d * inner.b,
			c: self.a * inner.c + self.c * inner.d,
			d: self.b * inner.c + self.d * inner.d,
			tx: self.a * inner.tx + self.c * inner.ty + self.tx,
			ty: self.b * inner.tx + self.d * inner.ty + self.ty,
		}
	}

	pub fn apply(&self, point: [f32; 2]) -> [f32; 2] {
		[
			self.a * point[0] + self.c * point[1] + self.tx,
			self.b * point[0] + self.d * point[1] + self.ty,
		]
	}

	/// Apply only the linear part (for directions / deltas).
	pub fn apply_vector(&self, vector: [f32; 2]) -> [f32; 2] {
		[
			self.a * vector[0] + self.c * vector[1],
			self.b * vector[0] + self.d * vector[1],
		]
	}

	pub fn determinant(&self) -> f32 {
		self.a * self.d - self.b * self.c
	}

	pub fn inverse(&self) -> Option<Self> {
		let det = self.determinant();
		if !det.is_finite() || det.abs() < 1e-12 {
			return None;
		}
		let inv = 1.0 / det;
		let (a, b, c, d) = (self.d * inv, -self.b * inv, -self.c * inv, self.a * inv);
		Some(Self {
			a,
			b,
			c,
			d,
			tx: -(a * self.tx + c * self.ty),
			ty: -(b * self.tx + d * self.ty),
		})
	}

	/// Length of the transformed x / y unit vectors.
	pub fn axis_scales(&self) -> [f32; 2] {
		[self.a.hypot(self.b), self.c.hypot(self.d)]
	}

	/// Largest magnification (for picking texture resolution).
	pub fn max_scale(&self) -> f32 {
		let [x, y] = self.axis_scales();
		x.max(y)
	}

	/// Rotation of the x axis, in degrees.
	pub fn rotation(&self) -> f32 {
		self.b.atan2(self.a).to_degrees()
	}
}

/// How a layer combines with the layers below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BlendMode {
	#[default]
	Normal,
	Add,
	Screen,
	Multiply,
	Overlay,
	Lighten,
	Darken,
	Difference,
}

impl BlendMode {
	pub const ALL: [Self; 8] = [
		Self::Normal,
		Self::Add,
		Self::Screen,
		Self::Multiply,
		Self::Overlay,
		Self::Lighten,
		Self::Darken,
		Self::Difference,
	];

	pub fn label(self) -> &'static str {
		match self {
			Self::Normal => "Normal",
			Self::Add => "Add",
			Self::Screen => "Screen",
			Self::Multiply => "Multiply",
			Self::Overlay => "Overlay",
			Self::Lighten => "Lighten",
			Self::Darken => "Darken",
			Self::Difference => "Difference",
		}
	}

	/// Blend one straight (non-premultiplied) channel: `backdrop` below, `source` above.
	#[inline]
	pub fn channel(self, backdrop: f32, source: f32) -> f32 {
		match self {
			Self::Normal => source,
			Self::Add => (backdrop + source).min(1.0),
			Self::Screen => backdrop + source - backdrop * source,
			Self::Multiply => backdrop * source,
			Self::Overlay => {
				if backdrop <= 0.5 {
					2.0 * backdrop * source
				} else {
					1.0 - 2.0 * (1.0 - backdrop) * (1.0 - source)
				}
			}
			Self::Lighten => backdrop.max(source),
			Self::Darken => backdrop.min(source),
			Self::Difference => (backdrop - source).abs(),
		}
	}
}

impl std::fmt::Display for BlendMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.label())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn close(a: [f32; 2], b: [f32; 2]) -> bool {
		(a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
	}

	#[test]
	fn composes_in_order() {
		let m = Affine::translate(10.0, 0.0).then_apply(Affine::rotate(90.0));
		// 回転してから平行移動: (1,0) → (0,1) → (10,1)
		assert!(close(m.apply([1.0, 0.0]), [10.0, 1.0]));
		let inverse = m.inverse().unwrap();
		assert!(close(inverse.apply([10.0, 1.0]), [1.0, 0.0]));
		assert!(Affine::scale(0.0, 1.0).inverse().is_none());
	}

	#[test]
	fn reports_scale_and_rotation() {
		let m = Affine::rotate(30.0).then_apply(Affine::scale(2.0, 3.0));
		let [sx, sy] = m.axis_scales();
		assert!((sx - 2.0).abs() < 1e-5 && (sy - 3.0).abs() < 1e-5);
		assert!((m.rotation() - 30.0).abs() < 1e-4);
	}

	#[test]
	fn blend_modes() {
		assert_eq!(BlendMode::Multiply.channel(0.5, 0.5), 0.25);
		assert_eq!(BlendMode::Screen.channel(0.5, 0.5), 0.75);
		assert_eq!(BlendMode::Add.channel(0.75, 0.5), 1.0);
		assert_eq!(BlendMode::Difference.channel(0.25, 1.0), 0.75);
	}
}
