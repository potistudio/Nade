//! Reading stream facts from media files.

use anyhow::{Context, Result, bail};
use core::MediaInfo;
use std::path::Path;
use std::process::Command;

use crate::tools::ffprobe_path;

/// Whether the file is a still image decoded by the `image` crate.
pub fn is_still_image(path: &Path) -> bool {
	path.extension()
		.and_then(|extension| extension.to_str())
		.map(|extension| extension.to_ascii_lowercase())
		.is_some_and(|extension| {
			matches!(
				extension.as_str(),
				"png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff"
			)
		})
}

/// Inspect a media file on disk.
pub fn probe(path: &Path) -> Result<MediaInfo> {
	if is_still_image(path) {
		let (width, height) =
			image::image_dimensions(path).with_context(|| format!("reading image header of {}", path.display()))?;
		return Ok(MediaInfo::still(width, height));
	}

	let output = Command::new(ffprobe_path())
		.args([
			"-v",
			"error",
			"-show_entries",
			"stream=codec_type,width,height,avg_frame_rate,duration:stream_disposition=attached_pic:format=duration",
			"-of",
			"compact",
		])
		.arg(path)
		.output()
		.with_context(|| format!("running {}", ffprobe_path().display()))?;
	if !output.status.success() {
		bail!(
			"ffprobe failed for {}: {}",
			path.display(),
			String::from_utf8_lossy(&output.stderr).trim()
		);
	}
	parse_compact(&String::from_utf8_lossy(&output.stdout))
}

/// Parse `ffprobe -of compact` output.
fn parse_compact(output: &str) -> Result<MediaInfo> {
	let mut info = MediaInfo::default();
	let mut stream_duration: Option<f32> = None;
	let mut format_duration: Option<f32> = None;

	for line in output.lines() {
		let mut fields = line.split('|');
		let section = fields.next().unwrap_or_default();
		let get = |key: &str| {
			line.split('|')
				.skip(1)
				.filter_map(|field| field.split_once('='))
				.find(|(name, _)| *name == key)
				.map(|(_, value)| value)
		};
		match section {
			"stream" => match get("codec_type") {
				Some("video") if get("disposition:attached_pic") != Some("1") && !info.has_video => {
					info.has_video = true;
					info.width = get("width").and_then(|value| value.parse().ok()).unwrap_or(0);
					info.height = get("height").and_then(|value| value.parse().ok()).unwrap_or(0);
					info.fps = get("avg_frame_rate").and_then(parse_rate);
					stream_duration = get("duration").and_then(|value| value.parse().ok());
				}
				Some("audio") => {
					info.has_audio = true;
					if !info.has_video {
						stream_duration = stream_duration.or(get("duration").and_then(|value| value.parse().ok()));
					}
				}
				_ => {}
			},
			"format" => format_duration = get("duration").and_then(|value| value.parse().ok()),
			_ => {}
		}
	}

	if !info.has_video && !info.has_audio {
		bail!("no audio or video streams");
	}
	info.duration = stream_duration
		.or(format_duration)
		.filter(|duration| duration.is_finite() && *duration > 0.0);
	if info.has_video && info.duration.is_none() {
		// 長さの分からない映像（単一フレームの動画等）は静止画として扱う
		info.fps = None;
	}
	Ok(info)
}

/// `30000/1001` → 29.97
fn parse_rate(value: &str) -> Option<f32> {
	let (numerator, denominator) = value.split_once('/')?;
	let numerator: f32 = numerator.parse().ok()?;
	let denominator: f32 = denominator.parse().ok()?;
	(denominator > 0.0 && numerator > 0.0).then(|| numerator / denominator)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_video_with_audio() {
		let info = parse_compact(
			"stream|codec_type=video|width=1280|height=720|avg_frame_rate=30000/1001|duration=10.010000|disposition:attached_pic=0\n\
			 stream|codec_type=audio|width=N/A|height=N/A|avg_frame_rate=0/0|duration=10.050000|disposition:attached_pic=0\n\
			 format|duration=10.050000\n",
		)
		.expect("valid output");
		assert!(info.has_video && info.has_audio);
		assert_eq!((info.width, info.height), (1280, 720));
		assert!((info.fps.unwrap() - 29.97).abs() < 0.01);
		assert!((info.duration.unwrap() - 10.01).abs() < 1e-4);
	}

	#[test]
	fn ignores_cover_art_in_audio_files() {
		let info = parse_compact(
			"stream|codec_type=audio|duration=180.5|disposition:attached_pic=0\n\
			 stream|codec_type=video|width=600|height=600|avg_frame_rate=0/0|duration=N/A|disposition:attached_pic=1\n\
			 format|duration=180.500000\n",
		)
		.expect("valid output");
		assert!(!info.has_video && info.has_audio);
		assert_eq!(info.duration, Some(180.5));
	}

	#[test]
	fn rejects_files_without_streams() {
		assert!(parse_compact("format|duration=1.0\n").is_err());
	}
}
