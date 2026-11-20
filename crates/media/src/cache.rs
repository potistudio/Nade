//! Per-thread cache of decoded stills and running video decoders.

use nade_core::MediaInfo;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::Picture;
use crate::video::VideoDecoder;

/// Coarsest mip level kept for stills and decoded video.
const MAX_LEVEL: u32 = 4;
/// Renders a video decoder may stay unused before its process is stopped.
const VIDEO_IDLE_RENDERS: u64 = 120;

/// Mip level (number of halvings) good enough for drawing at `scale`.
pub fn mip_level(scale: f32) -> u32 {
	if !scale.is_finite() || scale <= 0.0 {
		return MAX_LEVEL;
	}
	if scale >= 0.5 {
		return 0;
	}
	((1.0 / scale).log2().floor() as u32).min(MAX_LEVEL)
}

fn level_size(width: u32, height: u32, level: u32) -> (u32, u32) {
	((width >> level).max(1), (height >> level).max(1))
}

struct VideoSlot {
	decoder: VideoDecoder,
	level: u32,
	last_used: u64,
}

/// Decoded footage shared by consecutive renders on one thread.
#[derive(Default)]
pub struct MediaCache {
	/// Mip chains of premultiplied stills. `None` when the file failed to load.
	stills: HashMap<PathBuf, Option<Vec<Arc<Picture>>>>,
	videos: HashMap<(PathBuf, u64), VideoSlot>,
	renders: u64,
}

impl MediaCache {
	pub fn new() -> Self {
		Self::default()
	}

	/// Premultiplied still picture suitable for drawing at `scale`.
	pub fn still(&mut self, path: &Path, scale: f32) -> Option<Arc<Picture>> {
		let chain = self
			.stills
			.entry(path.to_path_buf())
			.or_insert_with(|| load_still(path))
			.as_mut()?;
		let level = mip_level(scale) as usize;
		while chain.len() <= level {
			let previous = chain.last().expect("chain starts with the full picture");
			if previous.width() <= 1 && previous.height() <= 1 {
				break;
			}
			let (width, height) = ((previous.width() / 2).max(1), (previous.height() / 2).max(1));
			let smaller = image::imageops::resize(previous.as_ref(), width, height, image::imageops::Triangle);
			chain.push(Arc::new(smaller));
		}
		chain.get(level).or(chain.last()).cloned()
	}

	/// Video frame at `source_time` for the layer identified by `key`, decoded
	/// at a resolution suitable for drawing at `scale`.
	pub fn video_frame(
		&mut self,
		path: &Path,
		key: u64,
		info: &MediaInfo,
		source_time: f32,
		scale: f32,
	) -> Option<Arc<Picture>> {
		if info.width == 0 || info.height == 0 {
			return None;
		}
		let level = mip_level(scale);
		let renders = self.renders;
		let slot = self
			.videos
			.entry((path.to_path_buf(), key))
			.or_insert_with(|| VideoSlot {
				decoder: VideoDecoder::new(
					path,
					info.fps.unwrap_or(30.0),
					info.duration,
					level_size(info.width, info.height, level),
				),
				level,
				last_used: renders,
			});
		if slot.level != level {
			slot.decoder = VideoDecoder::new(
				path,
				info.fps.unwrap_or(30.0),
				info.duration,
				level_size(info.width, info.height, level),
			);
			slot.level = level;
		}
		slot.last_used = renders;
		slot.decoder.frame_at(source_time)
	}

	/// Call once per rendered frame: stops decoders that went unused.
	pub fn finish_render(&mut self) {
		self.renders += 1;
		let renders = self.renders;
		self.videos
			.retain(|_, slot| renders.saturating_sub(slot.last_used) <= VIDEO_IDLE_RENDERS);
	}
}

fn load_still(path: &Path) -> Option<Vec<Arc<Picture>>> {
	match image::open(path) {
		Ok(image) => {
			let mut picture = image.into_rgba8();
			premultiply(&mut picture);
			Some(vec![Arc::new(picture)])
		}
		Err(error) => {
			log::warn!("media: failed to load {}: {error}", path.display());
			None
		}
	}
}

fn premultiply(picture: &mut Picture) {
	for pixel in picture.pixels_mut() {
		let alpha = u16::from(pixel[3]);
		if alpha == 255 {
			continue;
		}
		for channel in 0..3 {
			pixel[channel] = ((u16::from(pixel[channel]) * alpha + 127) / 255) as u8;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn picks_mip_levels() {
		assert_eq!(mip_level(1.0), 0);
		assert_eq!(mip_level(0.5), 0);
		assert_eq!(mip_level(0.49), 1);
		assert_eq!(mip_level(0.25), 2);
		assert_eq!(mip_level(0.01), MAX_LEVEL);
		assert_eq!(mip_level(0.0), MAX_LEVEL);
	}

	#[test]
	fn stills_are_premultiplied_and_mipmapped() {
		let dir = std::env::temp_dir().join(format!("nade-media-still-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("half.png");
		image::RgbaImage::from_pixel(8, 4, image::Rgba([200, 100, 0, 128]))
			.save(&path)
			.unwrap();

		let mut cache = MediaCache::new();
		let full = cache.still(&path, 1.0).expect("png loads");
		assert_eq!(full.dimensions(), (8, 4));
		assert_eq!(full.get_pixel(0, 0).0, [100, 50, 0, 128]);
		assert_eq!(cache.still(&path, 0.25).expect("mip").dimensions(), (2, 1));
		assert!(cache.still(&dir.join("missing.png"), 1.0).is_none());
	}

	#[test]
	fn video_decoders_are_reused_and_evicted() {
		let Some(path) = crate::video::tests::numbered_clip() else {
			return;
		};
		let info = MediaInfo {
			width: 64,
			height: 36,
			duration: Some(2.0),
			fps: Some(30.0),
			has_video: true,
			has_audio: false,
		};
		let mut cache = MediaCache::new();
		assert_eq!(
			cache.video_frame(&path, 1, &info, 0.0, 1.0).unwrap().dimensions(),
			(64, 36)
		);
		assert_eq!(
			cache.video_frame(&path, 1, &info, 0.1, 0.3).unwrap().dimensions(),
			(32, 18)
		);
		assert_eq!(cache.videos.len(), 1);
		for _ in 0..=VIDEO_IDLE_RENDERS + 1 {
			cache.finish_render();
		}
		assert!(cache.videos.is_empty());
	}
}
