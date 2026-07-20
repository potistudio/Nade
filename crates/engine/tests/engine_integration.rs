use constants::TICK_INTERVAL_MS;
use nade_engine::{AudioEngine, clock::MasterClock, state::EngineState};
use std::{
	sync::Arc,
	thread,
	time::{Duration, Instant},
};

fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
	let start = Instant::now();
	while start.elapsed() < timeout {
		if condition() {
			return true;
		}
		thread::sleep(Duration::from_millis(5));
	}
	condition()
}

/// f64 の近似比較を行う。誤差が 1e-9 未満であれば等しいとみなす。
fn approx_eq_f64(a: f64, b: f64) {
	assert!((a - b).abs() < 1e-9, "left: {a}, right: {b}");
}

#[test]
/// Verify that a newly created AudioEngine starts in a stopped state with zero time.
fn audio_engine_starts_stopped_with_zero_time() {
	let engine = AudioEngine::new(1_000);
	let status = engine.current_status();

	assert!(!engine.is_playing());
	assert_eq!(status.sample_rate, 1_000);
	assert_eq!(status.ticks, 0);
	approx_eq_f64(status.seconds, 0.0);
}

#[test]
fn audio_engine_play_pause_stop_lifecycle() {
	let engine = AudioEngine::new(1_000);

	engine.play();
	assert!(engine.is_playing());
	assert!(
		wait_until(Duration::from_millis(300), || { engine.current_status().ticks > 0 }),
		"ticks did not advance while playing"
	);

	let playing_status = engine.current_status();
	assert!(playing_status.seconds > 0.0);

	engine.pause();
	assert!(!engine.is_playing());

	// Pause後は tick が安定して増えなくなることを確認する。
	thread::sleep(Duration::from_millis(TICK_INTERVAL_MS * 4));
	let paused_a = engine.current_status().ticks;
	thread::sleep(Duration::from_millis(TICK_INTERVAL_MS * 4));
	let paused_b = engine.current_status().ticks;
	assert_eq!(paused_a, paused_b, "ticks should not advance while paused");

	engine.stop();
	let stopped = engine.current_status();
	assert!(!engine.is_playing());
	assert_eq!(stopped.ticks, 0);
	approx_eq_f64(stopped.seconds, 0.0);
}

#[test]
fn master_clock_does_not_advance_when_not_playing() {
	let state = Arc::new(EngineState::new(1_000));
	let _clock = MasterClock::spawn(state.clone());

	thread::sleep(Duration::from_millis(TICK_INTERVAL_MS * 5));

	assert_eq!(state.current_ticks(), 0);
	approx_eq_f64(state.current_seconds(), 0.0);
}

#[test]
fn master_clock_advances_when_playing() {
	let state = Arc::new(EngineState::new(1_000));
	let _clock = MasterClock::spawn(state.clone());

	state.set_playing(true);
	assert!(
		wait_until(Duration::from_millis(300), || state.current_ticks() > 0),
		"ticks did not advance while master clock was playing"
	);

	assert!(state.current_ticks() > 0);
	assert!(state.current_seconds() > 0.0);
}
