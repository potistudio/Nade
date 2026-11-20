//! Generate and Noise & Grain effects. They paint inside the layer's alpha.

use crate::stack::EffectContext;
use crate::surface::{Surface, lerp, premultiply, unpremultiply};

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
	[lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

/// Fill: replace the layer's color, keeping its alpha.
pub fn fill(surface: &mut Surface, color: [f32; 3], opacity: f32) {
	surface.map(|_, _, pixel| {
		if pixel[3] <= 0.0 {
			return pixel;
		}
		let (original, alpha) = unpremultiply(pixel);
		premultiply(mix3(original, color, opacity), alpha)
	});
}

pub struct Ramp {
	pub start: [f32; 2],
	pub start_color: [f32; 3],
	pub end: [f32; 2],
	pub end_color: [f32; 3],
	pub radial: bool,
	pub blend: f32,
}

/// Gradient Ramp between two composition points.
pub fn gradient_ramp(surface: &mut Surface, ctx: &EffectContext, ramp: Ramp) {
	let [sx, sy] = ctx.point(ramp.start);
	let [ex, ey] = ctx.point(ramp.end);
	let (dx, dy) = (ex - sx, ey - sy);
	let length_sq = (dx * dx + dy * dy).max(1e-6);
	let length = length_sq.sqrt();
	surface.map(|x, y, pixel| {
		if pixel[3] <= 0.0 {
			return pixel;
		}
		let (px, py) = (x as f32 + 0.5 - sx, y as f32 + 0.5 - sy);
		let t = if ramp.radial {
			(px * px + py * py).sqrt() / length
		} else {
			(px * dx + py * dy) / length_sq
		};
		let (original, alpha) = unpremultiply(pixel);
		let color = mix3(ramp.start_color, ramp.end_color, t.clamp(0.0, 1.0));
		premultiply(mix3(color, original, ramp.blend), alpha)
	});
}

/// Integer hash → 0..1.
#[inline]
fn hash(mut value: u32) -> f32 {
	value ^= value >> 16;
	value = value.wrapping_mul(0x7feb_352d);
	value ^= value >> 15;
	value = value.wrapping_mul(0x846c_a68b);
	value ^= value >> 16;
	(value >> 8) as f32 / (1u32 << 24) as f32
}

/// Noise: random per-pixel grain, new every frame when animated.
pub fn noise(surface: &mut Surface, ctx: &EffectContext, amount: f32, color_noise: bool, animated: bool) {
	if amount <= 0.0 {
		return;
	}
	let seed = if animated {
		(ctx.time * 1000.0).round() as i64 as u32
	} else {
		0
	}
	.wrapping_mul(0x9e37_79b9);
	let width = surface.width as u32;
	surface.map(|x, y, pixel| {
		if pixel[3] <= 0.0 {
			return pixel;
		}
		let base = (y as u32).wrapping_mul(width).wrapping_add(x as u32).wrapping_mul(3) ^ seed;
		let (color, alpha) = unpremultiply(pixel);
		let grain = |channel: u32| (hash(base.wrapping_add(if color_noise { channel } else { 0 })) - 0.5) * amount;
		premultiply([color[0] + grain(0), color[1] + grain(1), color[2] + grain(2)], alpha)
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	fn ctx() -> EffectContext {
		EffectContext {
			scale: 1.0,
			time: 0.0,
			width: 10,
			height: 10,
		}
	}

	#[test]
	fn ramp_runs_from_start_to_end_color() {
		let mut surface = Surface::new(10, 10);
		surface.map(|_, _, _| [1.0; 4]);
		gradient_ramp(
			&mut surface,
			&ctx(),
			Ramp {
				start: [-5.0, 0.0],
				start_color: [0.0; 3],
				end: [5.0, 0.0],
				end_color: [1.0; 3],
				radial: false,
				blend: 0.0,
			},
		);
		assert!(surface.get(0, 5)[0] < 0.1);
		assert!(surface.get(9, 5)[0] > 0.9);
	}

	#[test]
	fn noise_changes_with_time_only_when_animated() {
		let mut base = Surface::new(8, 8);
		base.map(|_, _, _| [0.5, 0.5, 0.5, 1.0]);
		let run = |time: f32, animated: bool| {
			let mut surface = base.clone();
			let ctx = EffectContext { time, ..ctx() };
			noise(&mut surface, &ctx, 0.5, true, animated);
			surface
		};
		assert_ne!(run(0.0, true), run(1.0, true));
		assert_eq!(run(0.0, false), run(1.0, false));
		assert_ne!(run(0.0, false), base);
	}

	#[test]
	fn fill_keeps_alpha() {
		let mut surface = Surface::new(1, 1);
		surface.pixels[0] = [0.1, 0.1, 0.1, 0.5];
		fill(&mut surface, [1.0, 0.0, 0.0], 1.0);
		assert_eq!(surface.pixels[0], [0.5, 0.0, 0.0, 0.5]);
	}
}
