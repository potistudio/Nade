//! Frame-accurate video decoding through an `ffmpeg` pipe.
//!
//! A decoder keeps one `ffmpeg` process streaming raw RGBA frames from the
//! last seek point. Requests for the next few frames (playback) are served by
//! reading ahead on the pipe; anything else (scrubbing, jumping back) restarts
//! the process with a fast input seek.

use anyhow::{Context, Result};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::Arc;

use crate::Picture;
use crate::tools::ffmpeg_path;

/// Frames we are willing to decode and drop instead of re-seeking.
const MAX_READ_AHEAD_SECS: f32 = 1.5;

struct Stream {
	child: Child,
	stdout: BufReader<ChildStdout>,
	/// Index of the frame the next read returns.
	next_index: i64,
}

impl Drop for Stream {
	fn drop(&mut self) {
		let _ = self.child.kill();
		let _ = self.child.wait();
	}
}

/// Decodes frames of one video file at a fixed output size.
pub struct VideoDecoder {
	path: PathBuf,
	fps: f32,
	/// Number of frames in the file, when known.
	frame_count: Option<i64>,
	/// Output size of decoded frames.
	size: (u32, u32),
	stream: Option<Stream>,
	last: Option<(i64, Arc<Picture>)>,
}

impl VideoDecoder {
	/// `size` is the output picture size; frames are scaled by FFmpeg.
	pub fn new(path: impl Into<PathBuf>, fps: f32, duration: Option<f32>, size: (u32, u32)) -> Self {
		let fps = if fps.is_finite() && fps > 0.0 { fps } else { 30.0 };
		Self {
			path: path.into(),
			fps,
			frame_count: duration.map(|duration| ((duration * fps).round() as i64).max(1)),
			size: (size.0.max(1), size.1.max(1)),
			stream: None,
			last: None,
		}
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn size(&self) -> (u32, u32) {
		self.size
	}

	/// Frame index shown at `time` seconds into the footage.
	pub fn frame_index(&self, time: f32) -> i64 {
		let index = (f64::from(time.max(0.0)) * f64::from(self.fps) + 1e-3).floor() as i64;
		match self.frame_count {
			Some(count) => index.min(count - 1),
			None => index,
		}
	}

	/// Picture shown at `time` seconds into the footage. Past the end the last
	/// decoded frame is held.
	pub fn frame_at(&mut self, time: f32) -> Option<Arc<Picture>> {
		let index = self.frame_index(time);
		if let Some((last_index, picture)) = &self.last
			&& *last_index == index
		{
			return Some(Arc::clone(picture));
		}

		let read_ahead = (MAX_READ_AHEAD_SECS * self.fps) as i64;
		let reusable = self
			.stream
			.as_ref()
			.is_some_and(|stream| index >= stream.next_index && index - stream.next_index <= read_ahead);
		if !reusable {
			self.stream = None;
			match self.spawn(index) {
				Ok(stream) => self.stream = Some(stream),
				Err(error) => {
					log::warn!("video: {error:#}");
					return self.last.as_ref().map(|(_, picture)| Arc::clone(picture));
				}
			}
		}

		match self.read_until(index) {
			Some(picture) => {
				let picture = Arc::new(picture);
				self.last = Some((index, Arc::clone(&picture)));
				Some(picture)
			}
			None => {
				// 終端に達した: 最後のフレームを保持する
				self.stream = None;
				if self.frame_count.is_none_or(|count| index < count - 1) {
					self.frame_count = Some(index.max(1));
				}
				self.last.as_ref().map(|(_, picture)| Arc::clone(picture))
			}
		}
	}

	fn read_until(&mut self, index: i64) -> Option<Picture> {
		let (width, height) = self.size;
		let stream = self.stream.as_mut()?;
		let mut buffer = vec![0u8; width as usize * height as usize * 4];
		while stream.next_index <= index {
			if stream.stdout.read_exact(&mut buffer).is_err() {
				return None;
			}
			stream.next_index += 1;
		}
		Picture::from_raw(width, height, buffer)
	}

	fn spawn(&self, index: i64) -> Result<Stream> {
		let (width, height) = self.size;
		// わずかに手前へシークし、目的のフレームが確実に先頭に来るようにする
		let seek = ((index as f64 - 0.25) / f64::from(self.fps)).max(0.0);
		let mut child = Command::new(ffmpeg_path())
			.args(["-v", "error", "-nostdin", "-ss"])
			.arg(format!("{seek:.6}"))
			.arg("-i")
			.arg(&self.path)
			.args(["-an", "-sn", "-vf"])
			.arg(format!("scale={width}:{height}:flags=bilinear"))
			.arg("-r")
			.arg(format!("{}", self.fps))
			.args(["-pix_fmt", "rgba", "-f", "rawvideo", "-"])
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::null())
			.spawn()
			.with_context(|| format!("starting {} for {}", ffmpeg_path().display(), self.path.display()))?;
		let stdout = child.stdout.take().context("ffmpeg stdout")?;
		Ok(Stream {
			child,
			stdout: BufReader::with_capacity(width as usize * height as usize * 4, stdout),
			next_index: index,
		})
	}
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use std::process::Command;

	/// 64×36, 30fps, 2s. Frame `n` has red = `n * 4`.
	pub(crate) fn numbered_clip() -> Option<PathBuf> {
		static CLIP: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
		CLIP.get_or_init(create_numbered_clip).clone()
	}

	fn create_numbered_clip() -> Option<PathBuf> {
		let dir = std::env::temp_dir().join(format!("nade-media-test-{}", std::process::id()));
		std::fs::create_dir_all(&dir).ok()?;
		let path = dir.join("numbered.mkv");
		let status = Command::new(ffmpeg_path())
			.args([
				"-v",
				"error",
				"-y",
				"-f",
				"lavfi",
				"-i",
				"color=c=black:s=64x36:r=30:d=2,format=rgb24,geq=r='N*4':g='0':b='0'",
				"-c:v",
				"ffv1",
			])
			.arg(&path)
			.status()
			.ok()?;
		status.success().then_some(path)
	}

	fn red(picture: &Picture) -> u8 {
		picture.get_pixel(32, 18)[0]
	}

	#[test]
	fn decodes_requested_frames_in_order_and_after_seeks() {
		let Some(path) = numbered_clip() else {
			eprintln!("ffmpeg unavailable; skipping");
			return;
		};
		let mut decoder = VideoDecoder::new(&path, 30.0, Some(2.0), (64, 36));

		for (time, frame) in [(0.0, 0), (1.0 / 30.0, 1), (0.1, 3), (0.5, 15), (0.2, 6), (1.99, 59)] {
			let picture = decoder.frame_at(time).expect("frame decodes");
			assert_eq!(picture.dimensions(), (64, 36));
			let expected = frame * 4;
			assert!(
				(i32::from(red(&picture)) - expected).abs() <= 2,
				"time {time}: expected red {expected}, got {}",
				red(&picture)
			);
		}

		// 終端を越えたら最後のフレームを保持
		let held = decoder.frame_at(5.0).expect("last frame is held");
		assert!((i32::from(red(&held)) - 59 * 4).abs() <= 2);
	}

	#[test]
	fn scales_output() {
		let Some(path) = numbered_clip() else {
			return;
		};
		let mut decoder = VideoDecoder::new(&path, 30.0, Some(2.0), (32, 18));
		assert_eq!(decoder.frame_at(0.0).expect("frame").dimensions(), (32, 18));
	}
}
