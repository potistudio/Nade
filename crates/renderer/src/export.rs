//! Rendering a whole composition to a movie file.

use anyhow::{Result, bail};
use core::FrameBuffer;
use domain::{Composition, Instance};
use encoder::{AudioClip, Encoder, ExportSettings, FfmpegEncoder};
use std::path::Path;

use crate::{CompositionLibrary, Compositor, Scene};

/// Output settings for the `[start, end]` seconds of `composition`
/// (usually its work area), including its audible clips re-timed so the
/// output starts at zero.
pub fn export_settings(
	composition: &Composition,
	path: &Path,
	range: [f32; 2],
	is_track_rendered: impl Fn(usize) -> bool,
	library: &(impl CompositionLibrary + ?Sized),
) -> ExportSettings {
	let mut clips = Vec::new();
	collect_audio(
		composition,
		&is_track_rendered,
		library,
		&mut vec![composition.id()],
		&mut clips,
	);
	let [start, end] = range;
	let end = end.max(start);
	let audio = clips
		.into_iter()
		.filter_map(|clip| retime(clip, -start, [start, end].map(|time| time - start)))
		.collect();
	ExportSettings {
		path: path.to_path_buf(),
		width: composition.width().max(1),
		height: composition.height().max(1),
		fps: composition.fps().max(1.0),
		duration: end - start,
		audio,
	}
}

/// Audible clips of `composition` in its own time, nested comps included.
fn collect_audio(
	composition: &Composition,
	is_track_rendered: &dyn Fn(usize) -> bool,
	library: &(impl CompositionLibrary + ?Sized),
	stack: &mut Vec<core::CompositionId>,
	out: &mut Vec<AudioClip>,
) {
	for instance in composition
		.all_objects()
		.filter(|instance| is_track_rendered(instance.track_index()))
	{
		if let Some(source) = instance.content().as_media() {
			if source.has_audio() {
				out.push(AudioClip {
					path: source.path.clone(),
					source_offset: instance.source_offset,
					start: instance.start_time(),
					duration: instance.duration(),
				});
			}
			continue;
		}
		let Some(id) = instance.content().as_composition() else {
			continue;
		};
		let Some(nested) = library
			.composition(id)
			.filter(|_| !stack.contains(&id) && stack.len() < 16)
		else {
			continue;
		};
		stack.push(id);
		let mut inner = Vec::new();
		collect_audio(nested, &|_| true, library, stack, &mut inner);
		stack.pop();
		out.extend(inner.into_iter().filter_map(|clip| shift_into_layer(clip, instance)));
	}
}

/// Map a clip from a nested comp's time onto the comp layer playing it,
/// trimmed to the layer's in / out points.
fn shift_into_layer(clip: AudioClip, layer: &Instance) -> Option<AudioClip> {
	retime(
		clip,
		layer.start_time() - layer.source_offset,
		[layer.start_time(), layer.end_time()],
	)
}

/// Move a clip by `shift` seconds and keep only the part inside `window`.
fn retime(clip: AudioClip, shift: f32, window: [f32; 2]) -> Option<AudioClip> {
	let start = clip.start + shift;
	let end = start + clip.duration;
	let (visible_start, visible_end) = (start.max(window[0]), end.min(window[1]));
	(visible_end - visible_start > 1e-4).then(|| AudioClip {
		source_offset: clip.source_offset + (visible_start - start),
		start: visible_start,
		duration: visible_end - visible_start,
		path: clip.path,
	})
}

/// Number of frames written for `settings`.
pub fn frame_count(settings: &ExportSettings) -> u32 {
	((settings.duration * settings.fps).round() as u32).max(1)
}

/// Render every frame of `composition` from `start` seconds on and encode it
/// with FFmpeg (`settings` from [`export_settings`] with the same start).
///
/// `progress(done, total)` is called after each frame; returning `false`
/// cancels the export and removes the partial file.
pub fn export_composition(
	composition: &Composition,
	settings: ExportSettings,
	start: f32,
	is_track_rendered: impl Fn(usize) -> bool,
	library: &(impl CompositionLibrary + ?Sized),
	mut progress: impl FnMut(u32, u32) -> bool,
) -> Result<()> {
	let total = frame_count(&settings);
	let fps = settings.fps;
	let mut encoder = FfmpegEncoder::new(settings);
	encoder.prepare()?;
	let mut compositor = Compositor::new();

	for index in 0..total {
		let time = start + index as f32 / fps;
		let scene = Scene::from_composition(composition, time, 1.0, &is_track_rendered, library);
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
		add_audio_layers(&mut composition);
		composition
	}

	fn add_audio_layers(composition: &mut Composition) {
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
	}

	#[test]
	fn settings_collect_audible_clips() {
		let composition = composition_with_audio();
		let settings = export_settings(&composition, Path::new("out.mp4"), [0.0, 1.0], |_| true, &());
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
		let muted = export_settings(&composition, Path::new("out.mp4"), [0.0, 1.0], |track| track == 0, &());
		assert!(muted.audio.is_empty(), "hidden tracks are muted");
	}

	#[test]
	fn work_area_exports_shift_audio_to_zero() {
		let composition = composition_with_audio();
		let settings = export_settings(&composition, Path::new("out.mp4"), [0.5, 1.0], |_| true, &());
		assert_eq!(frame_count(&settings), 5);
		let clip = &settings.audio[0];
		assert!(clip.start.abs() < 1e-5, "{clip:?}");
		assert!((clip.duration - 0.25).abs() < 1e-5, "{clip:?}");
		assert!((clip.source_offset - 2.25).abs() < 1e-5, "{clip:?}");
	}

	#[test]
	fn nested_audio_follows_the_comp_layer() {
		let mut project = domain::Project::default();
		let main = project.create_composition("Main", 64, 36, 10.0);
		let inner = project.create_composition("Inner", 64, 36, 10.0);
		add_audio_layers(project.composition_mut(&inner).unwrap());
		let layer = project
			.composition_mut(&main)
			.unwrap()
			.add_instance_at(NodeId::new(9), "Inner", 2.0, 0.5);
		layer.set_content(InstanceContent::Composition(inner));
		// ネスト側 0.5s から再生 → 音（0.25..0.75）の後半 0.25s だけが 2.0s から鳴る
		layer.source_offset = 0.5;

		let comp = project.composition(&main).unwrap();
		let settings = export_settings(comp, Path::new("out.mp4"), [0.0, 10.0], |_| true, &project);
		assert_eq!(settings.audio.len(), 1);
		let clip = &settings.audio[0];
		assert!((clip.start - 2.0).abs() < 1e-5, "{clip:?}");
		assert!((clip.duration - 0.25).abs() < 1e-5, "{clip:?}");
		assert!((clip.source_offset - 2.25).abs() < 1e-5, "{clip:?}");
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
		let settings = export_settings(&composition, &output, [0.0, 1.0], |_| true, &());
		let mut calls = 0;
		export_composition(
			&composition,
			settings,
			0.0,
			|_| true,
			&(),
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
		let settings = export_settings(&composition, &output, [0.0, 1.0], |_| true, &());
		let result = export_composition(&composition, settings, 0.0, |_| true, &(), |done, _| done < 3);
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
