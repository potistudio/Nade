//! Render-ready description of one composition frame.

use core::{Affine, BlendMode, MediaInfo, SceneObjectId, TextObject};
use domain::{Composition, Instance, InstanceContent, MediaKind};
use std::path::PathBuf;

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
	/// Another composition rendered offscreen and drawn like footage.
	Nested { scene: Box<Scene> },
}

/// One drawable layer, already evaluated at the frame time.
#[derive(Debug, Clone)]
pub struct SceneLayer {
	/// Layer space (origin at the layer center, layer pixels) → output pixels.
	pub matrix: Affine,
	pub opacity: f32,
	pub blend: BlendMode,
	pub kind: LayerKind,
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
	/// filters hidden / non-solo tracks.
	pub fn from_composition(
		composition: &Composition,
		time: f32,
		factor: f32,
		is_track_rendered: impl Fn(usize) -> bool,
	) -> Self {
		let factor = if factor.is_finite() && factor > 0.0 {
			factor
		} else {
			1.0
		};
		let scaled = |value: u32| ((value as f32 * factor).round() as u32).max(1);
		let (width, height) = (scaled(composition.width()), scaled(composition.height()));
		// コンポ空間（中心原点）→ 出力画素。解像度を落とすときは全体を縮める
		let output =
			Affine::translate(width as f32 / 2.0, height as f32 / 2.0).then_apply(Affine::scale(factor, factor));
		let layers = composition
			.objects_in_draw_order()
			.filter(|instance| instance.is_active_at(time) && is_track_rendered(instance.track_index()))
			.filter_map(|instance| {
				let matrix = output.then_apply(composition.world_matrix(instance.id(), time));
				scene_layer(instance, time, matrix)
			})
			.collect();
		Self {
			width,
			height,
			background: composition.background(),
			layers,
		}
	}
}

fn scene_layer(instance: &Instance, time: f32, matrix: Affine) -> Option<SceneLayer> {
	let transform = instance.transform_at(time);
	if transform.opacity <= 0.0 {
		return None;
	}

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
				key: instance.id().value() as u64,
				info: source.info,
				source_time: instance.source_time_at(time),
			},
		},
	};
	Some(SceneLayer {
		matrix,
		opacity: transform.opacity,
		blend: instance.blend_mode,
		kind,
	})
}
