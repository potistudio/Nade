//! Layer effects (After Effects style effect stack).
//!
//! An effect is a [`EffectKind`] plus one animatable scalar per parameter
//! slot. Vector parameters (points, colors) span several consecutive slots so
//! every channel keys like a transform channel does.

use nade_core::AnimationCurve;
use std::ops::Range;

/// Effects & Presets category an effect is listed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectCategory {
	BlurSharpen,
	ColorCorrection,
	Distort,
	Generate,
	NoiseGrain,
	Perspective,
	Stylize,
	Transition,
}

impl EffectCategory {
	pub const ALL: [Self; 8] = [
		Self::BlurSharpen,
		Self::ColorCorrection,
		Self::Distort,
		Self::Generate,
		Self::NoiseGrain,
		Self::Perspective,
		Self::Stylize,
		Self::Transition,
	];

	pub fn label(self) -> &'static str {
		match self {
			Self::BlurSharpen => "Blur & Sharpen",
			Self::ColorCorrection => "Color Correction",
			Self::Distort => "Distort",
			Self::Generate => "Generate",
			Self::NoiseGrain => "Noise & Grain",
			Self::Perspective => "Perspective",
			Self::Stylize => "Stylize",
			Self::Transition => "Transition",
		}
	}

	/// Effects of this category in menu order.
	pub fn effects(self) -> impl Iterator<Item = EffectKind> {
		EffectKind::ALL.into_iter().filter(move |kind| kind.category() == self)
	}
}

/// Every effect the renderer knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EffectKind {
	GaussianBlur,
	DirectionalBlur,
	RadialBlur,
	Sharpen,
	BrightnessContrast,
	HueSaturation,
	Levels,
	Exposure,
	Tint,
	BlackAndWhite,
	Invert,
	Posterize,
	Threshold,
	Vignette,
	WaveWarp,
	Twirl,
	Bulge,
	Offset,
	Mirror,
	Fill,
	GradientRamp,
	Noise,
	DropShadow,
	Glow,
	Mosaic,
	FindEdges,
	Emboss,
	LinearWipe,
	RadialWipe,
	VenetianBlinds,
}

/// How a parameter is edited and stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamKind {
	/// Plain number clamped to `min..=max`, shown with `suffix`.
	Number {
		min: f32,
		max: f32,
		step: f32,
		precision: usize,
		suffix: &'static str,
	},
	/// Fraction stored as 0..1, shown as 0..100 %.
	Percent { min: f32, max: f32 },
	/// Degrees.
	Angle,
	/// Composition pixels, scaled with the preview resolution.
	Pixels { max: f32 },
	/// Point in composition space (origin at the composition center); two slots.
	Point,
	/// Straight RGB 0..1; three slots.
	Color,
	/// One of `options` (stored as its index, not animatable).
	Choice(&'static [&'static str]),
	/// On / off (stored as 0 / 1, not animatable).
	Toggle,
}

impl ParamKind {
	/// Scalar slots the parameter occupies.
	pub fn slots(self) -> usize {
		match self {
			Self::Point => 2,
			Self::Color => 3,
			_ => 1,
		}
	}

	/// Whether the stopwatch applies (discrete values do not interpolate).
	pub fn is_animatable(self) -> bool {
		!matches!(self, Self::Choice(_) | Self::Toggle)
	}

	fn clamp(self, value: f32) -> f32 {
		if !value.is_finite() {
			return 0.0;
		}
		match self {
			Self::Number { min, max, .. } | Self::Percent { min, max } => value.clamp(min, max),
			Self::Pixels { max } => value.clamp(0.0, max),
			Self::Color => value.clamp(0.0, 1.0),
			Self::Choice(options) => value.round().clamp(0.0, options.len().saturating_sub(1) as f32),
			Self::Toggle => {
				if value >= 0.5 {
					1.0
				} else {
					0.0
				}
			}
			Self::Angle | Self::Point => value,
		}
	}
}

/// One row of an effect's controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectParam {
	pub name: &'static str,
	pub kind: ParamKind,
	/// Default value of each slot (`kind.slots()` entries).
	pub default: &'static [f32],
}

const fn number(
	name: &'static str,
	default: &'static [f32],
	min: f32,
	max: f32,
	step: f32,
	precision: usize,
	suffix: &'static str,
) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Number {
			min,
			max,
			step,
			precision,
			suffix,
		},
		default,
	}
}

const fn percent(name: &'static str, default: &'static [f32], min: f32, max: f32) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Percent { min, max },
		default,
	}
}

const fn angle(name: &'static str, default: &'static [f32]) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Angle,
		default,
	}
}

const fn pixels(name: &'static str, default: &'static [f32], max: f32) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Pixels { max },
		default,
	}
}

const fn point(name: &'static str) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Point,
		default: &[0.0, 0.0],
	}
}

const fn color(name: &'static str, default: &'static [f32]) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Color,
		default,
	}
}

const fn choice(name: &'static str, default: &'static [f32], options: &'static [&'static str]) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Choice(options),
		default,
	}
}

const fn toggle(name: &'static str, on: bool) -> EffectParam {
	EffectParam {
		name,
		kind: ParamKind::Toggle,
		default: if on { &[1.0] } else { &[0.0] },
	}
}

const BLEND: EffectParam = percent("Blend", &[0.0], 0.0, 1.0);

impl EffectKind {
	pub const ALL: [Self; 30] = [
		Self::GaussianBlur,
		Self::DirectionalBlur,
		Self::RadialBlur,
		Self::Sharpen,
		Self::BrightnessContrast,
		Self::HueSaturation,
		Self::Levels,
		Self::Exposure,
		Self::Tint,
		Self::BlackAndWhite,
		Self::Invert,
		Self::Posterize,
		Self::Threshold,
		Self::Vignette,
		Self::WaveWarp,
		Self::Twirl,
		Self::Bulge,
		Self::Offset,
		Self::Mirror,
		Self::Fill,
		Self::GradientRamp,
		Self::Noise,
		Self::DropShadow,
		Self::Glow,
		Self::Mosaic,
		Self::FindEdges,
		Self::Emboss,
		Self::LinearWipe,
		Self::RadialWipe,
		Self::VenetianBlinds,
	];

	pub fn name(self) -> &'static str {
		match self {
			Self::GaussianBlur => "Gaussian Blur",
			Self::DirectionalBlur => "Directional Blur",
			Self::RadialBlur => "Radial Blur",
			Self::Sharpen => "Sharpen",
			Self::BrightnessContrast => "Brightness & Contrast",
			Self::HueSaturation => "Hue/Saturation",
			Self::Levels => "Levels",
			Self::Exposure => "Exposure",
			Self::Tint => "Tint",
			Self::BlackAndWhite => "Black & White",
			Self::Invert => "Invert",
			Self::Posterize => "Posterize",
			Self::Threshold => "Threshold",
			Self::Vignette => "Vignette",
			Self::WaveWarp => "Wave Warp",
			Self::Twirl => "Twirl",
			Self::Bulge => "Bulge",
			Self::Offset => "Offset",
			Self::Mirror => "Mirror",
			Self::Fill => "Fill",
			Self::GradientRamp => "Gradient Ramp",
			Self::Noise => "Noise",
			Self::DropShadow => "Drop Shadow",
			Self::Glow => "Glow",
			Self::Mosaic => "Mosaic",
			Self::FindEdges => "Find Edges",
			Self::Emboss => "Emboss",
			Self::LinearWipe => "Linear Wipe",
			Self::RadialWipe => "Radial Wipe",
			Self::VenetianBlinds => "Venetian Blinds",
		}
	}

	pub fn category(self) -> EffectCategory {
		match self {
			Self::GaussianBlur | Self::DirectionalBlur | Self::RadialBlur | Self::Sharpen => {
				EffectCategory::BlurSharpen
			}
			Self::BrightnessContrast
			| Self::HueSaturation
			| Self::Levels
			| Self::Exposure
			| Self::Tint
			| Self::BlackAndWhite
			| Self::Invert
			| Self::Posterize
			| Self::Threshold
			| Self::Vignette => EffectCategory::ColorCorrection,
			Self::WaveWarp | Self::Twirl | Self::Bulge | Self::Offset | Self::Mirror => EffectCategory::Distort,
			Self::Fill | Self::GradientRamp => EffectCategory::Generate,
			Self::Noise => EffectCategory::NoiseGrain,
			Self::DropShadow => EffectCategory::Perspective,
			Self::Glow | Self::Mosaic | Self::FindEdges | Self::Emboss => EffectCategory::Stylize,
			Self::LinearWipe | Self::RadialWipe | Self::VenetianBlinds => EffectCategory::Transition,
		}
	}

	/// Control rows in display order.
	pub fn params(self) -> &'static [EffectParam] {
		match self {
			Self::GaussianBlur => {
				const P: &[EffectParam] = &[
					pixels("Blurriness", &[10.0], 500.0),
					choice(
						"Dimensions",
						&[0.0],
						&["Horizontal and Vertical", "Horizontal", "Vertical"],
					),
				];
				P
			}
			Self::DirectionalBlur => {
				const P: &[EffectParam] = &[angle("Direction", &[0.0]), pixels("Blur Length", &[20.0], 500.0)];
				P
			}
			Self::RadialBlur => {
				const P: &[EffectParam] = &[
					number("Amount", &[10.0], 0.0, 100.0, 0.1, 1, ""),
					point("Center"),
					choice("Type", &[1.0], &["Spin", "Zoom"]),
				];
				P
			}
			Self::Sharpen => {
				const P: &[EffectParam] = &[number("Amount", &[20.0], 0.0, 500.0, 0.5, 0, "")];
				P
			}
			Self::BrightnessContrast => {
				const P: &[EffectParam] = &[
					number("Brightness", &[0.0], -150.0, 150.0, 0.5, 0, ""),
					number("Contrast", &[0.0], -100.0, 100.0, 0.5, 0, ""),
				];
				P
			}
			Self::HueSaturation => {
				const P: &[EffectParam] = &[
					angle("Master Hue", &[0.0]),
					number("Saturation", &[0.0], -100.0, 100.0, 0.5, 0, ""),
					number("Lightness", &[0.0], -100.0, 100.0, 0.5, 0, ""),
				];
				P
			}
			Self::Levels => {
				const P: &[EffectParam] = &[
					number("Input Black", &[0.0], 0.0, 255.0, 0.5, 1, ""),
					number("Input White", &[255.0], 0.0, 255.0, 0.5, 1, ""),
					number("Gamma", &[1.0], 0.1, 10.0, 0.005, 2, ""),
					number("Output Black", &[0.0], 0.0, 255.0, 0.5, 1, ""),
					number("Output White", &[255.0], 0.0, 255.0, 0.5, 1, ""),
				];
				P
			}
			Self::Exposure => {
				const P: &[EffectParam] = &[
					number("Exposure", &[0.0], -20.0, 20.0, 0.01, 2, ""),
					number("Offset", &[0.0], -2.0, 2.0, 0.002, 4, ""),
					number("Gamma", &[1.0], 0.1, 10.0, 0.005, 2, ""),
				];
				P
			}
			Self::Tint => {
				const P: &[EffectParam] = &[
					color("Map Black To", &[0.0, 0.0, 0.0]),
					color("Map White To", &[1.0, 1.0, 1.0]),
					percent("Amount", &[1.0], 0.0, 1.0),
				];
				P
			}
			Self::BlackAndWhite => {
				const P: &[EffectParam] = &[
					percent("Reds", &[0.4], -2.0, 3.0),
					percent("Yellows", &[0.6], -2.0, 3.0),
					percent("Greens", &[0.4], -2.0, 3.0),
					percent("Cyans", &[0.6], -2.0, 3.0),
					percent("Blues", &[0.2], -2.0, 3.0),
					percent("Magentas", &[0.8], -2.0, 3.0),
				];
				P
			}
			Self::Invert => {
				const P: &[EffectParam] = &[BLEND];
				P
			}
			Self::Posterize => {
				const P: &[EffectParam] = &[number("Level", &[6.0], 2.0, 255.0, 0.1, 0, "")];
				P
			}
			Self::Threshold => {
				const P: &[EffectParam] = &[number("Level", &[128.0], 0.0, 255.0, 0.5, 0, "")];
				P
			}
			Self::Vignette => {
				const P: &[EffectParam] = &[
					percent("Amount", &[0.5], -1.0, 1.0),
					percent("Midpoint", &[0.5], 0.0, 1.0),
					percent("Feather", &[0.5], 0.0, 1.0),
				];
				P
			}
			Self::WaveWarp => {
				const P: &[EffectParam] = &[
					choice("Wave Type", &[0.0], &["Sine", "Square", "Triangle", "Sawtooth"]),
					pixels("Wave Height", &[10.0], 2000.0),
					pixels("Wave Width", &[40.0], 4000.0),
					angle("Direction", &[90.0]),
					number("Wave Speed", &[1.0], -50.0, 50.0, 0.01, 2, " /s"),
					angle("Phase", &[0.0]),
				];
				P
			}
			Self::Twirl => {
				const P: &[EffectParam] = &[
					angle("Angle", &[50.0]),
					percent("Radius", &[0.5], 0.0, 2.0),
					point("Center"),
				];
				P
			}
			Self::Bulge => {
				const P: &[EffectParam] = &[
					pixels("Radius", &[200.0], 4000.0),
					number("Height", &[1.0], -4.0, 4.0, 0.005, 2, ""),
					point("Center"),
				];
				P
			}
			Self::Offset => {
				const P: &[EffectParam] = &[point("Shift Center"), BLEND];
				P
			}
			Self::Mirror => {
				const P: &[EffectParam] = &[point("Center"), angle("Angle", &[0.0])];
				P
			}
			Self::Fill => {
				const P: &[EffectParam] = &[color("Color", &[1.0, 0.0, 0.0]), percent("Opacity", &[1.0], 0.0, 1.0)];
				P
			}
			Self::GradientRamp => {
				const P: &[EffectParam] = &[
					EffectParam {
						name: "Start Point",
						kind: ParamKind::Point,
						default: &[0.0, -300.0],
					},
					color("Start Color", &[0.0, 0.0, 0.0]),
					EffectParam {
						name: "End Point",
						kind: ParamKind::Point,
						default: &[0.0, 300.0],
					},
					color("End Color", &[1.0, 1.0, 1.0]),
					choice("Shape", &[0.0], &["Linear", "Radial"]),
					BLEND,
				];
				P
			}
			Self::Noise => {
				const P: &[EffectParam] = &[
					percent("Amount", &[0.2], 0.0, 1.0),
					toggle("Color Noise", true),
					toggle("Animated", true),
				];
				P
			}
			Self::DropShadow => {
				const P: &[EffectParam] = &[
					color("Color", &[0.0, 0.0, 0.0]),
					percent("Opacity", &[0.5], 0.0, 1.0),
					angle("Direction", &[135.0]),
					pixels("Distance", &[5.0], 4000.0),
					pixels("Softness", &[4.0], 500.0),
					toggle("Shadow Only", false),
				];
				P
			}
			Self::Glow => {
				const P: &[EffectParam] = &[
					percent("Threshold", &[0.6], 0.0, 1.0),
					pixels("Radius", &[10.0], 500.0),
					number("Intensity", &[1.0], 0.0, 10.0, 0.005, 2, ""),
				];
				P
			}
			Self::Mosaic => {
				const P: &[EffectParam] = &[
					number("Horizontal", &[10.0], 1.0, 1000.0, 0.1, 0, ""),
					number("Vertical", &[10.0], 1.0, 1000.0, 0.1, 0, ""),
					toggle("Sharp Colors", false),
				];
				P
			}
			Self::FindEdges => {
				const P: &[EffectParam] = &[toggle("Invert", false), BLEND];
				P
			}
			Self::Emboss => {
				const P: &[EffectParam] = &[
					angle("Direction", &[45.0]),
					pixels("Relief", &[1.5], 50.0),
					percent("Contrast", &[1.0], 0.0, 2.0),
					BLEND,
				];
				P
			}
			Self::LinearWipe => {
				const P: &[EffectParam] = &[
					percent("Completion", &[0.0], 0.0, 1.0),
					angle("Wipe Angle", &[90.0]),
					pixels("Feather", &[0.0], 2000.0),
				];
				P
			}
			Self::RadialWipe => {
				const P: &[EffectParam] = &[
					percent("Completion", &[0.0], 0.0, 1.0),
					angle("Start Angle", &[0.0]),
					point("Center"),
					choice("Wipe", &[0.0], &["Clockwise", "Counterclockwise", "Both"]),
					pixels("Feather", &[0.0], 2000.0),
				];
				P
			}
			Self::VenetianBlinds => {
				const P: &[EffectParam] = &[
					percent("Completion", &[0.0], 0.0, 1.0),
					angle("Direction", &[0.0]),
					pixels("Width", &[30.0], 2000.0),
					pixels("Feather", &[0.0], 500.0),
				];
				P
			}
		}
	}

	/// Total scalar slots of all parameters.
	pub fn slot_count(self) -> usize {
		self.params().iter().map(|param| param.kind.slots()).sum()
	}

	/// Slots of parameter `index`.
	pub fn slot_range(self, index: usize) -> Range<usize> {
		let params = self.params();
		let start: usize = params.iter().take(index).map(|param| param.kind.slots()).sum();
		let len = params.get(index).map_or(0, |param| param.kind.slots());
		start..start + len
	}

	/// Parameter owning `slot` and the slot's channel within it.
	pub fn param_of_slot(self, slot: usize) -> Option<(usize, &'static EffectParam, usize)> {
		let mut start = 0;
		for (index, param) in self.params().iter().enumerate() {
			let slots = param.kind.slots();
			if slot < start + slots {
				return Some((index, param, slot - start));
			}
			start += slots;
		}
		None
	}

	/// Default value of every slot.
	pub fn defaults(self) -> Vec<f32> {
		self.params()
			.iter()
			.flat_map(|param| (0..param.kind.slots()).map(|channel| param.default.get(channel).copied().unwrap_or(0.0)))
			.collect()
	}
}

impl std::fmt::Display for EffectKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.name())
	}
}

/// An effect applied to a layer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LayerEffect {
	/// Unique within its layer (stable across reordering).
	pub id: u32,
	pub kind: EffectKind,
	/// The "fx" switch.
	pub enabled: bool,
	/// Static value of each slot.
	values: Vec<f32>,
	/// Keyframes of each slot (empty when static).
	curves: Vec<AnimationCurve>,
}

impl LayerEffect {
	pub fn new(id: u32, kind: EffectKind) -> Self {
		let values = kind.defaults();
		let curves = vec![AnimationCurve::default(); values.len()];
		Self {
			id,
			kind,
			enabled: true,
			values,
			curves,
		}
	}

	pub fn name(&self) -> &'static str {
		self.kind.name()
	}

	fn base(&self, slot: usize) -> f32 {
		self.values
			.get(slot)
			.copied()
			.unwrap_or_else(|| self.kind.defaults().get(slot).copied().unwrap_or(0.0))
	}

	/// Keyframes of `slot` (empty when the slot does not exist).
	pub fn curve(&self, slot: usize) -> &AnimationCurve {
		static EMPTY: AnimationCurve = AnimationCurve::EMPTY;
		self.curves.get(slot).unwrap_or(&EMPTY)
	}

	/// Grow storage written by an older version of the effect.
	fn ensure_slots(&mut self) {
		let defaults = self.kind.defaults();
		if self.values.len() < defaults.len() {
			self.values.extend_from_slice(&defaults[self.values.len()..]);
		}
		self.curves
			.resize_with(defaults.len().max(self.curves.len()), AnimationCurve::default);
	}

	/// Evaluated value of `slot` at `time`.
	pub fn value_at(&self, slot: usize, time: f32) -> f32 {
		let base = self.base(slot);
		let value = self.curve(slot).evaluate(base, time);
		match self.kind.param_of_slot(slot) {
			Some((_, param, _)) => param.kind.clamp(value),
			None => value,
		}
	}

	/// Evaluated value of every slot at `time`.
	pub fn values_at(&self, time: f32) -> Vec<f32> {
		(0..self.kind.slot_count())
			.map(|slot| self.value_at(slot, time))
			.collect()
	}

	/// Whether parameter `index` has keyframes on any channel.
	pub fn is_animated(&self, index: usize) -> bool {
		self.kind.slot_range(index).any(|slot| !self.curve(slot).is_empty())
	}

	/// Whether parameter `index` has a key on `time`.
	pub fn has_key_at(&self, index: usize, time: f32) -> bool {
		self.kind
			.slot_range(index)
			.any(|slot| self.curve(slot).key_at(time).is_some())
	}

	/// Write a slot: animated slots get a key at `time`, static ones change.
	pub fn set_value_at(&mut self, slot: usize, time: f32, value: f32) {
		let Some((index, param, _)) = self.kind.param_of_slot(slot) else {
			return;
		};
		self.ensure_slots();
		let value = param.kind.clamp(value);
		if param.kind.is_animatable() && self.is_animated(index) {
			self.curves[slot].add_key(time, value);
		} else {
			self.values[slot] = value;
		}
	}

	/// Stopwatch for parameter `index`: removes its keys at `time`, or keys
	/// every channel at its current value. Returns `true` when keys were added.
	pub fn toggle_key_at(&mut self, index: usize, time: f32) -> bool {
		if !self
			.kind
			.params()
			.get(index)
			.is_some_and(|param| param.kind.is_animatable())
		{
			return false;
		}
		self.ensure_slots();
		let on_key = self.has_key_at(index, time);
		for slot in self.kind.slot_range(index) {
			let value = self.value_at(slot, time);
			let curve = &mut self.curves[slot];
			match curve.key_at(time).map(|key| key.id) {
				Some(id) if on_key => {
					curve.remove_key(id);
					// 最後のキーを消したら静的な値として残す
					if curve.is_empty() {
						self.values[slot] = value;
					}
				}
				None if !on_key => {
					curve.add_key(time, value);
				}
				_ => {}
			}
		}
		!on_key
	}

	/// Previous / next key time of parameter `index`.
	pub fn adjacent_key_time(&self, index: usize, time: f32, forward: bool) -> Option<f32> {
		let times = self.kind.slot_range(index).filter_map(|slot| {
			let curve = self.curve(slot);
			if forward {
				curve.next_key_time(time)
			} else {
				curve.previous_key_time(time)
			}
		});
		if forward {
			times.reduce(f32::min)
		} else {
			times.reduce(f32::max)
		}
	}

	/// Back to the defaults, dropping keyframes.
	pub fn reset(&mut self) {
		let enabled = self.enabled;
		*self = Self::new(self.id, self.kind);
		self.enabled = enabled;
	}

	/// Move every key by `delta` seconds.
	pub fn shift(&mut self, delta: f32) {
		for curve in &mut self.curves {
			curve.shift_times(delta);
		}
	}

	/// Times of every key (unsorted, may repeat).
	pub fn key_times(&self) -> impl Iterator<Item = f32> + '_ {
		self.curves
			.iter()
			.flat_map(|curve| curve.keys().iter().map(|key| key.time))
	}

	/// `(slot, key id)` of every key sitting on `time`.
	pub fn keys_at(&self, time: f32) -> Vec<(usize, u64)> {
		self.curves
			.iter()
			.enumerate()
			.filter_map(|(slot, curve)| Some((slot, curve.key_at(time)?.id)))
			.collect()
	}

	/// Move key `id` of `slot` to `time`, keeping its value.
	pub fn move_key(&mut self, slot: usize, id: u64, time: f32) {
		if let Some(curve) = self.curves.get_mut(slot)
			&& let Some(value) = curve.key(id).map(|key| key.value)
		{
			curve.set_key(id, time, value);
		}
	}

	/// Easy Ease every key on `time` (all keys when `None`). Returns the count.
	pub fn easy_ease(&mut self, time: Option<f32>) -> usize {
		let mut count = 0;
		for curve in &mut self.curves {
			let ids: Vec<u64> = match time {
				Some(time) => curve.key_at(time).map(|key| key.id).into_iter().collect(),
				None => curve.keys().iter().map(|key| key.id).collect(),
			};
			for id in ids {
				count += usize::from(curve.easy_ease(id));
			}
		}
		count
	}

	/// The effect frozen at `time`, ready to render.
	pub fn evaluate(&self, time: f32) -> EvaluatedEffect {
		EvaluatedEffect {
			kind: self.kind,
			values: self.values_at(time),
		}
	}
}

/// An effect with every parameter evaluated at one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedEffect {
	pub kind: EffectKind,
	/// One value per slot, in [`EffectKind::params`] order.
	pub values: Vec<f32>,
}

impl EvaluatedEffect {
	/// Value of `slot` (0 when missing).
	pub fn value(&self, slot: usize) -> f32 {
		self.values.get(slot).copied().unwrap_or(0.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_effect_has_consistent_defaults() {
		for kind in EffectKind::ALL {
			assert!(!kind.params().is_empty(), "{kind} has controls");
			for param in kind.params() {
				assert_eq!(param.default.len(), param.kind.slots(), "{kind}: {}", param.name);
			}
			assert_eq!(kind.defaults().len(), kind.slot_count());
		}
		let listed: usize = EffectCategory::ALL
			.iter()
			.map(|category| category.effects().count())
			.sum();
		assert_eq!(listed, EffectKind::ALL.len());
	}

	#[test]
	fn slots_map_back_to_their_params() {
		let kind = EffectKind::GradientRamp;
		assert_eq!(kind.slot_range(0), 0..2);
		assert_eq!(kind.slot_range(1), 2..5);
		let (index, param, channel) = kind.param_of_slot(4).unwrap();
		assert_eq!((index, param.name, channel), (1, "Start Color", 2));
		assert!(kind.param_of_slot(kind.slot_count()).is_none());
	}

	#[test]
	fn values_clamp_and_key_like_transform_channels() {
		let mut effect = LayerEffect::new(0, EffectKind::GaussianBlur);
		effect.set_value_at(0, 0.0, -5.0);
		assert_eq!(effect.value_at(0, 0.0), 0.0, "blurriness is never negative");

		assert!(effect.toggle_key_at(0, 0.0));
		effect.set_value_at(0, 2.0, 40.0);
		assert!(effect.is_animated(0));
		assert_eq!(effect.value_at(0, 2.0), 40.0);
		assert_eq!(effect.adjacent_key_time(0, 1.0, true), Some(2.0));

		effect.shift(1.0);
		assert_eq!(effect.value_at(0, 3.0), 40.0);
		assert!(!effect.toggle_key_at(1, 0.0), "choices do not animate");
	}

	#[test]
	fn color_params_key_all_channels() {
		let mut effect = LayerEffect::new(0, EffectKind::Fill);
		assert!(effect.toggle_key_at(0, 1.0));
		assert_eq!(effect.keys_at(1.0).len(), 3);
		assert!(!effect.toggle_key_at(0, 1.0));
		assert!(!effect.is_animated(0));
		assert_eq!(effect.value_at(0, 0.0), 1.0);
	}
}
