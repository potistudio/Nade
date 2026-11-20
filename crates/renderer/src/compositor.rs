//! Draws [`Scene`]s: solids, footage and text over the composition background.

use core::Transform;
use image::{ImageBuffer, Rgba, RgbaImage};
use media::{MediaCache, Picture};
use rayon::prelude::*;

use crate::scene::{LayerKind, Scene};

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

	/// Render `scene` to an opaque RGBA image.
	pub fn render(&mut self, scene: &Scene) -> RgbaImage {
		let background = scene.background;
		let to_byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
		// 背景は黒の上に合成した不透明色として塗る
		let alpha = background[3].clamp(0.0, 1.0);
		let fill = Rgba([
			to_byte(background[0] * alpha),
			to_byte(background[1] * alpha),
			to_byte(background[2] * alpha),
			255,
		]);
		let mut image = crate::filled_buffer(scene.width, scene.height, fill);

		for layer in &scene.layers {
			let transform = layer.transform;
			match &layer.kind {
				LayerKind::Text(text) => crate::draw_text(&mut image, text, scene.width, scene.height),
				LayerKind::Solid { size, color } => draw_solid(&mut image, &transform, *size, *color),
				LayerKind::Still { path, size } => {
					if let Some(picture) = self.media.still(path, screen_scale(&transform)) {
						draw_picture(&mut image, &picture, *size, &transform);
					}
				}
				LayerKind::Video {
					path,
					key,
					info,
					source_time,
				} => {
					let scale = screen_scale(&transform);
					if let Some(picture) = self.media.video_frame(path, *key, info, *source_time, scale) {
						let size = [info.width as f32, info.height as f32];
						draw_picture(&mut image, &picture, size, &transform);
					}
				}
			}
		}

		self.media.finish_render();
		image
	}
}

/// Largest on-screen magnification of the layer (for mip selection).
fn screen_scale(transform: &Transform) -> f32 {
	transform.scale[0].abs().max(transform.scale[1].abs())
}

/// Screen → layer mapping of a 2D layer centered at its position.
#[derive(Clone, Copy)]
struct LayerMapping {
	center: [f32; 2],
	cos: f32,
	sin: f32,
	inv_scale: [f32; 2],
	half: [f32; 2],
}

impl LayerMapping {
	fn new(transform: &Transform, size: [f32; 2], canvas: (u32, u32)) -> Option<Self> {
		let [sx, sy] = [transform.scale[0], transform.scale[1]];
		if sx.abs() < 1e-6 || sy.abs() < 1e-6 || size[0] <= 0.0 || size[1] <= 0.0 {
			return None;
		}
		let (sin, cos) = transform.rotation[2].to_radians().sin_cos();
		Some(Self {
			center: [
				canvas.0 as f32 / 2.0 + transform.position[0],
				canvas.1 as f32 / 2.0 + transform.position[1],
			],
			cos,
			sin,
			inv_scale: [1.0 / sx, 1.0 / sy],
			half: [size[0] / 2.0, size[1] / 2.0],
		})
	}

	/// Layer-local point (origin at the top-left corner) under a screen point.
	#[inline]
	fn to_local(self, x: f32, y: f32) -> (f32, f32) {
		let dx = x - self.center[0];
		let dy = y - self.center[1];
		let lx = (dx * self.cos + dy * self.sin) * self.inv_scale[0];
		let ly = (-dx * self.sin + dy * self.cos) * self.inv_scale[1];
		(lx + self.half[0], ly + self.half[1])
	}

	/// Screen rows / columns the layer can touch, clipped to the canvas.
	fn screen_bounds(&self, canvas: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
		let sx = 1.0 / self.inv_scale[0];
		let sy = 1.0 / self.inv_scale[1];
		let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(cx, cy): (f32, f32)| {
			let x = cx * self.half[0] * sx;
			let y = cy * self.half[1] * sy;
			(
				x * self.cos - y * self.sin + self.center[0],
				x * self.sin + y * self.cos + self.center[1],
			)
		});
		let min_x = corners.iter().map(|c| c.0).fold(f32::INFINITY, f32::min) - 1.0;
		let max_x = corners.iter().map(|c| c.0).fold(f32::NEG_INFINITY, f32::max) + 1.0;
		let min_y = corners.iter().map(|c| c.1).fold(f32::INFINITY, f32::min) - 1.0;
		let max_y = corners.iter().map(|c| c.1).fold(f32::NEG_INFINITY, f32::max) + 1.0;
		let x0 = min_x.floor().max(0.0) as u32;
		let y0 = min_y.floor().max(0.0) as u32;
		let x1 = (max_x.ceil().max(0.0) as u32).min(canvas.0);
		let y1 = (max_y.ceil().max(0.0) as u32).min(canvas.1);
		(x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
	}
}

/// Rasterize a layer: `shade(local_x, local_y)` returns the premultiplied
/// color (0..1) of the layer at a point, which is blended over the canvas.
fn raster(image: &mut RgbaImage, mapping: &LayerMapping, opacity: f32, shade: impl Fn(f32, f32) -> [f32; 4] + Sync) {
	let canvas = image.dimensions();
	let Some((x0, y0, x1, y1)) = mapping.screen_bounds(canvas) else {
		return;
	};
	let opacity = opacity.clamp(0.0, 1.0);
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
				let [r, g, b, a] = shade(lx, ly);
				let a = a * opacity;
				if a <= 0.0 {
					continue;
				}
				let offset = x as usize * 4;
				let inverse = 1.0 - a;
				let blend = |src: f32, dst: u8| (src * opacity * 255.0 + f32::from(dst) * inverse).clamp(0.0, 255.0);
				row[offset] = blend(r, row[offset]) as u8;
				row[offset + 1] = blend(g, row[offset + 1]) as u8;
				row[offset + 2] = blend(b, row[offset + 2]) as u8;
				row[offset + 3] = 255;
			}
		});
}

/// Anti-aliased (possibly rotated) solid rectangle.
pub(crate) fn draw_solid(image: &mut RgbaImage, transform: &Transform, size: [f32; 2], color: [f32; 4]) {
	let canvas = image.dimensions();
	let Some(mapping) = LayerMapping::new(transform, size, canvas) else {
		return;
	};
	let alpha = color[3].clamp(0.0, 1.0);
	let premultiplied = [color[0] * alpha, color[1] * alpha, color[2] * alpha, alpha];
	// 端の被覆率をスクリーン画素単位で求めるため、ローカル距離にスケールを掛ける
	let scale = [transform.scale[0].abs(), transform.scale[1].abs()];
	raster(image, &mapping, transform.opacity, |lx, ly| {
		let edge_x = (lx.min(size[0] - lx) * scale[0] + 0.5).clamp(0.0, 1.0);
		let edge_y = (ly.min(size[1] - ly) * scale[1] + 0.5).clamp(0.0, 1.0);
		let coverage = edge_x * edge_y;
		premultiplied.map(|channel| channel * coverage)
	});
}

/// Draw a premultiplied picture stretched over `size` layer pixels with
/// bilinear filtering (transparent outside, which also softens the edges).
pub(crate) fn draw_picture(image: &mut RgbaImage, picture: &Picture, size: [f32; 2], transform: &Transform) {
	let canvas = image.dimensions();
	let Some(mapping) = LayerMapping::new(transform, size, canvas) else {
		return;
	};
	let (width, height) = picture.dimensions();
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
	raster(image, &mapping, transform.opacity, |lx, ly| {
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

	fn solid(transform: Transform, color: [f32; 4]) -> SceneLayer {
		SceneLayer {
			transform,
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
	fn pictures_draw_pixel_exact_at_identity() {
		let mut picture = Picture::new(4, 2);
		for (x, y, pixel) in picture.enumerate_pixels_mut() {
			*pixel = Rgba([(x * 60) as u8, (y * 200) as u8, 7, 255]);
		}
		let mut canvas = RgbaImage::from_pixel(4, 2, Rgba([0, 0, 0, 255]));
		draw_picture(&mut canvas, &picture, [4.0, 2.0], &Transform::default());
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
		draw_picture(&mut canvas, &picture, [10.0, 10.0], &transform);
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
		assert_eq!(at_one.layers[0].transform.position[0], 10.0);
		assert_eq!(at_one.layers[0].transform.scale[0], 0.5);

		let at_seven = Scene::from_composition(&composition, 7.0, 1.0, |_| true);
		assert_eq!(at_seven.layers.len(), 1);
		let hidden = Scene::from_composition(&composition, 1.0, 1.0, |track| track != 0);
		assert!(hidden.layers.is_empty());
	}
}
