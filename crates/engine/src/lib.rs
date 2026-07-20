pub mod clock;
pub mod state;

use self::{clock::MasterClock, state::EngineState};
use std::sync::Arc;

/// UIから操作されるメインのエンジンスターター
pub struct AudioEngine {
	state: Arc<EngineState>,
	// Clockは保持し続ける必要がある（ドロップするとスレッドが止まる実装にする場合などに備えて）
	_clock: MasterClock,
}

impl AudioEngine {
	pub fn new(sample_rate: u32) -> Self {
		let state = Arc::new(EngineState::new(sample_rate));
		let clock = MasterClock::spawn(state.clone());

		Self { state, _clock: clock }
	}

	// --- UI向けの操作インターフェース ---

	pub fn play(&self) {
		log::info!("Engine: Play");
		self.state.set_playing(true);
	}

	pub fn pause(&self) {
		log::info!("Engine: Pause");
		self.state.set_playing(false);
	}

	pub fn stop(&self) {
		log::info!("Engine: Stop");
		self.state.set_playing(false);
		self.state.set_current_ticks(0);
	}

	pub fn is_playing(&self) -> bool {
		self.state.is_playing()
	}

	/// 現在の状態（秒数など）を取得するヘルパー
	/// UI描画のために必要な情報をここで整形して返す
	pub fn current_status(&self) -> PlaybackStatus {
		PlaybackStatus {
			seconds: self.state.current_seconds(),
			ticks: self.state.current_ticks(),
			sample_rate: self.state.sample_rate(),
		}
	}
}

/// UIに渡すための読み取り専用ステータス
#[derive(Debug, Clone, Copy)]
pub struct PlaybackStatus {
	pub seconds: f64,
	pub ticks: u64,
	pub sample_rate: u32,
}
