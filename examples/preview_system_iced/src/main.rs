mod preview_canvas;
mod properties_panel;

use iced::widget::{container, row};
use iced::{Element, Length, Subscription, Task, Theme};
use preview_canvas::PreviewCanvas;
use properties_panel::{PropertiesPanel, PropertiesPanelMessage};
use std::time::{Duration, Instant};

fn main() -> iced::Result {
	env_logger::init();
	log::debug!("Starting Preview System Demo (Iced) - Realtime Pixel Graphics");

	iced::application("Realtime Pixel Graphics Demo", App::update, App::view)
		.theme(|_| Theme::Dark)
		.window_size((1200.0, 700.0))
		.subscription(App::subscription)
		.run_with(App::new)
}

struct App {
	preview_canvas: PreviewCanvas,
	properties_panel: PropertiesPanel,
}

#[derive(Debug, Clone)]
enum Message {
	Tick,
	PropertiesPanel(PropertiesPanelMessage),
}

impl App {
	fn new() -> (Self, Task<Message>) {
		let preview_canvas = PreviewCanvas::new();
		let properties_panel = PropertiesPanel::new(preview_canvas.get_params().clone());

		(
			Self {
				preview_canvas,
				properties_panel,
			},
			Task::none(),
		)
	}

	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Tick => {
				self.preview_canvas.tick();
			}
			Message::PropertiesPanel(msg) => {
				self.properties_panel.update(msg.clone());

				// プロパティパネルからの変更をキャンバスに適用
				match msg {
					PropertiesPanelMessage::TimeScaleChanged(v) => {
						self.preview_canvas.set_time_scale(v);
					}
					PropertiesPanelMessage::Color1RChanged(v) => {
						let mut color = self.properties_panel.get_color1();
						color.r = v;
						self.preview_canvas.set_color1(color);
					}
					PropertiesPanelMessage::Color1GChanged(v) => {
						let mut color = self.properties_panel.get_color1();
						color.g = v;
						self.preview_canvas.set_color1(color);
					}
					PropertiesPanelMessage::Color1BChanged(v) => {
						let mut color = self.properties_panel.get_color1();
						color.b = v;
						self.preview_canvas.set_color1(color);
					}
					PropertiesPanelMessage::Color2RChanged(v) => {
						let mut color = self.properties_panel.get_color2();
						color.r = v;
						self.preview_canvas.set_color2(color);
					}
					PropertiesPanelMessage::Color2GChanged(v) => {
						let mut color = self.properties_panel.get_color2();
						color.g = v;
						self.preview_canvas.set_color2(color);
					}
					PropertiesPanelMessage::Color2BChanged(v) => {
						let mut color = self.properties_panel.get_color2();
						color.b = v;
						self.preview_canvas.set_color2(color);
					}
					PropertiesPanelMessage::EffectTypeChanged(effect) => {
						self.preview_canvas.set_effect_type(effect);
					}
					PropertiesPanelMessage::Param1Changed(v) => {
						self.preview_canvas.set_param1(v);
					}
					PropertiesPanelMessage::Param2Changed(v) => {
						self.preview_canvas.set_param2(v);
					}
					PropertiesPanelMessage::Param3Changed(v) => {
						self.preview_canvas.set_param3(v);
					}
				}
			}
		}
		Task::none()
	}

	fn view(&self) -> Element<'_, Message> {
		let preview = container(self.preview_canvas.view().map(|_| Message::Tick))
			.width(Length::FillPortion(3))
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(iced::Color::from_rgb(0.08, 0.08, 0.08).into()),
				..Default::default()
			});

		let properties = container(self.properties_panel.view().map(Message::PropertiesPanel))
			.width(Length::FillPortion(1))
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(iced::Color::from_rgb(0.12, 0.12, 0.12).into()),
				..Default::default()
			});

		let content = row![preview, properties]
			.spacing(1)
			.width(Length::Fill)
			.height(Length::Fill);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn subscription(&self) -> Subscription<Message> {
		// 60 FPS でティック
		iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
	}
}
