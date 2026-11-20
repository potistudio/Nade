//! Transition effects: animated alpha masks.

use crate::stack::EffectContext;
use crate::surface::{Surface, scale4};

/// Anti-aliased step: 0 at `distance <= -feather/2`, 1 at `>= feather/2`.
#[inline]
fn soft(distance: f32, feather: f32) -> f32 {
	(distance / feather.max(1.0) + 0.5).clamp(0.0, 1.0)
}

fn mask(surface: &mut Surface, coverage: impl Fn(f32, f32) -> f32 + Sync) {
	surface.map(|x, y, pixel| {
		if pixel[3] <= 0.0 {
			return pixel;
		}
		scale4(pixel, coverage(x as f32 + 0.5, y as f32 + 0.5))
	});
}

/// Linear Wipe: the layer disappears along `angle` (90° = from the left).
pub fn linear_wipe(surface: &mut Surface, ctx: &EffectContext, completion: f32, angle: f32, feather: f32) {
	if completion <= 0.0 {
		return;
	}
	let theta = angle.to_radians();
	let (dx, dy) = (theta.sin(), -theta.cos());
	let [cx, cy] = ctx.point([0.0, 0.0]);
	let extent = (surface.width as f32 / 2.0 * dx).abs() + (surface.height as f32 / 2.0 * dy).abs();
	let feather = feather * ctx.scale;
	let edge = -extent - feather / 2.0 + completion * (2.0 * extent + feather);
	mask(surface, |x, y| {
		let along = (x - cx) * dx + (y - cy) * dy;
		soft(along - edge, feather)
	});
}

/// Radial Wipe around `center`, starting at `start` degrees (0° = up).
pub fn radial_wipe(
	surface: &mut Surface,
	ctx: &EffectContext,
	completion: f32,
	start: f32,
	center: [f32; 2],
	mode: usize,
	feather: f32,
) {
	if completion <= 0.0 {
		return;
	}
	if completion >= 1.0 {
		surface.map(|_, _, _| [0.0; 4]);
		return;
	}
	let [cx, cy] = ctx.point(center);
	let feather = feather * ctx.scale;
	mask(surface, |x, y| {
		let (vx, vy) = (x - cx, y - cy);
		let radius = (vx * vx + vy * vy).sqrt();
		// 画面座標（y 下向き）で上から時計回りの角度
		let angle = (vx.atan2(-vy).to_degrees() - start).rem_euclid(360.0);
		let arc = |degrees: f32| degrees.to_radians() * radius;
		match mode {
			1 => {
				let angle = (360.0 - angle).rem_euclid(360.0);
				soft(arc(angle - completion * 360.0), feather) * soft(arc(360.0 - angle), feather)
			}
			2 => {
				let from_start = angle.min(360.0 - angle);
				soft(arc(from_start - completion * 180.0), feather)
			}
			_ => soft(arc(angle - completion * 360.0), feather) * soft(arc(360.0 - angle), feather),
		}
	});
}

/// Venetian Blinds: stripes `width` apart close as `completion` grows.
pub fn venetian_blinds(
	surface: &mut Surface,
	ctx: &EffectContext,
	completion: f32,
	direction: f32,
	width: f32,
	feather: f32,
) {
	if completion <= 0.0 {
		return;
	}
	let width = (width * ctx.scale).max(1.0);
	let theta = direction.to_radians();
	let (dx, dy) = (-theta.sin(), theta.cos());
	let [cx, cy] = ctx.point([0.0, 0.0]);
	let feather = feather * ctx.scale;
	let closed = completion * width;
	mask(surface, |x, y| {
		let local = ((x - cx) * dx + (y - cy) * dy).rem_euclid(width);
		soft(local - closed, feather) * soft(width - local, feather)
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

	fn opaque() -> Surface {
		let mut surface = Surface::new(20, 20);
		surface.map(|_, _, _| [1.0; 4]);
		surface
	}

	#[test]
	fn linear_wipe_clears_from_the_left() {
		let mut surface = opaque();
		linear_wipe(&mut surface, &ctx(), 0.5, 90.0, 0.0);
		assert_eq!(surface.get(2, 10)[3], 0.0);
		assert_eq!(surface.get(17, 10)[3], 1.0);

		let mut done = opaque();
		linear_wipe(&mut done, &ctx(), 1.0, 90.0, 0.0);
		assert!(done.pixels.iter().all(|pixel| pixel[3] == 0.0));
	}

	#[test]
	fn radial_wipe_clears_clockwise_from_the_top() {
		let mut surface = opaque();
		radial_wipe(&mut surface, &ctx(), 0.25, 0.0, [0.0, 0.0], 0, 0.0);
		// 右上（0..90°）が消え、左上は残る
		assert_eq!(surface.get(15, 4)[3], 0.0);
		assert_eq!(surface.get(4, 4)[3], 1.0);
		assert_eq!(surface.get(15, 15)[3], 1.0);
	}

	#[test]
	fn blinds_close_in_stripes() {
		let mut surface = opaque();
		venetian_blinds(&mut surface, &ctx(), 0.5, 0.0, 10.0, 0.0);
		let covered: f32 = surface.pixels.iter().map(|pixel| pixel[3]).sum();
		assert!((covered / 400.0 - 0.5).abs() < 0.1, "about half remains: {covered}");
	}
}
