//! Draws [`Scene`]s: solids, footage and text over the composition background.

use core::{Affine, BlendMode};
use image::{ImageBuffer, Rgba, RgbaImage};
use media::{MediaCache, Picture};
use rayon::prelude::*;

use crate::scene::{LayerKind, Scene, SceneLayer};

/// Stateful frame renderer. Keeps decoded footage between frames, so use one
/// per thread and feed it consecutive frames where possible.
#[derive(Default)]
pub struct Compositor {
	media: MediaCache,
}

impl Compositor {
	pub fn new() -> Self {
		Self::default()
	}

	/// Render `scene` to an opaque RGBA image (flattened over black).
	pub fn render(&mut self, scene: &Scene) -> RgbaImage {
		let mut image = self.render_premultiplied(scene);
		// 乗算済みなので、黒の上に置くには α を 255 にするだけでよい
		let pixels: &mut [u8] = &mut image;
		pixels.par_chunks_mut(4).for_each(|pixel| pixel[3] = 255);
		self.media.finish_render();
		image
	}

	/// Render `scene` keeping its alpha (premultiplied RGBA).
	fn render_premultiplied(&mut self, scene: &Scene) -> RgbaImage {
		let background = scene.background;
		let to_byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
		let alpha = background[3].clamp(0.0, 1.0);
		let fill = Rgba([
			to_byte(background[0] * alpha),
			to_byte(background[1] * alpha),
			to_byte(background[2] * alpha),
			to_byte(alpha),
		]);
		let mut image = crate::filled_buffer(scene.width, scene.height, fill);
		for layer in &scene.layers {
			self.draw_layer(&mut image, layer);
		}
		image
	}

	fn draw_layer(&mut self, image: &mut RgbaImage, layer: &SceneLayer) {
		let paint = Paint {
			matrix: layer.matrix,
			opacity: layer.opacity,
			blend: layer.blend,
		};
		match &layer.kind {
			LayerKind::Text(text) => crate::draw_text(image, text, &paint),
			LayerKind::Solid { size, color } => draw_solid(image, &paint, *size, *color),
			LayerKind::Still { path, size } => {
				if let Some(picture) = self.media.still(path, layer.matrix.max_scale()) {
					draw_picture(image, &picture, *size, &paint);
				}
			}
			LayerKind::Video {
				path,
				key,
				info,
				source_time,
			} => {
				let scale = layer.matrix.max_scale();
				if let Some(picture) = self.media.video_frame(path, *key, info, *source_time, scale) {
					let size = [info.width as f32, info.height as f32];
					draw_picture(image, &picture, size, &paint);
				}
			}
			LayerKind::Nested { scene } => {
				let picture = self.render_premultiplied(scene);
				let size = [scene.width as f32, scene.height as f32];
				draw_picture(image, &picture, size, &paint);
			}
		}
	}
}

/// How a layer lands on the canvas.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Paint {
	/// Layer space (origin at the layer center) → canvas pixels.
	pub matrix: Affine,
	pub opacity: f32,
	pub blend: BlendMode,
}

/// Blend a premultiplied source color (0..1) onto a premultiplied canvas pixel.
#[inline]
pub(crate) fn composite(pixel: &mut [u8], source: [f32; 4], blend: BlendMode) {
	let sa = source[3].clamp(0.0, 1.0);
	if sa <= 0.0 {
		return;
	}
	let to_byte = |value: f32| (value * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
	let ba = f32::from(pixel[3]) / 255.0;
	let keep = 1.0 - sa;
	for channel in 0..3 {
		let cs = source[channel];
		let cb = f32::from(pixel[channel]) / 255.0;
		let out = match blend {
			BlendMode::Normal => cs + cb * keep,
			mode => {
				// W3C compositing: 乗算済みのまま非 Normal を混ぜる
				let straight_b = if ba > 0.0 { (cb / ba).min(1.0) } else { 0.0 };
				let straight_s = (cs / sa).min(1.0);
				cs * (1.0 - ba) + cb * keep + sa * ba * mode.channel(straight_b, straight_s)
			}
		};
		pixel[channel] = to_byte(out);
	}
	pixel[3] = to_byte(sa + ba * keep);
}

/// Canvas → layer mapping of a `size`d layer centered on its origin.
#[derive(Clone, Copy)]
struct LayerMapping {
	forward: Affine,
	inverse: Affine,
	half: [f32; 2],
}

impl LayerMapping {
	fn new(matrix: Affine, size: [f32; 2]) -> Option<Self> {
		if size[0] <= 0.0 || size[1] <= 0.0 {
			return None;
		}
		Some(Self {
			forward: matrix,
			inverse: matrix.inverse()?,
			half: [size[0] / 2.0, size[1] / 2.0],
		})
	}

	/// Layer-local point (origin at the top-left corner) under a canvas point.
	#[inline]
	fn to_local(self, x: f32, y: f32) -> (f32, f32) {
		let [lx, ly] = self.inverse.apply([x, y]);
		(lx + self.half[0], ly + self.half[1])
	}

	/// Canvas rows / columns the layer can touch, clipped to the canvas.
	fn screen_bounds(&self, canvas: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
		let [hx, hy] = self.half;
		let corners = [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]].map(|corner| self.forward.apply(corner));
		let min_x = corners.iter().map(|c| c[0]).fold(f32::INFINITY, f32::min) - 1.0;
		let max_x = corners.iter().map(|c| c[0]).fold(f32::NEG_INFINITY, f32::max) + 1.0;
		let min_y = corners.iter().map(|c| c[1]).fold(f32::INFINITY, f32::min) - 1.0;
		let max_y = corners.iter().map(|c| c[1]).fold(f32::NEG_INFINITY, f32::max) + 1.0;
		if !(min_x.is_finite() && max_x.is_finite() && min_y.is_finite() && max_y.is_finite()) {
			return None;
		}
		let x0 = min_x.floor().max(0.0) as u32;
		let y0 = min_y.floor().max(0.0) as u32;
		let x1 = (max_x.ceil().max(0.0) as u32).min(canvas.0);
		let y1 = (max_y.ceil().max(0.0) as u32).min(canvas.1);
		(x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
	}
}

/// Rasterize a layer: `shade(local_x, local_y)` returns the premultiplied
/// color (0..1) of the layer at a point, which is blended onto the canvas.
fn raster(image: &mut RgbaImage, mapping: &LayerMapping, paint: &Paint, shade: impl Fn(f32, f32) -> [f32; 4] + Sync) {
	let canvas = image.dimensions();
	let Some((x0, y0, x1, y1)) = mapping.screen_bounds(canvas) else {
		return;
	};
	let opacity = paint.opacity.clamp(0.0, 1.0);
	if opacity <= 0.0 {
		return;
	}
	let blend = paint.blend;
	let stride = canvas.0 as usize * 4;
	let pixels: &mut [u8] = image;
	pixels
		.par_chunks_mut(stride)
		.enumerate()
		.skip(y0 as usize)
		.take((y1 - y0) as usize)
		.for_each(|(y, row)| {
			let py = y as f32 + 0.5;
			for x in x0..x1 {
				let (lx, ly) = mapping.to_local(x as f32 + 0.5, py);
				let color = shade(lx, ly);
				if color[3] <= 0.0 {
					continue;
				}
				let offset = x as usize * 4;
				composite(&mut row[offset..offset + 4], color.map(|c| c * opacity), blend);
			}
		});
}

/// Anti-aliased (possibly rotated / skewed) solid rectangle.
pub(crate) fn draw_solid(image: &mut RgbaImage, paint: &Paint, size: [f32; 2], color: [f32; 4]) {
	let Some(mapping) = LayerMapping::new(paint.matrix, size) else {
		return;
	};
	let alpha = color[3].clamp(0.0, 1.0);
	let premultiplied = [color[0] * alpha, color[1] * alpha, color[2] * alpha, alpha];
	// 端の被覆率をスクリーン画素単位で求めるため、ローカル距離にスケールを掛ける
	let scale = paint.matrix.axis_scales();
	raster(image, &mapping, paint, |lx, ly| {
		let edge_x = (lx.min(size[0] - lx) * scale[0] + 0.5).clamp(0.0, 1.0);
		let edge_y = (ly.min(size[1] - ly) * scale[1] + 0.5).clamp(0.0, 1.0);
		let coverage = edge_x * edge_y;
		premultiplied.map(|channel| channel * coverage)
	});
}

/// Draw a premultiplied picture stretched over `size` layer pixels with
/// bilinear filtering (transparent outside, which also softens the edges).
pub(crate) fn draw_picture(image: &mut RgbaImage, picture: &Picture, size: [f32; 2], paint: &Paint) {
	let Some(mapping) = LayerMapping::new(paint.matrix, size) else {
		return;
	};
	let (width, height) = picture.dimensions();
	if width == 0 || height == 0 {
		return;
	}
	let to_texel = [width as f32 / size[0], height as f32 / size[1]];
	let data: &[u8] = picture.as_raw();
	let fetch = |x: i64, y: i64| -> [f32; 4] {
		if x < 0 || y < 0 || x >= i64::from(width) || y >= i64::from(height) {
			return [0.0; 4];
		}
		let offset = (y as usize * width as usize + x as usize) * 4;
		[
			f32::from(data[offset]),
			f32::from(data[offset + 1]),
			f32::from(data[offset + 2]),
			f32::from(data[offset + 3]),
		]
	};
	raster(image, &mapping, paint, |lx, ly| {
		let u = lx * to_texel[0] - 0.5;
		let v = ly * to_texel[1] - 0.5;
		if u < -1.0 || v < -1.0 || u > width as f32 || v > height as f32 {
			return [0.0; 4];
		}
		let (x, y) = (u.floor(), v.floor());
		let (fx, fy) = (u - x, v - y);
		let (x, y) = (x as i64, y as i64);
		let [a, b, c, d] = [fetch(x, y), fetch(x + 1, y), fetch(x, y + 1), fetch(x + 1, y + 1)];
		let mut out = [0.0; 4];
		for channel in 0..4 {
			let top = a[channel] + (b[channel] - a[channel]) * fx;
			let bottom = c[channel] + (d[channel] - c[channel]) * fx;
			out[channel] = (top + (bottom - top) * fy) / 255.0;
		}
		out
	});
}

/// Convenience for tests and tools: render one scene without keeping caches.
pub fn render_scene(scene: &Scene) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
	Compositor::new().render(scene)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::scene::SceneLayer;
	use core::{NodeId, Transform};
	use domain::{Composition, InstanceContent};

	fn scene(layers: Vec<SceneLayer>) -> Scene {
		Scene {
			width: 100,
			height: 100,
			background: [0.0, 0.0, 0.0, 1.0],
			layers,
		}
	}

	fn centered(transform: Transform) -> Affine {
		Affine::translate(50.0, 50.0).then_apply(transform.local_matrix())
	}

	fn paint(transform: Transform) -> Paint {
		Paint {
			matrix: centered(transform),
			opacity: transform.opacity,
			blend: BlendMode::Normal,
		}
	}

	fn solid(transform: Transform, color: [f32; 4]) -> SceneLayer {
		SceneLayer {
			matrix: centered(transform),
			opacity: transform.opacity,
			blend: BlendMode::Normal,
			kind: LayerKind::Solid {
				size: [40.0, 20.0],
				color,
			},
		}
	}

	#[test]
	fn background_fills_the_frame() {
		let mut empty = scene(Vec::new());
		empty.background = [1.0, 0.5, 0.0, 1.0];
		let image = render_scene(&empty);
		assert_eq!(image.get_pixel(0, 0).0, [255, 128, 0, 255]);
		assert_eq!(image.get_pixel(99, 99).0, [255, 128, 0, 255]);
	}

	#[test]
	fn solid_covers_its_rectangle_only() {
		let image = render_scene(&scene(vec![solid(Transform::default(), [1.0, 0.0, 0.0, 1.0])]));
		// 中心 (50,50) を中心に 40x20
		assert_eq!(image.get_pixel(50, 50).0, [255, 0, 0, 255]);
		assert_eq!(image.get_pixel(31, 41).0, [255, 0, 0, 255]);
		assert_eq!(image.get_pixel(29, 50).0, [0, 0, 0, 255]);
		assert_eq!(image.get_pixel(50, 39).0, [0, 0, 0, 255]);
	}

	#[test]
	fn opacity_and_rotation_apply_to_solids() {
		let transform = Transform {
			rotation: [0.0, 0.0, 90.0],
			opacity: 0.5,
			..Transform::default()
		};
		let image = render_scene(&scene(vec![solid(transform, [1.0, 1.0, 1.0, 1.0])]));
		let center = image.get_pixel(50, 50)[0];
		assert!((126..=129).contains(&center), "half opacity, got {center}");
		// 90° 回転で縦長になる
		assert!(image.get_pixel(50, 35)[0] > 100);
		assert_eq!(image.get_pixel(35, 50)[0], 0);
	}

	#[test]
	fn anchor_point_offsets_the_layer() {
		let transform = Transform {
			anchor: [-20.0, -10.0, 0.0],
			..Transform::default()
		};
		// 左上の角がコンポ中心に来る
		let image = render_scene(&scene(vec![solid(transform, [1.0, 1.0, 1.0, 1.0])]));
		assert_eq!(image.get_pixel(51, 51)[0], 255);
		assert_eq!(image.get_pixel(88, 58)[0], 255);
		assert_eq!(image.get_pixel(48, 51)[0], 0);
		assert_eq!(image.get_pixel(51, 48)[0], 0);
	}

	#[test]
	fn blend_modes_mix_with_the_backdrop() {
		let mut grey = scene(Vec::new());
		grey.background = [0.5, 0.5, 0.5, 1.0];
		let mut layer = solid(Transform::default(), [0.5, 1.0, 0.0, 1.0]);
		layer.blend = BlendMode::Multiply;
		grey.layers.push(layer);
		let pixel = render_scene(&grey).get_pixel(50, 50).0;
		assert_eq!(pixel, [64, 128, 0, 255]);

		grey.layers[0].blend = BlendMode::Screen;
		let pixel = render_scene(&grey).get_pixel(50, 50).0;
		assert_eq!(pixel, [192, 255, 128, 255]);
	}

	#[test]
	fn composite_keeps_alpha_on_transparent_canvas() {
		let mut pixel = [0u8; 4];
		composite(&mut pixel, [0.5, 0.0, 0.0, 0.5], BlendMode::Screen);
		assert_eq!(pixel, [128, 0, 0, 128]);
		composite(&mut pixel, [0.0, 0.0, 0.5, 0.5], BlendMode::Normal);
		assert_eq!(pixel, [64, 0, 128, 192]);
	}

	#[test]
	fn pictures_draw_pixel_exact_at_identity() {
		let mut picture = Picture::new(4, 2);
		for (x, y, pixel) in picture.enumerate_pixels_mut() {
			*pixel = Rgba([(x * 60) as u8, (y * 200) as u8, 7, 255]);
		}
		let mut canvas = RgbaImage::from_pixel(4, 2, Rgba([0, 0, 0, 255]));
		let paint = Paint {
			matrix: Affine::translate(2.0, 1.0),
			opacity: 1.0,
			blend: BlendMode::Normal,
		};
		draw_picture(&mut canvas, &picture, [4.0, 2.0], &paint);
		assert_eq!(canvas, picture);
	}

	#[test]
	fn scaled_pictures_cover_the_scaled_area() {
		let picture = Picture::from_pixel(10, 10, Rgba([0, 255, 0, 255]));
		let transform = Transform {
			scale: [2.0, 2.0, 1.0],
			..Transform::default()
		};
		let mut canvas = RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 255]));
		draw_picture(&mut canvas, &picture, [10.0, 10.0], &paint(transform));
		assert_eq!(canvas.get_pixel(41, 41).0, [0, 255, 0, 255]);
		assert_eq!(canvas.get_pixel(58, 58).0, [0, 255, 0, 255]);
		assert_eq!(canvas.get_pixel(38, 50).0, [0, 0, 0, 255]);
	}

	#[test]
	fn scene_follows_draw_order_visibility_and_resolution() {
		let mut composition = Composition::new(core::CompositionId::new(0), "Test", 200, 100, 30.0);
		let top = composition.add_instance_named(NodeId::new(0), "Top");
		top.set_content(InstanceContent::solid(10.0, 10.0, [1.0, 0.0, 0.0, 1.0]));
		top.transform.position = [20.0, 0.0, 0.0];
		let bottom = composition.add_instance_named(NodeId::new(1), "Bottom");
		bottom.set_content(InstanceContent::solid(10.0, 10.0, [0.0, 0.0, 1.0, 1.0]));
		bottom.set_start_time(6.0);
		composition.add_instance_named(NodeId::new(2), "Null");

		let at_one = Scene::from_composition(&composition, 1.0, 0.5, |_| true);
		assert_eq!((at_one.width, at_one.height), (100, 50));
		assert_eq!(at_one.layers.len(), 1, "inactive and empty layers are skipped");
		// 半分の解像度: 中心 (50,25) + 20*0.5
		assert_eq!(at_one.layers[0].matrix.apply([0.0, 0.0]), [60.0, 25.0]);
		assert_eq!(at_one.layers[0].matrix.axis_scales(), [0.5, 0.5]);

		let at_seven = Scene::from_composition(&composition, 7.0, 1.0, |_| true);
		assert_eq!(at_seven.layers.len(), 1);
		let hidden = Scene::from_composition(&composition, 1.0, 1.0, |track| track != 0);
		assert!(hidden.layers.is_empty());
	}

	#[test]
	fn children_follow_their_parent() {
		let mut composition = Composition::new(core::CompositionId::new(0), "Test", 100, 100, 30.0);
		let parent = composition.add_instance_named(NodeId::new(0), "Null").id();
		let child = composition.add_instance_named(NodeId::new(1), "Child");
		child.set_content(InstanceContent::solid(10.0, 10.0, [1.0, 1.0, 1.0, 1.0]));
		child.transform.position = [10.0, 0.0, 0.0];
		let child = child.id();
		composition.set_parent(child, Some(parent), 0.0);
		composition.get_mut(parent).unwrap().transform.rotation[2] = 90.0;

		let scene = Scene::from_composition(&composition, 0.0, 1.0, |_| true);
		let [x, y] = scene.layers[0].matrix.apply([0.0, 0.0]);
		assert!((x - 50.0).abs() < 1e-3 && (y - 60.0).abs() < 1e-3, "({x}, {y})");
	}
}
