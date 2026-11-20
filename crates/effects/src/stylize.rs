//! Stylize and Perspective effects.

use rayon::prelude::*;

use crate::blur::{gaussian, sigma_for};
use crate::stack::EffectContext;
use crate::surface::{Pixel, Surface, add4, lerp, luma, premultiply, scale4, unpremultiply};

pub struct Shadow {
	pub color: [f32; 3],
	pub opacity: f32,
	pub direction: f32,
	pub distance: f32,
	pub softness: f32,
	pub shadow_only: bool,
}

/// Drop Shadow: the layer's alpha, offset, softened and drawn underneath.
pub fn drop_shadow(surface: &mut Surface, ctx: &EffectContext, shadow: Shadow) {
	let theta = shadow.direction.to_radians();
	let distance = shadow.distance * ctx.scale;
	let (ox, oy) = (theta.sin() * distance, -theta.cos() * distance);
	let tint = premultiply(shadow.color, 1.0);
	let mut shade = surface.generate(|x, y| {
		let alpha = surface.sample(x as f32 + 0.5 - ox, y as f32 + 0.5 - oy)[3] * shadow.opacity;
		scale4(tint, alpha)
	});
	let sigma = sigma_for(shadow.softness * ctx.scale);
	gaussian(&mut shade, sigma, sigma);
	if shadow.shadow_only {
		*surface = shade;
		return;
	}
	surface
		.pixels
		.par_iter_mut()
		.zip(shade.pixels.par_iter())
		.for_each(|(pixel, below)| {
			// 元のレイヤーを影の上に重ねる
			*pixel = add4(*pixel, scale4(*below, 1.0 - pixel[3]));
		});
}

/// Glow: blurred highlights above `threshold`, added back on top.
pub fn glow(surface: &mut Surface, ctx: &EffectContext, threshold: f32, radius: f32, intensity: f32) {
	if intensity <= 0.0 {
		return;
	}
	let knee = (1.0 - threshold).max(1e-3);
	let mut highlights = surface.generate(|x, y| {
		let (color, alpha) = unpremultiply(surface.get(x, y));
		let bright = color.map(|c| ((c - threshold) / knee).max(0.0));
		let strength = bright[0].max(bright[1]).max(bright[2]).min(1.0);
		[
			bright[0] * alpha,
			bright[1] * alpha,
			bright[2] * alpha,
			strength * alpha,
		]
	});
	let sigma = sigma_for(radius * ctx.scale);
	gaussian(&mut highlights, sigma, sigma);
	surface
		.pixels
		.par_iter_mut()
		.zip(highlights.pixels.par_iter())
		.for_each(|(pixel, light)| {
			let mut out = add4(*pixel, scale4(*light, intensity));
			out[3] = (pixel[3] + light[3] * intensity * (1.0 - pixel[3])).min(1.0);
			// 加算で色がαを超えたらαを持ち上げる（乗算済みの整合性）
			let peak = out[0].max(out[1]).max(out[2]);
			out[3] = out[3].max(peak.min(1.0));
			for channel in out.iter_mut().take(3) {
				*channel = channel.min(1.0);
			}
			*pixel = out;
		});
}

/// Mosaic: average (or center-sample) blocks of the frame.
pub fn mosaic(surface: &mut Surface, horizontal: f32, vertical: f32, sharp: bool) {
	let (width, height) = (surface.width, surface.height);
	if width == 0 || height == 0 {
		return;
	}
	let columns = (horizontal.round() as usize).clamp(1, width);
	let rows = (vertical.round() as usize).clamp(1, height);
	let block = |index: usize, count: usize, size: usize| (index * size / count, (index + 1) * size / count);
	let colors: Vec<Pixel> = (0..rows * columns)
		.into_par_iter()
		.map(|cell| {
			let (row, column) = (cell / columns, cell % columns);
			let (x0, x1) = block(column, columns, width);
			let (y0, y1) = block(row, rows, height);
			if sharp {
				return surface.get((x0 + x1) / 2, (y0 + y1) / 2);
			}
			let mut sum = [0.0; 4];
			for y in y0..y1 {
				for x in x0..x1 {
					sum = add4(sum, surface.get(x, y));
				}
			}
			scale4(sum, 1.0 / ((x1 - x0) * (y1 - y0)).max(1) as f32)
		})
		.collect();
	surface.map(|x, y, _| {
		let column = (x * columns / width).min(columns - 1);
		let row = (y * rows / height).min(rows - 1);
		colors[row * columns + column]
	});
}

/// Find Edges: Sobel per channel, white paper with dark lines by default.
pub fn find_edges(surface: &mut Surface, invert: bool, blend: f32) {
	let source = surface.clone();
	*surface = source.generate(|x, y| {
		let original = source.get(x, y);
		if original[3] <= 0.0 {
			return original;
		}
		let (xi, yi) = (x as isize, y as isize);
		let fetch = |dx: isize, dy: isize| unpremultiply(source.clamped(xi + dx, yi + dy)).0;
		let mut edges = [0.0; 3];
		let (tl, t, tr) = (fetch(-1, -1), fetch(0, -1), fetch(1, -1));
		let (l, r) = (fetch(-1, 0), fetch(1, 0));
		let (bl, b, br) = (fetch(-1, 1), fetch(0, 1), fetch(1, 1));
		for channel in 0..3 {
			let gx = tr[channel] + 2.0 * r[channel] + br[channel] - tl[channel] - 2.0 * l[channel] - bl[channel];
			let gy = bl[channel] + 2.0 * b[channel] + br[channel] - tl[channel] - 2.0 * t[channel] - tr[channel];
			let magnitude = ((gx * gx + gy * gy).sqrt() / 2.0).min(1.0);
			edges[channel] = if invert { magnitude } else { 1.0 - magnitude };
		}
		let (color, alpha) = unpremultiply(original);
		let mixed = [
			lerp(edges[0], color[0], blend),
			lerp(edges[1], color[1], blend),
			lerp(edges[2], color[2], blend),
		];
		premultiply(mixed, alpha)
	});
}

/// Emboss: grey relief lit from `direction`.
pub fn emboss(surface: &mut Surface, ctx: &EffectContext, direction: f32, relief: f32, contrast: f32, blend: f32) {
	let theta = direction.to_radians();
	let relief = (relief * ctx.scale).max(0.5);
	let (dx, dy) = (theta.cos() * relief, -theta.sin() * relief);
	let source = surface.clone();
	*surface = source.generate(|x, y| {
		let original = source.get(x, y);
		if original[3] <= 0.0 {
			return original;
		}
		let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
		let lit = luma(unpremultiply(source.sample(px + dx, py + dy)).0);
		let shaded = luma(unpremultiply(source.sample(px - dx, py - dy)).0);
		let grey = (0.5 + (lit - shaded) * contrast * 2.0).clamp(0.0, 1.0);
		let (color, alpha) = unpremultiply(original);
		premultiply(color.map(|c| lerp(grey, c, blend)), alpha)
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	fn ctx() -> EffectContext {
		EffectContext {
			scale: 1.0,
			time: 0.0,
			width: 20,
			height: 20,
		}
	}

	fn square() -> Surface {
		let mut surface = Surface::new(20, 20);
		surface.map(|x, y, _| {
			if (5..10).contains(&x) && (5..10).contains(&y) {
				[1.0; 4]
			} else {
				[0.0; 4]
			}
		});
		surface
	}

	#[test]
	fn drop_shadow_lands_behind_and_offset() {
		let mut surface = square();
		drop_shadow(
			&mut surface,
			&ctx(),
			Shadow {
				color: [0.0; 3],
				opacity: 1.0,
				direction: 180.0,
				distance: 6.0,
				softness: 0.0,
				shadow_only: false,
			},
		);
		// 真下に 6px
		assert_eq!(surface.get(7, 13), [0.0, 0.0, 0.0, 1.0]);
		assert_eq!(surface.get(7, 7), [1.0; 4]);
		assert_eq!(surface.get(7, 2)[3], 0.0);
	}

	#[test]
	fn mosaic_averages_blocks() {
		let mut surface = Surface::new(4, 1);
		surface.pixels = vec![[1.0; 4], [0.0, 0.0, 0.0, 1.0], [1.0; 4], [1.0; 4]];
		mosaic(&mut surface, 2.0, 1.0, false);
		assert_eq!(surface.get(0, 0), [0.5, 0.5, 0.5, 1.0]);
		assert_eq!(surface.get(3, 0), [1.0; 4]);
	}

	#[test]
	fn glow_brightens_around_highlights() {
		let mut surface = square();
		glow(&mut surface, &ctx(), 0.5, 6.0, 1.0);
		assert!(surface.get(12, 7)[3] > 0.0, "glow spreads outside the layer");
	}

	#[test]
	fn find_edges_marks_borders() {
		let mut surface = Surface::new(6, 1);
		surface.pixels = vec![
			[0.0, 0.0, 0.0, 1.0],
			[0.0, 0.0, 0.0, 1.0],
			[0.0, 0.0, 0.0, 1.0],
			[1.0; 4],
			[1.0; 4],
			[1.0; 4],
		];
		find_edges(&mut surface, true, 0.0);
		assert!(surface.get(0, 0)[0] < 0.01);
		assert!(surface.get(3, 0)[0] > 0.5);
	}
}
