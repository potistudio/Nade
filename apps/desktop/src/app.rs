use constants::{LOG_INTERVAL_MS, SAMPLE_RATE};
use iced::{Element, Subscription, Task, widget, window};
use nade_engine::AudioEngine;

#[derive(Debug, Clone, Copy)]
pub enum Message {
	Play,
	Pause,
	Stop,
	Tick(std::time::Instant),
}

pub struct NadeApp {
	engine: AudioEngine,
	// ログ出力制御用
	last_logged_interval: u64,
}

impl NadeApp {
	pub fn new() -> Self {
		Self {
			engine: AudioEngine::new(SAMPLE_RATE),
			last_logged_interval: 0,
		}
	}

	pub fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Play => {
				self.engine.play();
			}
			Message::Pause => {
				self.engine.pause();
			}
			Message::Stop => {
				self.engine.stop();
				self.last_logged_interval = 0; // リセット
			}
			Message::Tick(_instant) => {
				// UI更新とログ出力の処理
				self.handle_tick_logic();
			}
		}
		Task::none()
	}

	fn handle_tick_logic(&mut self) {
		let status = self.engine.current_status();

		// ログ出力の判定ロジック
		// (ロジックは明確化のためここに切り出し)
		let ticks_per_log = (status.sample_rate as f64 * LOG_INTERVAL_MS as f64) as u64;

		if ticks_per_log > 0 {
			let current_interval = status.ticks / ticks_per_log;

			if current_interval > self.last_logged_interval {
				println!("⏱ Time: {:.1} sec ({} ticks)", status.seconds, status.ticks);
				self.last_logged_interval = current_interval;
			}
		}
	}

	pub fn view(&self) -> Element<'_, Message> {
		let is_playing = self.engine.is_playing();

		// ボタンエリア
		let controls = widget::row![
			widget::button(if is_playing { "Pause" } else { "Play" }).on_press(if is_playing {
				Message::Pause
			} else {
				Message::Play
			}),
			widget::button("Reset").on_press(Message::Stop)
		]
		.spacing(10);

		// ステータス表示エリア (デバッグ用に追加)
		let status = self.engine.current_status();
		let display = widget::column![
			widget::text(format!("Time: {:.2}s", status.seconds)).size(40),
			controls
		]
		.spacing(20)
		.padding(20);

		display.into()
	}

	pub fn subscription(&self) -> Subscription<Message> {
		if self.engine.is_playing() {
			window::frames().map(Message::Tick)
		} else {
			Subscription::none()
		}
	}
}
