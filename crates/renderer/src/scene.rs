//! Render-ready description of one composition frame.

use core::{Affine, BlendMode, CompositionId, MediaInfo, SceneObjectId, TextObject};
use domain::{Composition, EvaluatedEffect, Instance, InstanceContent, MediaKind, Project};
use std::path::PathBuf;

/// Where nested compositions are looked up.
pub trait CompositionLibrary {
	fn composition(&self, id: CompositionId) -> Option<&Composition>;
}

impl CompositionLibrary for Project {
	fn composition(&self, id: CompositionId) -> Option<&Composition> {
		Project::composition(self, &id)
	}
}

impl CompositionLibrary for [Composition] {
	fn composition(&self, id: CompositionId) -> Option<&Composition> {
		self.iter().find(|composition| composition.id() == id)
	}
}

impl CompositionLibrary for Vec<Composition> {
	fn composition(&self, id: CompositionId) -> Option<&Composition> {
		self.as_slice().composition(id)
	}
}

/// No nested compositions (they draw nothing).
impl CompositionLibrary for () {
	fn composition(&self, _id: CompositionId) -> Option<&Composition> {
		None
	}
}

/// Nesting depth after which comp layers stop drawing (guards against loops).
const MAX_NESTING: usize = 16;

/// What a layer draws.
#[derive(Debug, Clone)]
pub enum LayerKind {
	/// Text block (carries its own transform).
	Text(TextObject),
	/// Solid rectangle of `size` composition pixels.
	Solid { size: [f32; 2], color: [f32; 4] },
	/// Still image file.
	Still { path: PathBuf, size: [f32; 2] },
	/// Video frame at `source_time` seconds into the file. `key` separates
	/// layers that play the same file so each keeps its own decoder.
	Video {
		path: PathBuf,
		key: u64,
		info: MediaInfo,
		source_time: f32,
	},
	/// Another composition rendered offscreen (transparent background) and
	/// drawn like footage over `size` layer pixels.
	Nested { scene: Box<Scene>, size: [f32; 2] },
}

/// One drawable layer, already evaluated at the frame time.
#[derive(Debug, Clone)]
pub struct SceneLayer {
	/// Layer space (origin at the layer center, layer pixels) → output pixels.
	pub matrix: Affine,
	pub opacity: f32,
	pub blend: BlendMode,
	pub kind: LayerKind,
	/// Enabled effects evaluated at the frame time, applied in order.
	pub effects: Vec<EvaluatedEffect>,
	/// Output pixels per composition pixel, for sizing effect parameters.
	pub effect_scale: f32,
	/// Composition time of the frame (animated effects such as noise).
	pub time: f32,
}

impl SceneLayer {
	/// A plain layer without effects.
	pub fn new(matrix: Affine, opacity: f32, blend: BlendMode, kind: LayerKind) -> Self {
		Self {
			matrix,
			opacity,
			blend,
			kind,
			effects: Vec::new(),
			effect_scale: 1.0,
			time: 0.0,
		}
	}
}

/// Everything needed to draw one frame, back to front.
#[derive(Debug, Clone)]
pub struct Scene {
	pub width: u32,
	pub height: u32,
	pub background: [f32; 4],
	pub layers: Vec<SceneLayer>,
}

impl Scene {
	/// Evaluate `composition` at `time`.
	///
	/// `factor` scales the output (preview resolution); `is_track_rendered`
	/// filters hidden / non-solo tracks of the top-level composition;
	/// `library` resolves nested compositions.
	pub fn from_composition(
		composition: &Composition,
		time: f32,
		factor: f32,
		is_track_rendered: impl Fn(usize) -> bool,
		library: &(impl CompositionLibrary + ?Sized),
	) -> Self {
		let factor = if factor.is_finite() && factor > 0.0 {
			factor
		} else {
			1.0
		};
		let mut scene = build(composition, time, factor, &is_track_rendered, library, &mut Vec::new());
		scene.background = composition.background();
		scene
	}
}

fn build(
	composition: &Composition,
	time: f32,
	factor: f32,
	is_track_rendered: &dyn Fn(usize) -> bool,
	library: &(impl CompositionLibrary + ?Sized),
	stack: &mut Vec<CompositionId>,
) -> Scene {
	let scaled = |value: u32| ((value as f32 * factor).round() as u32).max(1);
	let (width, height) = (scaled(composition.width()), scaled(composition.height()));
	// コンポ空間（中心原点）→ 出力画素。解像度を落とすときは全体を縮める
	let output = Affine::translate(width as f32 / 2.0, height as f32 / 2.0).then_apply(Affine::scale(factor, factor));
	stack.push(composition.id());
	let layers = composition
		.objects_in_draw_order()
		.filter(|instance| instance.is_active_at(time) && is_track_rendered(instance.track_index()))
		.filter_map(|instance| {
			let matrix = output.then_apply(composition.world_matrix(instance.id(), time));
			let kind = match instance.content().as_composition() {
				Some(id) => nested_layer(id, instance, time, matrix, library, stack)?,
				None => layer_kind(composition.id(), instance, time)?,
			};
			scene_layer(instance, time, factor, matrix, kind)
		})
		.collect();
	stack.pop();
	Scene {
		width,
		height,
		// ネストしたコンポは透明背景で合成する（呼び出し元が上書きする）
		background: [0.0; 4],
		layers,
	}
}

/// A comp layer: the nested composition at the layer's source time, rendered
/// at the resolution the layer is shown at (never above 100%).
fn nested_layer(
	id: CompositionId,
	instance: &Instance,
	time: f32,
	matrix: Affine,
	library: &(impl CompositionLibrary + ?Sized),
	stack: &mut Vec<CompositionId>,
) -> Option<LayerKind> {
	if stack.contains(&id) || stack.len() >= MAX_NESTING {
		return None;
	}
	let nested = library.composition(id)?;
	let factor = matrix.max_scale().clamp(1.0 / 32.0, 1.0);
	let scene = build(nested, instance.source_time_at(time), factor, &|_| true, library, stack);
	Some(LayerKind::Nested {
		scene: Box::new(scene),
		size: [nested.width() as f32, nested.height() as f32],
	})
}

fn scene_layer(instance: &Instance, time: f32, factor: f32, matrix: Affine, kind: LayerKind) -> Option<SceneLayer> {
	let opacity = instance.property_at(domain::TransformProperty::Opacity, time);
	if opacity <= 0.0 {
		return None;
	}
	Some(SceneLayer {
		effects: instance.effects_at(time),
		effect_scale: factor,
		time,
		..SceneLayer::new(matrix, opacity, instance.blend_mode, kind)
	})
}

fn layer_kind(composition: CompositionId, instance: &Instance, time: f32) -> Option<LayerKind> {
	let transform = instance.transform_at(time);

	let kind = match instance.content() {
		InstanceContent::Empty => return None,
		InstanceContent::Text {
			text,
			font_path,
			font_size,
			spacing,
			fill_color,
		} => LayerKind::Text(TextObject {
			id: SceneObjectId(instance.id().value() as u64),
			name: instance.name().to_string(),
			transform,
			start_time: instance.start_time(),
			duration: instance.duration(),
			text: text.clone(),
			font_path: font_path.clone(),
			font_size: *font_size,
			spacing: *spacing,
			fill_color: *fill_color,
		}),
		InstanceContent::Solid { width, height, color } => LayerKind::Solid {
			size: [*width, *height],
			color: *color,
		},
		InstanceContent::Media(source) => match source.kind {
			MediaKind::Audio => return None,
			MediaKind::Image => LayerKind::Still {
				path: source.path.clone(),
				size: source.size(),
			},
			MediaKind::Video if !source.info.is_moving() => LayerKind::Still {
				path: source.path.clone(),
				size: source.size(),
			},
			MediaKind::Video => LayerKind::Video {
				path: source.path.clone(),
				// 同じファイルを複数のコンポで使ってもデコーダを分ける
				key: ((composition.value() as u64) << 32) | instance.id().value() as u64,
				info: source.info,
				source_time: instance.source_time_at(time),
			},
		},
		InstanceContent::Composition(_) => return None,
	};
	Some(kind)
}
