use super::*;
use anyhow::{Context, Result, bail};
use core::FrameBuffer;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::JoinHandle;

/// A piece of sound placed on the output timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioClip {
	/// File to read the sound from.
	pub path: PathBuf,
	/// Position in the file where the clip starts (seconds).
	pub source_offset: f32,
	/// Position on the output timeline (seconds).
	pub start: f32,
	/// Length of the clip (seconds).
	pub duration: f32,
}

/// Output file description.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
	pub path: PathBuf,
	pub width: u32,
	pub height: u32,
	pub fps: f32,
	/// Total length of the output (seconds).
	pub duration: f32,
	/// Sound mixed into the output.
	pub audio: Vec<AudioClip>,
}

impl ExportSettings {
	/// `ffmpeg` arguments that read raw RGBA frames from stdin.
	pub fn ffmpeg_args(&self) -> Vec<String> {
		let mut args: Vec<String> = [
			"-v",
			"error",
			"-y",
			"-f",
			"rawvideo",
			"-pixel_format",
			"rgba",
			"-video_size",
		]
		.map(String::from)
		.to_vec();
		args.push(format!("{}x{}", self.width, self.height));
		args.extend(["-framerate".to_string(), format!("{}", self.fps)]);
		args.extend(["-i", "-"].map(String::from));

		let audio: Vec<&AudioClip> = self
			.audio
			.iter()
			.filter(|clip| clip.duration > 0.0 && clip.start < self.duration)
			.collect();
		args.extend(audio_inputs(&audio));

		let mut filters = Vec::new();
		// yuv420p は偶数サイズが必要
		if self.width % 2 == 1 || self.height % 2 == 1 {
			filters.push("[0:v]pad=ceil(iw/2)*2:ceil(ih/2)*2[vout]".to_string());
		}
		if !audio.is_empty() {
			filters.push(audio_mix_graph(&audio, 1, ""));
		}
		if !filters.is_empty() {
			args.extend(["-filter_complex".to_string(), filters.join(";")]);
		}

		let video = if filters.iter().any(|filter| filter.ends_with("[vout]")) {
			"[vout]"
		} else {
			"0:v"
		};
		args.extend(["-map".to_string(), video.to_string()]);
		if !audio.is_empty() {
			args.extend(["-map", "[aout]", "-c:a", "aac", "-b:a", "192k"].map(String::from));
		}
		args.extend(
			[
				"-c:v",
				"libx264",
				"-preset",
				"medium",
				"-crf",
				"18",
				"-pix_fmt",
				"yuv420p",
				"-movflags",
				"+faststart",
				"-t",
			]
			.map(String::from),
		);
		args.push(format!("{:.6}", self.duration));
		args.push(self.path.to_string_lossy().into_owned());
		args
	}
}

/// `-ss/-t/-i` arguments reading each clip's span of its file.
fn audio_inputs(clips: &[&AudioClip]) -> Vec<String> {
	clips
		.iter()
		.flat_map(|clip| {
			[
				"-ss".to_string(),
				format!("{:.6}", clip.source_offset.max(0.0)),
				"-t".to_string(),
				format!("{:.6}", clip.duration),
				"-i".to_string(),
				clip.path.to_string_lossy().into_owned(),
			]
		})
		.collect()
}

/// Filter graph placing each clip at its start time and mixing them into
/// `[aout]`. `first_input` is the ffmpeg input index of the first clip;
/// `post` is appended after the mix (e.g. `,apad`).
fn audio_mix_graph(clips: &[&AudioClip], first_input: usize, post: &str) -> String {
	let mut filters = Vec::new();
	let mut mix_inputs = String::new();
	for (index, clip) in clips.iter().enumerate() {
		let delay = (clip.start.max(0.0) * 1000.0).round() as u64;
		filters.push(format!(
			"[{input}:a]aresample=48000,adelay=delays={delay}:all=1[a{index}]",
			input = index + first_input
		));
		mix_inputs.push_str(&format!("[a{index}]"));
	}
	filters.push(format!(
		"{mix_inputs}amix=inputs={}:normalize=0:dropout_transition=0{post}[aout]",
		clips.len()
	));
	filters.join(";")
}

/// `ffmpeg` arguments that mix `clips` from timeline time `from` to `until`
/// into raw interleaved little-endian `f32` samples on stdout, padded with
/// silence so the stream always lasts the full span. `None` when nothing is
/// audible in that span.
pub fn preview_audio_args(clips: &[AudioClip], from: f32, until: f32, rate: u32, channels: u16) -> Option<Vec<String>> {
	let span = until - from;
	if span <= 0.0 {
		return None;
	}
	// 再生開始位置より前の部分を切り落とし、開始位置を 0 秒とする
	let shifted: Vec<AudioClip> = clips
		.iter()
		.filter(|clip| clip.duration > 0.0 && clip.start + clip.duration > from && clip.start < until)
		.map(|clip| {
			let skipped = (from - clip.start).max(0.0);
			AudioClip {
				path: clip.path.clone(),
				source_offset: clip.source_offset + skipped,
				start: (clip.start - from).max(0.0),
				duration: clip.duration - skipped,
			}
		})
		.collect();
	if shifted.is_empty() {
		return None;
	}
	let refs: Vec<&AudioClip> = shifted.iter().collect();
	let mut args: Vec<String> = ["-v", "error", "-nostdin"].map(String::from).to_vec();
	args.extend(audio_inputs(&refs));
	args.extend(["-filter_complex".to_string(), audio_mix_graph(&refs, 0, ",apad")]);
	args.extend(["-map", "[aout]", "-t"].map(String::from));
	args.push(format!("{span:.6}"));
	args.extend(["-f", "f32le", "-ac"].map(String::from));
	args.push(channels.to_string());
	args.push("-ar".to_string());
	args.push(rate.to_string());
	args.push("-".to_string());
	Some(args)
}

/// Encodes frames to H.264 (plus AAC sound) through an `ffmpeg` process.
pub struct FfmpegEncoder {
	settings: ExportSettings,
	ffmpeg: Option<Child>,
	stdin: Option<ChildStdin>,
	stderr: Option<JoinHandle<String>>,
}

impl FfmpegEncoder {
	pub fn new(settings: ExportSettings) -> Self {
		Self {
			settings,
			ffmpeg: None,
			stdin: None,
			stderr: None,
		}
	}

	pub fn settings(&self) -> &ExportSettings {
		&self.settings
	}

	fn error_output(&mut self) -> String {
		self.stderr
			.take()
			.and_then(|handle| handle.join().ok())
			.map(|text| text.trim().to_string())
			.unwrap_or_default()
	}

	/// Stop the process and remove the partial output.
	pub fn abort(&mut self) {
		drop(self.stdin.take());
		if let Some(mut child) = self.ffmpeg.take() {
			let _ = child.kill();
			let _ = child.wait();
		}
		let _ = std::fs::remove_file(&self.settings.path);
	}
}

impl Encoder for FfmpegEncoder {
	fn prepare(&mut self) -> Result<()> {
		log::debug!("Preparing FFmpeg encoder: {:?}", self.settings);
		let mut child = Command::new(media::ffmpeg_path())
			.args(self.settings.ffmpeg_args())
			.stdin(Stdio::piped())
			.stdout(Stdio::null())
			.stderr(Stdio::piped())
			.spawn()
			.with_context(|| format!("starting {}", media::ffmpeg_path().display()))?;
		self.stdin = child.stdin.take();
		// stderr を溜め込むと ffmpeg が書き込みで止まるので別スレッドで読み切る
		self.stderr = child.stderr.take().map(|mut stderr| {
			std::thread::spawn(move || {
				let mut text = String::new();
				let _ = stderr.read_to_string(&mut text);
				text
			})
		});
		self.ffmpeg = Some(child);
		Ok(())
	}

	fn encode_frame(&mut self, frame: &FrameBuffer, index: u32) -> Result<()> {
		log::trace!("Encoding frame [{index}]");
		let stdin = self.stdin.as_mut().context("encoder is not prepared")?;
		if let Err(error) = stdin.write_all(&frame.data) {
			drop(self.stdin.take());
			if let Some(mut child) = self.ffmpeg.take() {
				let _ = child.wait();
			}
			let output = self.error_output();
			bail!("ffmpeg stopped accepting frames ({error}): {output}");
		}
		Ok(())
	}

	fn finish(&mut self) -> Result<()> {
		drop(self.stdin.take());
		let status = self.ffmpeg.take().context("encoder is not prepared")?.wait()?;
		let output = self.error_output();
		log::debug!("Finished FFmpeg with status: {status}");
		if !status.success() {
			bail!("ffmpeg failed ({status}): {output}");
		}
		Ok(())
	}
}

impl Drop for FfmpegEncoder {
	fn drop(&mut self) {
		if self.ffmpeg.is_some() {
			self.abort();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn settings(audio: Vec<AudioClip>) -> ExportSettings {
		ExportSettings {
			path: PathBuf::from("/tmp/out.mp4"),
			width: 64,
			height: 36,
			fps: 30.0,
			duration: 1.0,
			audio,
		}
	}

	#[test]
	fn video_only_maps_the_pipe() {
		let args = settings(Vec::new()).ffmpeg_args();
		assert!(args.windows(2).any(|pair| pair == ["-map", "0:v"]));
		assert!(!args.contains(&"-filter_complex".to_string()));
		assert_eq!(args.last().map(String::as_str), Some("/tmp/out.mp4"));
	}

	#[test]
	fn audio_clips_are_delayed_and_mixed() {
		let clip = |start: f32| AudioClip {
			path: PathBuf::from("a.wav"),
			source_offset: 0.5,
			start,
			duration: 0.25,
		};
		let args = settings(vec![clip(0.25), clip(0.5), clip(3.0)]).ffmpeg_args();
		let graph = &args[args.iter().position(|arg| arg == "-filter_complex").unwrap() + 1];
		assert!(graph.contains("[1:a]aresample=48000,adelay=delays=250:all=1[a0]"));
		assert!(
			graph.contains("[a0][a1]amix=inputs=2"),
			"clips past the end are dropped: {graph}"
		);
		assert!(args.windows(2).any(|pair| pair == ["-ss", "0.500000"]));
	}

	#[test]
	fn preview_audio_starts_mid_clip() {
		let clip = AudioClip {
			path: PathBuf::from("a.wav"),
			source_offset: 1.0,
			start: 2.0,
			duration: 4.0,
		};
		let later = AudioClip {
			start: 8.0,
			..clip.clone()
		};
		let args = preview_audio_args(&[clip.clone(), later], 3.0, 10.0, 44100, 2).expect("audible");
		assert!(args.windows(2).any(|pair| pair == ["-ss", "2.000000"]), "{args:?}");
		assert!(args.windows(2).any(|pair| pair == ["-t", "3.000000"]));
		let graph = &args[args.iter().position(|arg| arg == "-filter_complex").unwrap() + 1];
		assert!(graph.contains("[0:a]aresample=48000,adelay=delays=0:all=1[a0]"));
		assert!(graph.contains("[1:a]aresample=48000,adelay=delays=5000:all=1[a1]"));
		assert!(graph.ends_with(",apad[aout]"));
		assert!(args.windows(2).any(|pair| pair == ["-t", "7.000000"]));
		assert!(args.windows(2).any(|pair| pair == ["-ar", "44100"]));

		assert!(preview_audio_args(&[clip], 6.5, 10.0, 48000, 2).is_none());
	}

	#[test]
	fn encodes_a_short_clip() {
		let dir = std::env::temp_dir().join(format!("nade-encoder-test-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let mut settings = settings(Vec::new());
		settings.width = 65; // 奇数サイズもパディングで通る
		settings.path = dir.join("out.mp4");
		let mut encoder = FfmpegEncoder::new(settings.clone());
		if encoder.prepare().is_err() {
			eprintln!("ffmpeg unavailable; skipping");
			return;
		}
		let frame = FrameBuffer {
			width: settings.width,
			height: settings.height,
			data: vec![128; (settings.width * settings.height * 4) as usize],
		};
		for index in 0..30 {
			encoder.encode_frame(&frame, index).unwrap();
		}
		encoder.finish().unwrap();
		assert!(std::fs::metadata(&settings.path).unwrap().len() > 0);
		let info = media::probe(&settings.path).unwrap();
		assert_eq!((info.width, info.height), (66, 36));
	}
}
