use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// スレッド間で共有される再生エンジンの状態
#[derive(Debug)]
pub struct EngineState {
	playing: AtomicBool,
	current_ticks: AtomicU64,
	sample_rate: u32,
}

impl EngineState {
	pub fn new(sample_rate: u32) -> Self {
		Self {
			playing: AtomicBool::new(false),
			current_ticks: AtomicU64::new(0),
			sample_rate,
		}
	}

	/// Returns whether the engine is currently playing.
	pub fn is_playing(&self) -> bool {
		self.playing.load(Ordering::Relaxed)
	}

	/// Returns the current tick count
	pub fn current_ticks(&self) -> u64 {
		self.current_ticks.load(Ordering::Relaxed)
	}

	/// Returns the sample rate
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate
	}

	/// Returns the current playback time in seconds.
	pub fn current_seconds(&self) -> f64 {
		self.current_ticks() as f64 / self.sample_rate as f64
	}

	/// Sets the playing state to the specified value.
	pub fn set_playing(&self, playing: bool) {
		self.playing.store(playing, Ordering::Relaxed);
	}

	/// Sets the current tick count to the specified value.
	pub fn set_current_ticks(&self, ticks: u64) {
		self.current_ticks.store(ticks, Ordering::Relaxed);
	}

	/// Adds the specified number of ticks to the current tick count.
	pub fn add_ticks(&self, ticks: u64) {
		self.current_ticks.fetch_add(ticks, Ordering::Release);
	}
}

#[cfg(test)]
mod tests {
	use super::EngineState;

	fn approx_eq_f64(a: f64, b: f64) {
		assert!((a - b).abs() < 1e-12, "left: {a}, right: {b}");
	}

	#[test]
	fn new_state_has_expected_defaults() {
		let state = EngineState::new(48_000);

		assert!(!state.is_playing());
		assert_eq!(state.current_ticks(), 0);
		assert_eq!(state.sample_rate(), 48_000);
		approx_eq_f64(state.current_seconds(), 0.0);
	}

	#[test]
	fn set_playing_toggles_playback_flag() {
		let state = EngineState::new(44_100);

		state.set_playing(true);
		assert!(state.is_playing());

		state.set_playing(false);
		assert!(!state.is_playing());
	}

	#[test]
	fn tick_operations_update_time_consistently() {
		let state = EngineState::new(48_000);

		state.add_ticks(24_000);
		assert_eq!(state.current_ticks(), 24_000);
		approx_eq_f64(state.current_seconds(), 0.5);

		state.add_ticks(12_000);
		assert_eq!(state.current_ticks(), 36_000);
		approx_eq_f64(state.current_seconds(), 0.75);

		state.set_current_ticks(48_000);
		assert_eq!(state.current_ticks(), 48_000);
		approx_eq_f64(state.current_seconds(), 1.0);
	}
}
