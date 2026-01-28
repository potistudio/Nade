use super::state::EngineState;
use constants::TICK_INTERVAL_MS;
use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	thread,
	time::{Duration, Instant},
};

/// Master clock for the engine
pub struct MasterClock {
	// Drop時に join() するために Option でラップする
	// (take() して所有権を奪う必要があるため)
	handle: Option<thread::JoinHandle<()>>,
	// スレッドを停止させるための共有フラグ
	is_available: Arc<AtomicBool>,
}

impl MasterClock {
	/// Spawns a new master clock thread
	pub fn spawn(state: Arc<EngineState>) -> Self {
		let is_available = Arc::new(AtomicBool::new(true));

		// スレッドに渡す用のフラグ（clone）
		let thread_is_available = is_available.clone();

		let handle = thread::spawn(move || {
			Self::run_loop(state, thread_is_available);
		});

		Self {
			handle: Some(handle),
			is_available: is_available,
		}
	}

	/// Runs the master clock loop
	fn run_loop(state: Arc<EngineState>, is_available: Arc<AtomicBool>) {
		let sample_rate = state.sample_rate() as f64;
		let sleep_duration = Duration::from_millis(TICK_INTERVAL_MS);
		let mut last_loop_time = Instant::now();

		while is_available.load(Ordering::Relaxed) {
			let now = Instant::now();
			let delta = now.duration_since(last_loop_time);

			if state.is_playing() {
				let delta_secs = delta.as_secs_f64();
				let advance_ticks = (delta_secs * sample_rate) as u64;

				if advance_ticks > 0 {
					state.add_ticks(advance_ticks);
				}
			}

			last_loop_time = now;
			thread::sleep(sleep_duration);
		}

		log::debug!("MasterClock thread exited cleanly.");
	}
}

impl Drop for MasterClock {
	/// Drops the master clock thread
	fn drop(&mut self) {
		log::info!("Dropping MasterClock...");

		// 1. Signal to stop the thread
		self.is_available.store(false, Ordering::Relaxed);

		// 2. Wait for the thread to finish (Join)
		// NOTE: if thread::sleep is used, it may block for up to TICK_INTERVAL_MS.
		// To ensure a clean shutdown, it is recommended to wait for the thread to finish.
		if let Some(handle) = self.handle.take() {
			match handle.join() {
				Ok(_) => log::info!("MasterClock thread joined successfully."),
				Err(e) => log::error!("MasterClock thread panicked: {:?}", e),
			}
		}
	}
}
