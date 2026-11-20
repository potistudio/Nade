//! Blur & Sharpen effects.

use rayon::prelude::*;

use crate::stack::EffectContext;
use crate::surface::{Pixel, Surface, add4, scale4};

/// Widths of three box blurs that approximate a Gaussian of `sigma`.
fn box_sizes(sigma: f32) -> [usize; 3] {
	const PASSES: f32 = 3.0;
	let ideal = (12.0 * sigma * sigma / PASSES + 1.0).sqrt();
	let mut lower = ideal.floor() as i64;
	if lower % 2 == 0 {
		lower -= 1;
	}
	let lower = lower.max(1);
	let upper = lower + 2;
	let lf = lower as f32;
	let lower_count = ((12.0 * sigma * sigma - PASSES * lf * lf - 4.0 * PASSES * lf - 3.0 * PASSES) / (-4.0 * lf - 4.0))
		.round()
		.clamp(0.0, PASSES) as usize;
	let mut sizes = [upper as usize; 3];
	for size in sizes.iter_mut().take(lower_count) {
		*size = lower as usize;
	}
	sizes
}

/// Box blur of every row with radius `radius` (outside counts as transparent).
fn box_rows(surface: &mut Surface, radius: usize) {
	if radius == 0 || surface.width == 0 {
		return;
	}
	let width = surface.width;
	let norm = 1.0 / (2 * radius + 1) as f32;
	surface.pixels.par_chunks_mut(width).for_each(|row| {
		let source: Vec<Pixel> = row.to_vec();
		let mut sum = [0.0_f32; 4];
		for pixel in source.iter().take(radius.min(width - 1) + 1) {
			sum = add4(sum, *pixel);
		}
		for x in 0..width {
			row[x] = scale4(sum, norm);
			if x + radius + 1 < width {
				sum = add4(sum, source[x + radius + 1]);
			}
			if x >= radius {
				sum = add4(sum, scale4(source[x - radius], -1.0));
			}
		}
		// 足し引きの誤差で微小な負値が残らないようにする
		for pixel in row.iter_mut() {
			for channel in pixel.iter_mut() {
				if *channel < 1e-6 {
					*channel = 0.0;
				}
			}
		}
	});
}

/// Gaussian blur with separate horizontal / vertical sigmas (output pixels).
pub fn gaussian(surface: &mut Surface, sigma_x: f32, sigma_y: f32) {
	if sigma_x > 0.2 {
		for size in box_sizes(sigma_x) {
			box_rows(surface, (size - 1) / 2);
		}
	}
	if sigma_y > 0.2 {
		let mut transposed = surface.transposed();
		for size in box_sizes(sigma_y) {
			box_rows(&mut transposed, (size - 1) / 2);
		}
		*surface = transposed.transposed();
	}
}

/// Sigma for an After Effects style blur amount in output pixels.
pub fn sigma_for(amount: f32) -> f32 {
	amount * 0.5
}

/// Gaussian Blur: Blurriness, Dimensions.
pub fn gaussian_blur(surface: &mut Surface, ctx: &EffectContext, blurriness: f32, dimensions: usize) {
	let sigma = sigma_for(blurriness * ctx.scale);
	let (sx, sy) = match dimensions {
		1 => (sigma, 0.0),
		2 => (0.0, sigma),
		_ => (sigma, sigma),
	};
	gaussian(surface, sx, sy);
}

/// Directional Blur: motion blur along `direction` (0° = vertical).
pub fn directional_blur(surface: &mut Surface, ctx: &EffectContext, direction: f32, length: f32) {
	let length = length * ctx.scale;
	if length < 0.5 {
		return;
	}
	let theta = direction.to_radians();
	let (dx, dy) = (theta.sin(), -theta.cos());
	let samples = (length.ceil() as usize + 1).clamp(2, 96);
	let source = surface.clone();
	*surface = source.generate(|x, y| {
		let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
		let mut sum = [0.0; 4];
		for index in 0..samples {
			let t = (index as f32 / (samples - 1) as f32 - 0.5) * length;
			sum = add4(sum, source.sample(px + dx * t, py + dy * t));
		}
		scale4(sum, 1.0 / samples as f32)
	});
}

/// Radial Blur: Spin (rotates `amount`°) or Zoom (towards the center).
pub fn radial_blur(surface: &mut Surface, ctx: &EffectContext, amount: f32, center: [f32; 2], zoom: bool) {
	if amount <= 0.0 {
		return;
	}
	let [cx, cy] = ctx.point(center);
	let samples = 32;
	let source = surface.clone();
	*surface = source.generate(|x, y| {
		let (vx, vy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
		let mut sum = [0.0; 4];
		for index in 0..samples {
			let t = index as f32 / (samples - 1) as f32;
			let (sx, sy) = if zoom {
				let k = 1.0 - amount / 100.0 * 0.5 * t;
				(cx + vx * k, cy + vy * k)
			} else {
				let angle = (t - 0.5) * amount.to_radians();
				let (sin, cos) = angle.sin_cos();
				(cx + vx * cos - vy * sin, cy + vx * sin + vy * cos)
			};
			sum = add4(sum, source.sample(sx, sy));
		}
		scale4(sum, 1.0 / samples as f32)
	});
}

/// Sharpen: unsharp mask against the 4-neighbour average.
pub fn sharpen(surface: &mut Surface, amount: f32) {
	let k = amount / 25.0;
	if k <= 0.0 {
		return;
	}
	let source = surface.clone();
	*surface = source.generate(|x, y| {
		let (xi, yi) = (x as isize, y as isize);
		let center = source.get(x, y);
		let neighbours = [
			source.clamped(xi - 1, yi),
			source.clamped(xi + 1, yi),
			source.clamped(xi, yi - 1),
			source.clamped(xi, yi + 1),
		];
		let mut out = center;
		for channel in 0..3 {
			let average = neighbours.iter().map(|pixel| pixel[channel]).sum::<f32>() / 4.0;
			out[channel] = (center[channel] + (center[channel] - average) * k).clamp(0.0, center[3]);
		}
		out
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dot() -> Surface {
		let mut surface = Surface::new(41, 41);
		surface.pixels[20 * 41 + 20] = [1.0; 4];
		surface
	}

	#[test]
	fn gaussian_spreads_and_keeps_energy() {
		let mut surface = dot();
		gaussian(&mut surface, 3.0, 3.0);
		let total: f32 = surface.pixels.iter().map(|pixel| pixel[3]).sum();
		assert!((total - 1.0).abs() < 0.01, "energy {total}");
		assert!(surface.get(20, 20)[3] < 0.1);
		assert!(surface.get(23, 20)[3] > 0.0);
		assert_eq!(surface.get(20, 20), surface.get(20, 20));
		// 対称に広がる
		assert!((surface.get(17, 20)[3] - surface.get(23, 20)[3]).abs() < 1e-4);
		assert!((surface.get(20, 17)[3] - surface.get(20, 23)[3]).abs() < 1e-4);
	}

	#[test]
	fn horizontal_blur_stays_on_its_row() {
		let mut surface = dot();
		gaussian(&mut surface, 3.0, 0.0);
		assert!(surface.get(24, 20)[3] > 0.0);
		assert_eq!(surface.get(20, 21)[3], 0.0);
	}

	#[test]
	fn box_sizes_grow_with_sigma() {
		assert!(box_sizes(1.0).iter().all(|size| size % 2 == 1));
		assert!(box_sizes(10.0)[0] > box_sizes(2.0)[0]);
	}
}
