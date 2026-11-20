//! # Audio
//!
//! Sound for the preview: the composition's audible clips are mixed by
//! `ffmpeg` (the same graph as export) into raw `f32` samples which are
//! streamed to the default output device with `cpal`.
//!
//! The number of samples actually handed to the device doubles as the
//! playback clock, so the picture can follow the sound.

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, I24, Sample, SampleFormat, SizedSample, StreamConfig, U24};
use nade_encoder::{AudioClip, preview_audio_args};
use std::collections::VecDeque;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Seconds of decoded sound kept ahead of the device.
const BUFFER_SECS: f32 = 0.5;
/// Samples read from ffmpeg in one go.
const CHUNK_SAMPLES: usize = 4096;

#[derive(Debug)]
struct Shared {
	samples: Mutex<VecDeque<f32>>,
	/// Frames (samples per channel) played so far.
	played: AtomicU64,
	stop: AtomicBool,
	/// Set once the decoder will not produce any more samples.
	decoder_done: AtomicBool,
	/// Set once no more sound will reach the device. This is delayed until the
	/// already-decoded buffer has been played so the clock never jumps ahead.
	ended: AtomicBool,
	rate: u32,
	channels: u16,
}

#[derive(Debug)]
struct Playback {
	from: f32,
	shared: Arc<Shared>,
	stop_tx: Sender<()>,
	output: Option<JoinHandle<()>>,
}

impl Drop for Playback {
	fn drop(&mut self) {
		self.shared.stop.store(true, Ordering::Relaxed);
		let _ = self.stop_tx.send(());
		if let Some(handle) = self.output.take() {
			let _ = handle.join();
		}
	}
}

/// A handle to the current audio preview playback, if any. Dropping it stops the sound.
#[derive(Debug, Default)]
pub struct AudioPreview {
	playback: Option<Playback>,
}

impl AudioPreview {
	/// Create a new handle.
	pub fn new() -> Self {
		Self::default()
	}

	/// Start playing `clips` from timeline time `from` until `until`
	/// (replacing any current playback). Returns `false` when there is nothing
	/// to hear or no output device.
	///
	/// The output starts on its own thread, so failures after this point are
	/// logged and reported by [`Self::has_ended`].
	pub fn play(&mut self, clips: &[AudioClip], from: f32, until: f32) -> bool {
		self.stop();
		match start(clips, from, until) {
			Ok(Some(playback)) => {
				self.playback = Some(playback);
				true
			}
			Ok(None) => false,
			Err(error) => {
				log::warn!("audio: {error:#}");
				false
			}
		}
	}

	/// Stop any current playback.
	pub fn stop(&mut self) {
		self.playback = None;
	}

	/// Returns whether a playback session exists. It may still be silent while
	/// the output starts, and it may already have ended before it is stopped.
	pub fn is_playing(&self) -> bool {
		self.playback.is_some()
	}

	/// Returns whether the current playback has ended or failed.
	///
	/// This becomes `true` once the decoder has no more samples and the
	/// already-decoded buffer has been handed to the device, or when the output
	/// stream fails. Callers should fall back to a wall clock instead of
	/// freezing on the last [`Self::clock`] value.
	pub fn has_ended(&self) -> bool {
		self.playback
			.as_ref()
			.is_some_and(|playback| playback.shared.ended.load(Ordering::Relaxed))
	}

	/// Returns the current playback position on the timeline in seconds.
	///
	/// The position is calculated from the start offset and the number of samples played so far.
	/// It no longer advances once [`Self::has_ended`] becomes `true`.
	///
	/// # Returns
	///
	/// * `Some(time)` - The elapsed timeline position in seconds once audio samples begin flowing.
	/// * `None` - If there is no active playback or no samples have been played yet (`played == 0`).
	pub fn clock(&self) -> Option<f32> {
		let playback = self.playback.as_ref()?;
		let played = playback.shared.played.load(Ordering::Relaxed);

		(played > 0).then(|| playback.from + played as f32 / playback.shared.rate as f32)
	}
}

// -----------------------------------------------------------------------------
//     Internal implementation
// -----------------------------------------------------------------------------

fn start(clips: &[AudioClip], from: f32, until: f32) -> Result<Option<Playback>> {
	let host = cpal::default_host();
	let device = host.default_output_device().context("no audio output device")?;
	let supported = device.default_output_config().context("querying the output format")?;
	let format = supported.sample_format();
	let config: StreamConfig = supported.into();
	let (rate, channels) = (config.sample_rate, config.channels);

	let Some(args) = preview_audio_args(clips, from, until, rate, channels) else {
		return Ok(None);
	};
	let shared = Arc::new(Shared {
		samples: Mutex::new(VecDeque::with_capacity(queue_limit(rate, channels) + CHUNK_SAMPLES)),
		played: AtomicU64::new(0),
		stop: AtomicBool::new(false),
		decoder_done: AtomicBool::new(false),
		ended: AtomicBool::new(false),
		rate,
		channels,
	});

	// Start the output first. If it fails, no decoder or ffmpeg process is left
	// behind, and the stream simply emits silence while the decoder warms up.
	// cpal のストリームはスレッド間で移動できないため、専用スレッドで保持する
	let (stop_tx, stop_rx) = mpsc::channel();
	let stream_shared = Arc::clone(&shared);
	let output = thread::Builder::new()
		.name("nade-audio-out".into())
		.spawn(move || run_output(device, config, format, stream_shared, stop_rx))
		.context("starting the audio thread")?;

	let child = match Command::new(nade_media::ffmpeg_path())
		.args(&args)
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
	{
		Ok(child) => child,
		Err(error) => {
			shutdown(&shared, &stop_tx, output);
			return Err(error).with_context(|| format!("starting {}", nade_media::ffmpeg_path().display()));
		}
	};
	if let Err(error) = spawn_decoder(child, Arc::clone(&shared)) {
		shutdown(&shared, &stop_tx, output);
		return Err(error);
	}

	log::debug!(
		"audio: playing {} clip(s) from {from:.3}s at {rate} Hz × {channels}",
		clips.len()
	);
	Ok(Some(Playback {
		from,
		shared,
		stop_tx,
		output: Some(output),
	}))
}

/// Stop the output thread and the decoder after a failed start.
fn shutdown(shared: &Shared, stop_tx: &Sender<()>, output: JoinHandle<()>) {
	shared.stop.store(true, Ordering::Relaxed);
	let _ = stop_tx.send(());
	let _ = output.join();
}

fn queue_limit(rate: u32, channels: u16) -> usize {
	(BUFFER_SECS * rate as f32) as usize * usize::from(channels)
}

/// Owns the ffmpeg process so it is killed if starting the decoder thread fails.
struct ChildGuard(Option<Child>);

impl ChildGuard {
	fn new(child: Child) -> Self {
		Self(Some(child))
	}

	fn get(&mut self) -> &mut Child {
		self.0.as_mut().expect("ffmpeg child")
	}

	fn kill(&mut self) {
		if let Some(mut child) = self.0.take() {
			let _ = child.kill();
			let _ = child.wait();
		}
	}

	fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
		let mut child = self.0.take().expect("ffmpeg child");
		child.wait()
	}
}

impl Drop for ChildGuard {
	fn drop(&mut self) {
		self.kill();
	}
}

/// Read samples from ffmpeg into the shared queue, keeping it short.
fn spawn_decoder(child: Child, shared: Arc<Shared>) -> Result<()> {
	let mut child = ChildGuard::new(child);
	let mut stdout = child.get().stdout.take().context("ffmpeg stdout")?;
	let stderr = child.get().stderr.take().and_then(|mut stderr| {
		thread::Builder::new()
			.name("nade-audio-ffmpeg-log".into())
			.spawn(move || {
				let mut text = String::new();
				let _ = stderr.read_to_string(&mut text);
				text
			})
			.ok()
	});
	let limit = queue_limit(shared.rate, shared.channels);
	thread::Builder::new()
		.name("nade-audio-decode".into())
		.spawn(move || {
			let mut child = child;
			let mut bytes = vec![0u8; CHUNK_SAMPLES * 4];
			let mut carry: Vec<u8> = Vec::new();
			let mut read_error = None;
			while !shared.stop.load(Ordering::Relaxed) {
				let queued = shared.samples.lock().map(|queue| queue.len()).unwrap_or(0);
				if queued > limit {
					thread::sleep(Duration::from_millis(5));
					continue;
				}
				let read = match stdout.read(&mut bytes) {
					Ok(0) => break,
					Ok(read) => read,
					Err(error) => {
						read_error = Some(error);
						break;
					}
				};
				carry.extend_from_slice(&bytes[..read]);
				let whole = carry.len() / 4 * 4;
				if let Ok(mut queue) = shared.samples.lock() {
					queue.extend(
						carry[..whole]
							.as_chunks::<4>()
							.0
							.iter()
							.map(|sample| f32::from_le_bytes(*sample)),
					);
				}
				carry.drain(..whole);
			}
			drop(stdout);
			// Let the output callback decide when the buffered tail is drained.
			shared.decoder_done.store(true, Ordering::Relaxed);

			let stopped = shared.stop.load(Ordering::Relaxed) || read_error.is_some();
			if stopped {
				child.kill();
			}
			let status = if stopped { None } else { Some(child.wait()) };
			let log = stderr.and_then(|handle| handle.join().ok()).unwrap_or_default();
			if let Some(error) = read_error {
				log::warn!("audio: reading ffmpeg output: {error}");
			}
			if let Some(status) = status {
				match status {
					Ok(status) if status.success() => {}
					Ok(status) => log::warn!("audio: ffmpeg stopped ({status}): {}", log.trim()),
					Err(error) => log::warn!("audio: waiting for ffmpeg: {error}"),
				}
			}
		})
		.context("starting the audio decoder thread")?;
	Ok(())
}

fn run_output(
	device: cpal::Device,
	config: StreamConfig,
	format: SampleFormat,
	shared: Arc<Shared>,
	stop_rx: Receiver<()>,
) {
	let stream = match format {
		SampleFormat::F32 => build::<f32>(&device, config, Arc::clone(&shared)),
		SampleFormat::I16 => build::<i16>(&device, config, Arc::clone(&shared)),
		SampleFormat::U16 => build::<u16>(&device, config, Arc::clone(&shared)),
		SampleFormat::I32 => build::<i32>(&device, config, Arc::clone(&shared)),
		SampleFormat::F64 => build::<f64>(&device, config, Arc::clone(&shared)),
		SampleFormat::I8 => build::<i8>(&device, config, Arc::clone(&shared)),
		SampleFormat::I24 => build::<I24>(&device, config, Arc::clone(&shared)),
		SampleFormat::I64 => build::<i64>(&device, config, Arc::clone(&shared)),
		SampleFormat::U8 => build::<u8>(&device, config, Arc::clone(&shared)),
		SampleFormat::U24 => build::<U24>(&device, config, Arc::clone(&shared)),
		SampleFormat::U32 => build::<u32>(&device, config, Arc::clone(&shared)),
		SampleFormat::U64 => build::<u64>(&device, config, Arc::clone(&shared)),
		other => Err(anyhow!("unsupported sample format {other}")),
	};
	match stream.and_then(|stream| stream.play().map(|()| stream).context("starting playback")) {
		Ok(stream) => {
			let _ = stop_rx.recv();
			drop(stream);
		}
		Err(error) => {
			log::warn!("audio: {error:#}");
			shared.stop.store(true, Ordering::Relaxed);
		}
	}
	shared.ended.store(true, Ordering::Release);
}

fn build<T>(device: &cpal::Device, config: StreamConfig, shared: Arc<Shared>) -> Result<cpal::Stream>
where
	T: SizedSample + FromSample<f32>,
{
	let channels = usize::from(config.channels);
	let error_shared = Arc::clone(&shared);
	device
		.build_output_stream(
			config,
			move |output: &mut [T], _| {
				let mut drained = false;
				let frames = match shared.samples.try_lock() {
					Ok(mut queue) => {
						let frames = fill_output(output, &mut queue, channels);
						drained = source_exhausted(&queue, channels, shared.decoder_done.load(Ordering::Relaxed));
						frames
					}
					Err(_) => {
						// デコードが追いつかない間は無音（時計も進めない）
						output.iter_mut().for_each(|sample| *sample = T::from_sample(0.0));
						0
					}
				};
				shared.played.fetch_add(frames, Ordering::Relaxed);
				if drained {
					shared.ended.store(true, Ordering::Release);
				}
			},
			move |error| {
				log::warn!("audio stream error: {error}");
				error_shared.stop.store(true, Ordering::Relaxed);
				error_shared.ended.store(true, Ordering::Release);
			},
			None,
		)
		.context("opening the audio output")
}

/// Copy whole frames from `queue` into `output`, leaving silence for missing
/// frames. Returns the number of frames actually played.
fn fill_output<T>(output: &mut [T], queue: &mut VecDeque<f32>, channels: usize) -> u64
where
	T: Sample + FromSample<f32>,
{
	let mut frames = 0;
	for frame in output.chunks_mut(channels) {
		if queue.len() < frame.len() {
			frame.iter_mut().for_each(|sample| *sample = T::from_sample(0.0));
			continue;
		}
		for sample in frame.iter_mut() {
			*sample = T::from_sample(queue.pop_front().unwrap_or(0.0));
		}
		frames += 1;
	}
	frames
}

/// Whether the decoder is done and the queue can no longer supply a full frame.
fn source_exhausted(queue: &VecDeque<f32>, channels: usize, decoder_done: bool) -> bool {
	decoder_done && queue.len() < channels
}

#[cfg(test)]
mod tests {
	use super::*;

	fn shared(rate: u32, channels: u16) -> Arc<Shared> {
		Arc::new(Shared {
			samples: Mutex::new(VecDeque::new()),
			played: AtomicU64::new(0),
			stop: AtomicBool::new(false),
			decoder_done: AtomicBool::new(false),
			ended: AtomicBool::new(false),
			rate,
			channels,
		})
	}

	fn preview(shared: Arc<Shared>) -> AudioPreview {
		let (stop_tx, _stop_rx) = mpsc::channel();
		AudioPreview {
			playback: Some(Playback {
				from: 2.0,
				shared,
				stop_tx,
				output: None,
			}),
		}
	}

	fn samples(values: &[f32]) -> VecDeque<f32> {
		values.iter().copied().collect()
	}

	#[test]
	fn clock_waits_for_the_first_frame_then_advances() {
		let shared = shared(48_000, 2);
		let preview = preview(Arc::clone(&shared));
		assert_eq!(preview.clock(), None);
		shared.played.store(48_000, Ordering::Relaxed);
		assert_eq!(preview.clock(), Some(3.0));
	}

	#[test]
	fn ended_playback_is_reported() {
		let shared = shared(48_000, 2);
		let preview = preview(Arc::clone(&shared));
		assert!(!preview.has_ended());
		shared.ended.store(true, Ordering::Relaxed);
		assert!(preview.has_ended());
	}

	#[test]
	fn dropping_a_preview_stops_the_stream() {
		let shared = shared(48_000, 2);
		let preview = preview(Arc::clone(&shared));
		drop(preview);
		assert!(shared.stop.load(Ordering::Relaxed));
	}

	#[test]
	fn underrun_keeps_a_partial_frame_and_counts_only_played_frames() {
		let mut queue = samples(&[0.1, 0.2, 0.3]);
		let mut output = [9.0f32; 4];
		assert_eq!(fill_output(&mut output, &mut queue, 2), 1);
		assert_eq!(output, [0.1, 0.2, 0.0, 0.0]);
		assert_eq!(queue, samples(&[0.3]));
	}

	#[test]
	fn empty_queue_is_silent() {
		let mut queue = VecDeque::new();
		let mut output = [1.0f32; 4];
		assert_eq!(fill_output(&mut output, &mut queue, 2), 0);
		assert_eq!(output, [0.0; 4]);
	}

	#[test]
	fn exhaustion_needs_a_finished_decoder_and_a_partial_frame() {
		assert!(!source_exhausted(&samples(&[]), 2, false));
		assert!(source_exhausted(&samples(&[]), 2, true));
		assert!(source_exhausted(&samples(&[0.5]), 2, true));
		assert!(!source_exhausted(&samples(&[0.5, 0.5]), 2, true));
	}
}
