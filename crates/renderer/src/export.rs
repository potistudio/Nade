//! Rendering a whole composition to a movie file.

use anyhow::{Result, bail};
use core::FrameBuffer;
use domain::Composition;
use encoder::{AudioClip, Encoder, ExportSettings, FfmpegEncoder};
use std::path::Path;

use crate::{Compositor, Scene};

/// Output settings for `composition`, including its audible clips.
pub fn export_settings(
	composition: &Composition,
	path: &Path,
	is_track_rendered: impl Fn(usize) -> bool,
) -> ExportSettings {
	let audio = composition
		.all_objects()
		.filter(|instance| is_track_rendered(instance.track_index()))
		.filter_map(|instance| {
			let source = instance.content().as_media()?;
			if !source.has_audio() {
				return None;
			}
			Some(AudioClip {
				path: source.path.clone(),
				source_offset: instance.source_offset,
				start: instance.start_time(),
				duration: instance.duration(),
			})
		})
		.collect();
	ExportSettings {
		path: path.to_path_buf(),
		width: composition.width().max(1),
		height: composition.height().max(1),
		fps: composition.fps().max(1.0),
		duration: composition.duration(),
		audio,
	}
}

/// Number of frames written for `settings`.
pub fn frame_count(settings: &ExportSettings) -> u32 {
	((settings.duration * settings.fps).round() as u32).max(1)
}

/// Render every frame of `composition` and encode it with FFmpeg.
///
/// `progress(done, total)` is called after each frame; returning `false`
/// cancels the export and removes the partial file.
pub fn export_composition(
	composition: &Composition,
	settings: ExportSettings,
	is_track_rendered: impl Fn(usize) -> bool,
	mut progress: impl FnMut(u32, u32) -> bool,
) -> Result<()> {
	let total = frame_count(&settings);
	let fps = settings.fps;
	let mut encoder = FfmpegEncoder::new(settings);
	encoder.prepare()?;
	let mut compositor = Compositor::new();

	for index in 0..total {
		let time = index as f32 / fps;
		let scene = Scene::from_composition(composition, time, 1.0, &is_track_rendered);
		let image = compositor.render(&scene);
		let frame = FrameBuffer {
			width: scene.width,
			height: scene.height,
			data: image.into_raw(),
		};
		encoder.encode_frame(&frame, index)?;
		if !progress(index + 1, total) {
			encoder.abort();
			bail!("export cancelled");
		}
	}
	encoder.finish()
}

#[cfg(test)]
mod tests {
	use super::*;
	use core::{AssetId, CompositionId, MediaInfo, NodeId};
	use domain::{InstanceContent, MediaKind, MediaSource};
	use std::path::PathBuf;

	fn composition_with_audio() -> Composition {
		let mut composition = Composition::new(CompositionId::new(0), "Export", 64, 36, 10.0);
		composition.set_duration(1.0);
		let solid = composition.add_instance_at(NodeId::new(0), "Solid", 0.0, 1.0);
		solid.set_content(InstanceContent::solid(32.0, 18.0, [1.0, 0.0, 0.0, 1.0]));
		let sound = composition.add_instance_at(NodeId::new(1), "Sound", 0.25, 0.5);
		sound.source_offset = 2.0;
		sound.set_content(InstanceContent::Media(MediaSource {
			asset: AssetId::new(0),
			kind: MediaKind::Audio,
			path: PathBuf::from("tone.wav"),
			info: MediaInfo {
				duration: Some(10.0),
				has_audio: true,
				..MediaInfo::default()
			},
		}));
		composition
	}

	#[test]
	fn settings_collect_audible_clips() {
		let composition = composition_with_audio();
		let settings = export_settings(&composition, Path::new("out.mp4"), |_| true);
		assert_eq!((settings.width, settings.height, settings.fps), (64, 36, 10.0));
		assert_eq!(frame_count(&settings), 10);
		assert_eq!(
			settings.audio,
			vec![AudioClip {
				path: PathBuf::from("tone.wav"),
				source_offset: 2.0,
				start: 0.25,
				duration: 0.5,
			}]
		);
		let muted = export_settings(&composition, Path::new("out.mp4"), |track| track == 0);
		assert!(muted.audio.is_empty(), "hidden tracks are muted");
	}

	#[test]
	fn exports_video_with_mixed_audio() {
		let dir = std::env::temp_dir().join(format!("nade-export-test-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let tone = dir.join("tone.wav");
		let made = std::process::Command::new(media::ffmpeg_path())
			.args([
				"-v",
				"error",
				"-y",
				"-f",
				"lavfi",
				"-i",
				"sine=frequency=440:duration=4",
			])
			.arg(&tone)
			.status()
			.is_ok_and(|status| status.success());
		if !made {
			eprintln!("ffmpeg unavailable; skipping");
			return;
		}

		let mut composition = composition_with_audio();
		let sound = composition.all_objects().nth(1).unwrap().id();
		if let Some(InstanceContent::Media(source)) = composition.get_mut(sound).map(|i| i.content_mut()) {
			source.path = tone.clone();
		}
		let output = dir.join("export.mp4");
		let settings = export_settings(&composition, &output, |_| true);
		let mut calls = 0;
		export_composition(
			&composition,
			settings,
			|_| true,
			|done, total| {
				calls += 1;
				assert!(done <= total);
				true
			},
		)
		.expect("export succeeds");
		assert_eq!(calls, 10);

		let info = media::probe(&output).expect("output is readable");
		assert!(info.has_video && info.has_audio);
		assert_eq!((info.width, info.height), (64, 36));
		assert!((info.duration.unwrap() - 1.0).abs() < 0.15, "{:?}", info.duration);
	}

	#[test]
	fn cancelling_removes_the_file() {
		let dir = std::env::temp_dir().join(format!("nade-export-cancel-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let mut composition = Composition::new(CompositionId::new(0), "Export", 64, 36, 10.0);
		composition.set_duration(1.0);
		let output = dir.join("cancel.mp4");
		let settings = export_settings(&composition, &output, |_| true);
		let result = export_composition(&composition, settings, |_| true, |done, _| done < 3);
		if result
			.as_ref()
			.is_err_and(|error| error.to_string().contains("starting"))
		{
			return;
		}
		assert!(result.is_err());
		assert!(!output.exists());
	}
}
