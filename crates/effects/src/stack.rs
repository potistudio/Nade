//! Runs a layer's evaluated effect stack over its rendered pixels.

use nade_domain::{EffectKind, EvaluatedEffect};

use crate::surface::Surface;
use crate::{blur, color, distort, generate, stylize, transition};

/// Frame-wide inputs of the effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectContext {
	/// Output pixels per composition pixel (preview resolution).
	pub scale: f32,
	/// Composition time in seconds.
	pub time: f32,
	/// Surface size in output pixels.
	pub width: usize,
	pub height: usize,
}

impl EffectContext {
	/// Output pixel of a composition-space point (origin at the center).
	#[inline]
	pub fn point(&self, point: [f32; 2]) -> [f32; 2] {
		[
			self.width as f32 / 2.0 + point[0] * self.scale,
			self.height as f32 / 2.0 + point[1] * self.scale,
		]
	}
}

/// Apply `effects` in order.
pub fn apply_stack(surface: &mut Surface, effects: &[EvaluatedEffect], ctx: &EffectContext) {
	for effect in effects {
		apply(surface, effect, ctx);
	}
}

/// Apply one effect.
pub fn apply(surface: &mut Surface, effect: &EvaluatedEffect, ctx: &EffectContext) {
	if surface.width == 0 || surface.height == 0 {
		return;
	}
	let v = |slot: usize| effect.value(slot);
	let index = |slot: usize| v(slot).round().max(0.0) as usize;
	let flag = |slot: usize| v(slot) >= 0.5;
	let point = |slot: usize| [v(slot), v(slot + 1)];
	let rgb = |slot: usize| [v(slot), v(slot + 1), v(slot + 2)];
	match effect.kind {
		EffectKind::GaussianBlur => blur::gaussian_blur(surface, ctx, v(0), index(1)),
		EffectKind::DirectionalBlur => blur::directional_blur(surface, ctx, v(0), v(1)),
		EffectKind::RadialBlur => blur::radial_blur(surface, ctx, v(0), point(1), index(3) == 1),
		EffectKind::Sharpen => blur::sharpen(surface, v(0)),
		EffectKind::BrightnessContrast => color::brightness_contrast(surface, v(0), v(1)),
		EffectKind::HueSaturation => color::hue_saturation(surface, v(0), v(1), v(2)),
		EffectKind::Levels => color::levels(surface, [v(0), v(1)], v(2), [v(3), v(4)]),
		EffectKind::Exposure => color::exposure(surface, v(0), v(1), v(2)),
		EffectKind::Tint => color::tint(surface, rgb(0), rgb(3), v(6)),
		EffectKind::BlackAndWhite => color::black_and_white(surface, [v(0), v(1), v(2), v(3), v(4), v(5)]),
		EffectKind::Invert => color::invert(surface, v(0)),
		EffectKind::Posterize => color::posterize(surface, v(0)),
		EffectKind::Threshold => color::threshold(surface, v(0)),
		EffectKind::Vignette => color::vignette(surface, v(0), v(1), v(2)),
		EffectKind::WaveWarp => distort::wave_warp(
			surface,
			ctx,
			distort::WaveWarp {
				shape: index(0),
				height: v(1),
				width: v(2),
				direction: v(3),
				speed: v(4),
				phase: v(5),
			},
		),
		EffectKind::Twirl => distort::twirl(surface, ctx, v(0), v(1), point(2)),
		EffectKind::Bulge => distort::bulge(surface, ctx, v(0), v(1), point(2)),
		EffectKind::Offset => distort::offset(surface, ctx, point(0), v(2)),
		EffectKind::Mirror => distort::mirror(surface, ctx, point(0), v(2)),
		EffectKind::Fill => generate::fill(surface, rgb(0), v(3)),
		EffectKind::GradientRamp => generate::gradient_ramp(
			surface,
			ctx,
			generate::Ramp {
				start: point(0),
				start_color: rgb(2),
				end: point(5),
				end_color: rgb(7),
				radial: index(10) == 1,
				blend: v(11),
			},
		),
		EffectKind::Noise => generate::noise(surface, ctx, v(0), flag(1), flag(2)),
		EffectKind::DropShadow => stylize::drop_shadow(
			surface,
			ctx,
			stylize::Shadow {
				color: rgb(0),
				opacity: v(3),
				direction: v(4),
				distance: v(5),
				softness: v(6),
				shadow_only: flag(7),
			},
		),
		EffectKind::Glow => stylize::glow(surface, ctx, v(0), v(1), v(2)),
		EffectKind::Mosaic => stylize::mosaic(surface, v(0), v(1), flag(2)),
		EffectKind::FindEdges => stylize::find_edges(surface, flag(0), v(1)),
		EffectKind::Emboss => stylize::emboss(surface, ctx, v(0), v(1), v(2), v(3)),
		EffectKind::LinearWipe => transition::linear_wipe(surface, ctx, v(0), v(1), v(2)),
		EffectKind::RadialWipe => transition::radial_wipe(surface, ctx, v(0), v(1), point(2), index(4), v(5)),
		EffectKind::VenetianBlinds => transition::venetian_blinds(surface, ctx, v(0), v(1), v(2), v(3)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use nade_domain::LayerEffect;

	/// Every effect runs with its defaults and keeps the surface valid.
	#[test]
	fn every_effect_runs_on_defaults() {
		let ctx = EffectContext {
			scale: 0.5,
			time: 0.25,
			width: 32,
			height: 24,
		};
		for kind in EffectKind::ALL {
			let mut surface = Surface::new(32, 24);
			surface.map(|x, y, _| {
				if (8..24).contains(&x) && (6..18).contains(&y) {
					[x as f32 / 32.0, y as f32 / 24.0, 0.5, 1.0]
				} else {
					[0.0; 4]
				}
			});
			apply(&mut surface, &LayerEffect::new(0, kind).evaluate(0.0), &ctx);
			assert_eq!(surface.pixels.len(), 32 * 24, "{kind}");
			for pixel in &surface.pixels {
				assert!(pixel.iter().all(|channel| channel.is_finite()), "{kind}: {pixel:?}");
				assert!((-1e-4..=1.0 + 1e-4).contains(&pixel[3]), "{kind}: alpha {pixel:?}");
			}
		}
	}

	/// The slot layout `apply` reads matches the parameter definitions.
	#[test]
	fn slot_layout_matches_params() {
		let slot_of = |kind: EffectKind, name: &str| {
			let index = kind.params().iter().position(|param| param.name == name).unwrap();
			kind.slot_range(index).start
		};
		assert_eq!(slot_of(EffectKind::GradientRamp, "End Point"), 5);
		assert_eq!(slot_of(EffectKind::GradientRamp, "Shape"), 10);
		assert_eq!(slot_of(EffectKind::GradientRamp, "Blend"), 11);
		assert_eq!(slot_of(EffectKind::DropShadow, "Shadow Only"), 7);
		assert_eq!(slot_of(EffectKind::RadialWipe, "Wipe"), 4);
		assert_eq!(slot_of(EffectKind::RadialWipe, "Feather"), 5);
		assert_eq!(slot_of(EffectKind::Mirror, "Angle"), 2);
		assert_eq!(slot_of(EffectKind::Offset, "Blend"), 2);
		assert_eq!(slot_of(EffectKind::Tint, "Amount"), 6);
		assert_eq!(slot_of(EffectKind::RadialBlur, "Type"), 3);
		assert_eq!(slot_of(EffectKind::Fill, "Opacity"), 3);
	}
}
