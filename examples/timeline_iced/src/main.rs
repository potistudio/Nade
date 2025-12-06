mod timeline;

use iced::{Element, Task, Theme};
use timeline::TimelineWidget;

fn main() -> iced::Result {
	unsafe {
		std::env::set_var("RUST_LOG", "debug");
	}
	env_logger::init();

	log::debug!("Starting Timeline Demo (Iced)");

	iced::application("Timeline Demo", App::update, App::view)
		.theme(|_| Theme::Dark)
		.window_size((1200.0, 400.0))
		.run()
}

#[derive(Default)]
struct App {
	timeline_widget: TimelineWidget,
}

#[derive(Debug, Clone)]
enum Message {
	Timeline(timeline::Message),
}

impl App {
	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Timeline(msg) => {
				self.timeline_widget.update(msg);
			}
		}
		Task::none()
	}

	fn view(&self) -> Element<'_, Message> {
		self.timeline_widget.view().map(Message::Timeline)
	}
}
