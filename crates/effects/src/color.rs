//! Color Correction effects. They work on straight (unpremultiplied) color
//! and keep the layer's alpha.

use crate::surface::{Surface, lerp, luma, premultiply, smoothstep, unpremultiply};

/// Apply `grade` to the straight color of every visible pixel.
pub fn grade(surface: &mut Surface, grade: impl Fn([f32; 3]) -> [f32; 3] + Sync) {
	surface.map(|_, _, pixel| {
		if pixel[3] <= 0.0 {
			return pixel;
		}
		let (color, alpha) = unpremultiply(pixel);
		premultiply(grade(color), alpha)
	});
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
	[lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

/// Brightness & Contrast (AE ranges: -150..150, -100..100).
pub fn brightness_contrast(surface: &mut Surface, brightness: f32, contrast: f32) {
	let offset = brightness / 255.0;
	let factor = if contrast >= 0.0 {
		1.0 / (1.0 - contrast / 100.0 * 0.99)
	} else {
		1.0 + contrast / 100.0
	};
	grade(surface, |color| color.map(|c| (c + offset - 0.5) * factor + 0.5));
}

fn rgb_to_hsl([r, g, b]: [f32; 3]) -> [f32; 3] {
	let max = r.max(g).max(b);
	let min = r.min(g).min(b);
	let l = (max + min) / 2.0;
	let d = max - min;
	if d <= 1e-6 {
		return [0.0, 0.0, l];
	}
	let s = if l > 0.5 {
		d / (2.0 - max - min)
	} else {
		d / (max + min)
	};
	let h = if max == r {
		(g - b) / d + if g < b { 6.0 } else { 0.0 }
	} else if max == g {
		(b - r) / d + 2.0
	} else {
		(r - g) / d + 4.0
	};
	[h / 6.0, s, l]
}

fn hsl_to_rgb([h, s, l]: [f32; 3]) -> [f32; 3] {
	if s <= 1e-6 {
		return [l; 3];
	}
	let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
	let p = 2.0 * l - q;
	let channel = |t: f32| {
		let t = t.rem_euclid(1.0);
		if t < 1.0 / 6.0 {
			p + (q - p) * 6.0 * t
		} else if t < 0.5 {
			q
		} else if t < 2.0 / 3.0 {
			p + (q - p) * (2.0 / 3.0 - t) * 6.0
		} else {
			p
		}
	};
	[channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
}

/// Hue/Saturation: master hue (degrees), saturation and lightness (-100..100).
pub fn hue_saturation(surface: &mut Surface, hue: f32, saturation: f32, lightness: f32) {
	let shift = hue / 360.0;
	let saturation = 1.0 + saturation / 100.0;
	let lightness = lightness / 100.0;
	grade(surface, |color| {
		let [h, s, l] = rgb_to_hsl(color);
		let color = hsl_to_rgb([h + shift, (s * saturation).clamp(0.0, 1.0), l]);
		color.map(|c| {
			if lightness >= 0.0 {
				c + (1.0 - c) * lightness
			} else {
				c * (1.0 + lightness)
			}
		})
	});
}

/// Levels (0..255 ranges like AE in 8 bpc).
pub fn levels(surface: &mut Surface, input: [f32; 2], gamma: f32, output: [f32; 2]) {
	let [in_black, in_white] = input.map(|value| value / 255.0);
	let [out_black, out_white] = output.map(|value| value / 255.0);
	let span = (in_white - in_black).max(1e-4);
	let inv_gamma = 1.0 / gamma.max(0.01);
	grade(surface, |color| {
		color.map(|c| {
			let t = ((c - in_black) / span).clamp(0.0, 1.0).powf(inv_gamma);
			lerp(out_black, out_white, t)
		})
	});
}

fn to_linear(c: f32) -> f32 {
	if c <= 0.04045 {
		c / 12.92
	} else {
		((c + 0.055) / 1.055).powf(2.4)
	}
}

fn to_srgb(c: f32) -> f32 {
	if c <= 0.0031308 {
		c * 12.92
	} else {
		1.055 * c.powf(1.0 / 2.4) - 0.055
	}
}

/// Exposure: stops and offset in linear light, then gamma.
pub fn exposure(surface: &mut Surface, stops: f32, offset: f32, gamma: f32) {
	let gain = 2.0_f32.powf(stops);
	let inv_gamma = 1.0 / gamma.max(0.01);
	grade(surface, |color| {
		color.map(|c| {
			let linear = (to_linear(c) * gain + offset).max(0.0);
			to_srgb(linear).clamp(0.0, 1.0).powf(inv_gamma)
		})
	});
}

/// Tint: map luminance onto a black → white color ramp.
pub fn tint(surface: &mut Surface, black: [f32; 3], white: [f32; 3], amount: f32) {
	grade(surface, |color| {
		let mapped = mix3(black, white, luma(color));
		mix3(color, mapped, amount)
	});
}

/// Black & White with per-hue weights (reds, yellows, greens, cyans, blues,
/// magentas), Photoshop style.
pub fn black_and_white(surface: &mut Surface, weights: [f32; 6]) {
	let [reds, yellows, greens, cyans, blues, magentas] = weights;
	grade(surface, |[r, g, b]| {
		let max = r.max(g).max(b);
		let min = r.min(g).min(b);
		let mid = r + g + b - max - min;
		// 最大チャンネルが原色、最大+中間が二次色の重みになる
		let (primary, secondary) = if max == r {
			(reds, if g >= b { yellows } else { magentas })
		} else if max == g {
			(greens, if r >= b { yellows } else { cyans })
		} else {
			(blues, if r >= g { magentas } else { cyans })
		};
		let gray = (min + (mid - min) * secondary + (max - mid) * primary).clamp(0.0, 1.0);
		[gray; 3]
	});
}

/// Invert, blended with the original.
pub fn invert(surface: &mut Surface, blend: f32) {
	grade(surface, |color| mix3(color.map(|c| 1.0 - c), color, blend));
}

/// Posterize to `levels` steps per channel.
pub fn posterize(surface: &mut Surface, levels: f32) {
	let steps = (levels.round().max(2.0)) - 1.0;
	grade(surface, |color| color.map(|c| (c * steps).round() / steps));
}

/// Threshold: white above `level` (0..255), black below.
pub fn threshold(surface: &mut Surface, level: f32) {
	let level = level / 255.0;
	grade(surface, |color| [if luma(color) >= level { 1.0 } else { 0.0 }; 3]);
}

/// Vignette: darken (positive amount) or lighten towards the frame edges.
pub fn vignette(surface: &mut Surface, amount: f32, midpoint: f32, feather: f32) {
	let (hw, hh) = (surface.width as f32 / 2.0, surface.height as f32 / 2.0);
	let inner = midpoint - feather / 2.0;
	let outer = midpoint + feather / 2.0;
	surface.map(|x, y, pixel| {
		if pixel[3] <= 0.0 {
			return pixel;
		}
		let dx = (x as f32 + 0.5 - hw) / hw.max(1.0);
		let dy = (y as f32 + 0.5 - hh) / hh.max(1.0);
		let distance = (dx * dx + dy * dy).sqrt() / std::f32::consts::SQRT_2;
		let v = smoothstep(inner, outer, distance);
		let (color, alpha) = unpremultiply(pixel);
		let color = color.map(|c| {
			if amount >= 0.0 {
				c * (1.0 - v * amount)
			} else {
				c + (1.0 - c) * v * -amount
			}
		});
		premultiply(color, alpha)
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	fn single(color: [f32; 3]) -> Surface {
		let mut surface = Surface::new(1, 1);
		surface.pixels[0] = premultiply(color, 1.0);
		surface
	}

	fn rgb(surface: &Surface) -> [f32; 3] {
		unpremultiply(surface.pixels[0]).0
	}

	#[test]
	fn hsl_round_trips() {
		for color in [[1.0, 0.0, 0.0], [0.2, 0.6, 0.4], [0.9, 0.9, 0.1]] {
			let back = hsl_to_rgb(rgb_to_hsl(color));
			for channel in 0..3 {
				assert!((back[channel] - color[channel]).abs() < 1e-4, "{color:?} → {back:?}");
			}
		}
	}

	#[test]
	fn hue_rotates_red_to_green() {
		let mut surface = single([1.0, 0.0, 0.0]);
		hue_saturation(&mut surface, 120.0, 0.0, 0.0);
		let [r, g, b] = rgb(&surface);
		assert!(r < 0.01 && g > 0.99 && b < 0.01);
	}

	#[test]
	fn invert_and_threshold() {
		let mut surface = single([0.25, 0.5, 1.0]);
		invert(&mut surface, 0.0);
		let [r, g, b] = rgb(&surface);
		assert!((r - 0.75).abs() < 1e-5 && (g - 0.5).abs() < 1e-5 && b.abs() < 1e-5);
		threshold(&mut surface, 128.0);
		assert_eq!(rgb(&surface), [1.0; 3]);
	}

	#[test]
	fn transparent_pixels_stay_transparent() {
		let mut surface = Surface::new(2, 1);
		brightness_contrast(&mut surface, 150.0, 50.0);
		assert_eq!(surface.pixels, vec![[0.0; 4]; 2]);
	}

	#[test]
	fn levels_and_exposure_identity() {
		let mut surface = single([0.3, 0.6, 0.9]);
		levels(&mut surface, [0.0, 255.0], 1.0, [0.0, 255.0]);
		exposure(&mut surface, 0.0, 0.0, 1.0);
		let [r, g, b] = rgb(&surface);
		assert!((r - 0.3).abs() < 1e-4 && (g - 0.6).abs() < 1e-4 && (b - 0.9).abs() < 1e-4);
	}

	#[test]
	fn black_and_white_defaults_match_weights() {
		let mut surface = single([1.0, 0.0, 0.0]);
		black_and_white(&mut surface, [0.4, 0.6, 0.4, 0.6, 0.2, 0.8]);
		assert!((rgb(&surface)[0] - 0.4).abs() < 1e-5);
	}
}
