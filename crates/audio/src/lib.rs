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
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
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

struct Shared {
	samples: Mutex<VecDeque<f32>>,
	/// Frames (samples per channel) played so far.
	played: AtomicU64,
	stop: AtomicBool,
	rate: u32,
	channels: u16,
}

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

/// Plays composition sound while the preview is running.
#[derive(Default)]
pub struct AudioPreview {
	playback: Option<Playback>,
}

impl AudioPreview {
	pub fn new() -> Self {
		Self::default()
	}

	/// Start playing `clips` from timeline time `from` until `until`
	/// (replacing any current playback). Returns `false` when there is nothing
	/// to hear or no output device.
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

	pub fn stop(&mut self) {
		self.playback = None;
	}

	pub fn is_playing(&self) -> bool {
		self.playback.is_some()
	}

	/// Timeline time of the sound currently heard, once samples flow.
	pub fn clock(&self) -> Option<f32> {
		let playback = self.playback.as_ref()?;
		let played = playback.shared.played.load(Ordering::Relaxed);
		(played > 0).then(|| playback.from + played as f32 / playback.shared.rate as f32)
	}
}

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
		samples: Mutex::new(VecDeque::new()),
		played: AtomicU64::new(0),
		stop: AtomicBool::new(false),
		rate,
		channels,
	});

	let child = Command::new(nade_media::ffmpeg_path())
		.args(&args)
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::null())
		.spawn()
		.with_context(|| format!("starting {}", nade_media::ffmpeg_path().display()))?;
	spawn_decoder(child, Arc::clone(&shared))?;

	// cpal のストリームはスレッド間で移動できないため、専用スレッドで保持する
	let (stop_tx, stop_rx) = mpsc::channel();
	let (ready_tx, ready_rx) = mpsc::channel();
	let stream_shared = Arc::clone(&shared);
	let output = thread::Builder::new()
		.name("nade-audio-out".into())
		.spawn(move || run_output(device, config, format, stream_shared, stop_rx, ready_tx))
		.context("starting the audio thread")?;
	ready_rx
		.recv_timeout(Duration::from_secs(2))
		.map_err(|_| anyhow!("audio output did not start"))??;

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

/// Read samples from ffmpeg into the shared queue, keeping it short.
fn spawn_decoder(mut child: Child, shared: Arc<Shared>) -> Result<()> {
	let mut stdout = child.stdout.take().context("ffmpeg stdout")?;
	let limit = (BUFFER_SECS * shared.rate as f32) as usize * usize::from(shared.channels);
	thread::Builder::new()
		.name("nade-audio-decode".into())
		.spawn(move || {
			let mut bytes = vec![0u8; 4096 * 4];
			let mut carry: Vec<u8> = Vec::new();
			while !shared.stop.load(Ordering::Relaxed) {
				let queued = shared.samples.lock().map(|queue| queue.len()).unwrap_or(0);
				if queued > limit {
					thread::sleep(Duration::from_millis(5));
					continue;
				}
				let read = match stdout.read(&mut bytes) {
					Ok(0) | Err(_) => break,
					Ok(read) => read,
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
			let _ = child.kill();
			let _ = child.wait();
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
	ready_tx: Sender<Result<()>>,
) {
	let stream = match format {
		SampleFormat::F32 => build::<f32>(&device, config, Arc::clone(&shared)),
		SampleFormat::I16 => build::<i16>(&device, config, Arc::clone(&shared)),
		SampleFormat::U16 => build::<u16>(&device, config, Arc::clone(&shared)),
		SampleFormat::I32 => build::<i32>(&device, config, Arc::clone(&shared)),
		SampleFormat::F64 => build::<f64>(&device, config, Arc::clone(&shared)),
		other => Err(anyhow!("unsupported sample format {other}")),
	};
	let stream = match stream.and_then(|stream| stream.play().map(|()| stream).context("starting playback")) {
		Ok(stream) => {
			let _ = ready_tx.send(Ok(()));
			stream
		}
		Err(error) => {
			let _ = ready_tx.send(Err(error));
			return;
		}
	};
	let _ = stop_rx.recv();
	drop(stream);
}

fn build<T>(device: &cpal::Device, config: StreamConfig, shared: Arc<Shared>) -> Result<cpal::Stream>
where
	T: SizedSample + FromSample<f32>,
{
	let channels = usize::from(config.channels);
	device
		.build_output_stream(
			config,
			move |output: &mut [T], _| {
				let mut frames = 0u64;
				if let Ok(mut queue) = shared.samples.try_lock() {
					for frame in output.chunks_mut(channels) {
						if queue.len() < frame.len() {
							// デコードが追いつかない間は無音（時計も進めない）
							frame.iter_mut().for_each(|sample| *sample = T::from_sample(0.0));
							continue;
						}
						for sample in frame.iter_mut() {
							*sample = T::from_sample(queue.pop_front().unwrap_or(0.0));
						}
						frames += 1;
					}
				} else {
					output.iter_mut().for_each(|sample| *sample = T::from_sample(0.0));
				}
				shared.played.fetch_add(frames, Ordering::Relaxed);
			},
			|error| log::warn!("audio stream error: {error}"),
			None,
		)
		.context("opening the audio output")
}
