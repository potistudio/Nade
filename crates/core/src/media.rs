//! Facts about imported media files.

use serde::{Deserialize, Serialize};

/// Stream facts probed from a media file on disk.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct MediaInfo {
	/// Picture width in pixels (0 for audio-only files).
	pub width: u32,
	/// Picture height in pixels (0 for audio-only files).
	pub height: u32,
	/// Playable length in seconds. `None` for still images.
	pub duration: Option<f32>,
	/// Average video frame rate. `None` for stills and audio.
	pub fps: Option<f32>,
	pub has_video: bool,
	pub has_audio: bool,
}

impl MediaInfo {
	/// Still image of the given size.
	pub fn still(width: u32, height: u32) -> Self {
		Self {
			width,
			height,
			has_video: true,
			..Self::default()
		}
	}

	/// Whether the file has moving pictures (not a still image).
	pub fn is_moving(&self) -> bool {
		self.has_video && self.duration.is_some()
	}
}
