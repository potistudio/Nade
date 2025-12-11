mod slider;

use iced::{
	Element, Length, Task, Theme,
	widget::{center, column, container, text},
};
use slider::{NumericSlider, NumericSliderMessage};

fn main() -> iced::Result {
	unsafe {
		std::env::set_var("RUST_LOG", "debug");
	}
	env_logger::init();

	log::debug!("Starting Custom Slider Demo (Iced)");

	iced::application("Custom Slider Demo", App::update, App::view)
		.theme(|_| Theme::Dracula)
		.window_size((400.0, 300.0))
		.run()
}

#[derive(Default)]
struct App {
	slider_value: f32,
}

#[derive(Debug, Clone)]
enum Message {
	Slider(NumericSliderMessage),
}

impl App {
	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Slider(slider::NumericSliderMessage::ValueChanged(value)) => {
				self.slider_value = value;
				log::debug!("Slider value: {}", value);
			}
		}
		Task::none()
	}

	fn view(&self) -> Element<'_, Message> {
		let title = text("Custom Slider Demo").size(24);

		let slider = NumericSlider::new(self.slider_value, 0.0..=100.0)
			.label("Value")
			.step(0.1);

		let content = column![
			title,
			container(slider.view().map(Message::Slider)).width(Length::Fixed(250.0)),
		]
		.spacing(30)
		.padding(40);

		center(content).into()
	}
}
