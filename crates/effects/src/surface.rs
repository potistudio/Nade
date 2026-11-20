//! Float image buffer the layer effects run on.

use rayon::prelude::*;

/// Premultiplied RGBA pixel, channels in 0..1.
pub type Pixel = [f32; 4];

/// A premultiplied RGBA image in floating point.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
	pub width: usize,
	pub height: usize,
	/// Row-major pixels.
	pub pixels: Vec<Pixel>,
}

impl Surface {
	/// Fully transparent surface.
	pub fn new(width: usize, height: usize) -> Self {
		Self {
			width,
			height,
			pixels: vec![[0.0; 4]; width * height],
		}
	}

	/// From premultiplied 8-bit RGBA bytes.
	pub fn from_rgba8(width: usize, height: usize, bytes: &[u8]) -> Self {
		let pixels = bytes.par_chunks_exact(4).map(pixel_from_bytes).collect();
		Self { width, height, pixels }
	}

	/// Premultiplied 8-bit RGBA bytes (alpha clamped, color kept under it).
	pub fn to_rgba8(&self) -> Vec<u8> {
		let mut bytes = vec![0u8; self.pixels.len() * 4];
		bytes
			.par_chunks_exact_mut(4)
			.zip(self.pixels.par_iter())
			.for_each(|(out, pixel)| {
				let alpha = pixel[3].clamp(0.0, 1.0);
				let to_byte = |value: f32| (value.clamp(0.0, alpha) * 255.0 + 0.5) as u8;
				out[0] = to_byte(pixel[0]);
				out[1] = to_byte(pixel[1]);
				out[2] = to_byte(pixel[2]);
				out[3] = (alpha * 255.0 + 0.5) as u8;
			});
		bytes
	}

	#[inline]
	pub fn get(&self, x: usize, y: usize) -> Pixel {
		self.pixels[y * self.width + x]
	}

	/// Pixel at integer coordinates, transparent outside the surface.
	#[inline]
	pub fn at(&self, x: isize, y: isize) -> Pixel {
		if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
			return [0.0; 4];
		}
		self.pixels[y as usize * self.width + x as usize]
	}

	/// Pixel at integer coordinates, edges repeated.
	#[inline]
	pub fn clamped(&self, x: isize, y: isize) -> Pixel {
		let x = x.clamp(0, self.width as isize - 1) as usize;
		let y = y.clamp(0, self.height as isize - 1) as usize;
		self.pixels[y * self.width + x]
	}

	/// Bilinear sample at a continuous position (pixel centers sit on `.5`),
	/// transparent outside.
	#[inline]
	pub fn sample(&self, x: f32, y: f32) -> Pixel {
		let (u, v) = (x - 0.5, y - 0.5);
		if !(u > -1.0 && v > -1.0 && u < self.width as f32 && v < self.height as f32) {
			return [0.0; 4];
		}
		let (x0, y0) = (u.floor(), v.floor());
		let (fx, fy) = (u - x0, v - y0);
		let (x0, y0) = (x0 as isize, y0 as isize);
		let a = self.at(x0, y0);
		let b = self.at(x0 + 1, y0);
		let c = self.at(x0, y0 + 1);
		let d = self.at(x0 + 1, y0 + 1);
		lerp4(lerp4(a, b, fx), lerp4(c, d, fx), fy)
	}

	/// Bilinear sample that wraps around the edges (tiling).
	#[inline]
	pub fn sample_wrapped(&self, x: f32, y: f32) -> Pixel {
		let (w, h) = (self.width as isize, self.height as isize);
		let (u, v) = (x - 0.5, y - 0.5);
		if !(u.is_finite() && v.is_finite()) {
			return [0.0; 4];
		}
		let (x0, y0) = (u.floor(), v.floor());
		let (fx, fy) = (u - x0, v - y0);
		let (x0, y0) = (x0 as isize, y0 as isize);
		let fetch = |x: isize, y: isize| self.pixels[y.rem_euclid(h) as usize * self.width + x.rem_euclid(w) as usize];
		lerp4(
			lerp4(fetch(x0, y0), fetch(x0 + 1, y0), fx),
			lerp4(fetch(x0, y0 + 1), fetch(x0 + 1, y0 + 1), fx),
			fy,
		)
	}

	/// Replace every pixel with `shade(x, y, pixel)` (in parallel by rows).
	pub fn map(&mut self, shade: impl Fn(usize, usize, Pixel) -> Pixel + Sync) {
		let width = self.width.max(1);
		self.pixels.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
			for (x, pixel) in row.iter_mut().enumerate() {
				*pixel = shade(x, y, *pixel);
			}
		});
	}

	/// New surface of the same size where each pixel is `shade(x, y)`.
	pub fn generate(&self, shade: impl Fn(usize, usize) -> Pixel + Sync) -> Self {
		let width = self.width.max(1);
		let mut out = Self::new(self.width, self.height);
		out.pixels.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
			for (x, pixel) in row.iter_mut().enumerate() {
				*pixel = shade(x, y);
			}
		});
		out
	}

	/// Distort: each output pixel center samples the source at `source(x, y)`.
	pub fn warp(&mut self, source: impl Fn(f32, f32) -> (f32, f32) + Sync) {
		let warped = self.generate(|x, y| {
			let (sx, sy) = source(x as f32 + 0.5, y as f32 + 0.5);
			self.sample(sx, sy)
		});
		*self = warped;
	}

	/// Swap rows and columns.
	pub fn transposed(&self) -> Self {
		let mut out = Self::new(self.height, self.width);
		let height = self.height.max(1);
		out.pixels.par_chunks_mut(height).enumerate().for_each(|(x, column)| {
			for (y, pixel) in column.iter_mut().enumerate() {
				*pixel = self.pixels[y * self.width + x];
			}
		});
		out
	}
}

#[inline]
fn pixel_from_bytes(bytes: &[u8]) -> Pixel {
	[
		f32::from(bytes[0]) / 255.0,
		f32::from(bytes[1]) / 255.0,
		f32::from(bytes[2]) / 255.0,
		f32::from(bytes[3]) / 255.0,
	]
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

#[inline]
pub fn lerp4(a: Pixel, b: Pixel, t: f32) -> Pixel {
	[
		lerp(a[0], b[0], t),
		lerp(a[1], b[1], t),
		lerp(a[2], b[2], t),
		lerp(a[3], b[3], t),
	]
}

#[inline]
pub fn add4(a: Pixel, b: Pixel) -> Pixel {
	[a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

#[inline]
pub fn scale4(a: Pixel, k: f32) -> Pixel {
	[a[0] * k, a[1] * k, a[2] * k, a[3] * k]
}

/// Straight color and alpha of a premultiplied pixel.
#[inline]
pub fn unpremultiply(pixel: Pixel) -> ([f32; 3], f32) {
	let alpha = pixel[3];
	if alpha <= 1e-6 {
		return ([0.0; 3], 0.0);
	}
	let inv = 1.0 / alpha;
	(
		[
			(pixel[0] * inv).clamp(0.0, 1.0),
			(pixel[1] * inv).clamp(0.0, 1.0),
			(pixel[2] * inv).clamp(0.0, 1.0),
		],
		alpha,
	)
}

#[inline]
pub fn premultiply(color: [f32; 3], alpha: f32) -> Pixel {
	[
		color[0].clamp(0.0, 1.0) * alpha,
		color[1].clamp(0.0, 1.0) * alpha,
		color[2].clamp(0.0, 1.0) * alpha,
		alpha,
	]
}

/// Rec. 709 luma of a straight color.
#[inline]
pub fn luma(color: [f32; 3]) -> f32 {
	0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]
}

#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	if edge1 <= edge0 {
		return if x >= edge0 { 1.0 } else { 0.0 };
	}
	let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn round_trips_bytes() {
		let bytes = [255, 128, 0, 255, 64, 0, 0, 128];
		let surface = Surface::from_rgba8(2, 1, &bytes);
		assert_eq!(surface.to_rgba8(), bytes);
	}

	#[test]
	fn samples_pixel_centers_exactly() {
		let mut surface = Surface::new(2, 2);
		surface.pixels[3] = [1.0, 1.0, 1.0, 1.0];
		assert_eq!(surface.sample(1.5, 1.5), [1.0; 4]);
		assert_eq!(surface.sample(1.0, 1.5)[3], 0.5);
		assert_eq!(surface.sample(-3.0, 0.5), [0.0; 4]);
		assert_eq!(surface.sample_wrapped(-0.5, -0.5), [1.0; 4]);
	}

	#[test]
	fn transposes() {
		let mut surface = Surface::new(3, 1);
		surface.pixels[2] = [1.0; 4];
		let transposed = surface.transposed();
		assert_eq!((transposed.width, transposed.height), (1, 3));
		assert_eq!(transposed.get(0, 2), [1.0; 4]);
	}
}
