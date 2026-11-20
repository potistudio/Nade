//! Distort effects: every output pixel samples the layer somewhere else.

use std::f32::consts::{PI, TAU};

use crate::stack::EffectContext;
use crate::surface::{Surface, lerp4};

/// Wave shapes of Wave Warp, all in -1..1 for a phase in radians.
fn wave(shape: usize, phase: f32) -> f32 {
	match shape {
		1 => {
			if phase.sin() >= 0.0 {
				1.0
			} else {
				-1.0
			}
		}
		2 => 2.0 / PI * phase.sin().asin(),
		3 => {
			let t = phase / TAU;
			2.0 * (t - (t + 0.5).floor())
		}
		_ => phase.sin(),
	}
}

pub struct WaveWarp {
	pub shape: usize,
	pub height: f32,
	pub width: f32,
	pub direction: f32,
	pub speed: f32,
	pub phase: f32,
}

/// Wave Warp: waves travel along `direction` and displace across it.
pub fn wave_warp(surface: &mut Surface, ctx: &EffectContext, params: WaveWarp) {
	let height = params.height * ctx.scale;
	let width = (params.width * ctx.scale).max(1.0);
	if height.abs() < 0.01 {
		return;
	}
	let theta = params.direction.to_radians();
	let (dx, dy) = (theta.sin(), -theta.cos());
	let (nx, ny) = (-dy, dx);
	let phase = params.phase.to_radians() - TAU * params.speed * ctx.time;
	let origin = ctx.point([0.0, 0.0]);
	surface.warp(|x, y| {
		let along = (x - origin[0]) * dx + (y - origin[1]) * dy;
		let offset = height * wave(params.shape, TAU * along / width + phase);
		(x - nx * offset, y - ny * offset)
	});
}

/// Twirl: rotate by `angle` at the center, fading to none at the radius.
pub fn twirl(surface: &mut Surface, ctx: &EffectContext, angle: f32, radius: f32, center: [f32; 2]) {
	let [cx, cy] = ctx.point(center);
	let radius = radius * surface.width.min(surface.height) as f32;
	if radius <= 0.0 || angle == 0.0 {
		return;
	}
	let angle = angle.to_radians();
	surface.warp(|x, y| {
		let (vx, vy) = (x - cx, y - cy);
		let distance = (vx * vx + vy * vy).sqrt();
		if distance >= radius {
			return (x, y);
		}
		let t = 1.0 - distance / radius;
		let (sin, cos) = (-angle * t * t).sin_cos();
		(cx + vx * cos - vy * sin, cy + vx * sin + vy * cos)
	});
}

/// Bulge: magnify (positive height) or pinch inside `radius`.
pub fn bulge(surface: &mut Surface, ctx: &EffectContext, radius: f32, height: f32, center: [f32; 2]) {
	let [cx, cy] = ctx.point(center);
	let radius = radius * ctx.scale;
	if radius <= 0.0 || height == 0.0 {
		return;
	}
	surface.warp(|x, y| {
		let (vx, vy) = (x - cx, y - cy);
		let r = (vx * vx + vy * vy).sqrt() / radius;
		if r >= 1.0 {
			return (x, y);
		}
		let falloff = (1.0 - r * r) * (1.0 - r * r) * height;
		let k = if falloff >= 0.0 {
			1.0 / (1.0 + falloff)
		} else {
			1.0 - falloff
		};
		(cx + vx * k, cy + vy * k)
	});
}

/// Offset: scroll the layer (wrapping around) by `shift` composition pixels.
pub fn offset(surface: &mut Surface, ctx: &EffectContext, shift: [f32; 2], blend: f32) {
	let (sx, sy) = (shift[0] * ctx.scale, shift[1] * ctx.scale);
	if sx.abs() < 1e-3 && sy.abs() < 1e-3 {
		return;
	}
	let source = surface.clone();
	*surface = source.generate(|x, y| {
		let shifted = source.sample_wrapped(x as f32 + 0.5 - sx, y as f32 + 0.5 - sy);
		lerp4(shifted, source.get(x, y), blend)
	});
}

/// Mirror: reflect the side the normal points to across the line through
/// `center` at `angle`.
pub fn mirror(surface: &mut Surface, ctx: &EffectContext, center: [f32; 2], angle: f32) {
	let [cx, cy] = ctx.point(center);
	let (ny, nx) = angle.to_radians().sin_cos();
	surface.warp(|x, y| {
		let side = (x - cx) * nx + (y - cy) * ny;
		if side <= 0.0 {
			(x, y)
		} else {
			(x - 2.0 * side * nx, y - 2.0 * side * ny)
		}
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
			height: 10,
		}
	}

	fn gradient() -> Surface {
		let mut surface = Surface::new(20, 10);
		surface.map(|x, _, _| [x as f32 / 20.0, 0.0, 0.0, 1.0]);
		surface
	}

	#[test]
	fn mirror_reflects_the_right_half() {
		let mut surface = gradient();
		mirror(&mut surface, &ctx(), [0.0, 0.0], 0.0);
		assert_eq!(surface.get(12, 5), gradient().get(7, 5));
		assert_eq!(surface.get(3, 5), gradient().get(3, 5));
	}

	#[test]
	fn offset_wraps_around() {
		let mut surface = gradient();
		offset(&mut surface, &ctx(), [5.0, 0.0], 0.0);
		assert_eq!(surface.get(5, 0), gradient().get(0, 0));
		assert_eq!(surface.get(2, 0), gradient().get(17, 0));
	}

	#[test]
	fn waves_stay_in_range() {
		for shape in 0..4 {
			for step in 0..32 {
				let value = wave(shape, step as f32 * 0.4);
				assert!((-1.0..=1.0).contains(&value), "shape {shape}: {value}");
			}
		}
	}

	#[test]
	fn twirl_keeps_outside_pixels() {
		let mut surface = gradient();
		twirl(&mut surface, &ctx(), 90.0, 0.2, [0.0, 0.0]);
		assert_eq!(surface.get(0, 0), gradient().get(0, 0));
		assert_ne!(surface.get(11, 5), gradient().get(11, 5));
	}
}
