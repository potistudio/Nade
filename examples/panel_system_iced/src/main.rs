mod panel_system;

use iced::{Element, Size, Subscription, Task, Theme, application, event, window};
use panel_system::{PanelSystem, PanelSystemMessage};

fn main() -> iced::Result {
	unsafe {
		std::env::set_var("LOG_LEVEL", "debug");
		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}
	env_logger::init();

	application("Panel System Demo", App::update, App::view)
		.theme(|_| Theme::Dark)
		.window_size((1200.0, 800.0))
		.subscription(App::subscription)
		.run_with(App::new)
}

struct App {
	panel_system: PanelSystem,
}

#[derive(Debug, Clone)]
enum Message {
	PanelSystem(PanelSystemMessage),
	WindowResized(Size),
}

impl App {
	fn new() -> (Self, Task<Message>) {
		let mut panel_system = PanelSystem::new();

		// サンプルパネルを追加
		panel_system.add_panel("Explorer", PanelContent::Explorer);
		panel_system.add_panel("Properties", PanelContent::Properties);
		panel_system.add_panel("Timeline", PanelContent::Timeline);
		panel_system.add_panel("Preview", PanelContent::Preview);
		panel_system.add_panel("Console", PanelContent::Console);
		panel_system.add_panel("Assets", PanelContent::Assets);

		// 初期ウィンドウサイズを設定
		panel_system.update(PanelSystemMessage::WindowResized(Size::new(1200.0, 800.0)));

		(Self { panel_system }, Task::none())
	}

	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::PanelSystem(msg) => {
				self.panel_system.update(msg);
			}
			Message::WindowResized(size) => {
				self.panel_system
					.update(PanelSystemMessage::WindowResized(size));
			}
		}
		Task::none()
	}

	fn view(&self) -> Element<'_, Message> {
		self.panel_system.view().map(Message::PanelSystem)
	}

	fn subscription(&self) -> Subscription<Message> {
		event::listen_with(|event, _status, _id| {
			if let iced::Event::Window(window::Event::Resized(size)) = event {
				Some(Message::WindowResized(size))
			} else {
				None
			}
		})
	}
}

// パネルコンテンツの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelContent {
	Explorer,
	Properties,
	Timeline,
	Preview,
	Console,
	Assets,
}

impl PanelContent {
	pub fn label(&self) -> &'static str {
		match self {
			Self::Explorer => "Explorer",
			Self::Properties => "Properties",
			Self::Timeline => "Timeline",
			Self::Preview => "Preview",
			Self::Console => "Console",
			Self::Assets => "Assets",
		}
	}

	pub fn icon(&self) -> &'static str {
		match self {
			Self::Explorer => "📁",
			Self::Properties => "⚙️",
			Self::Timeline => "🎬",
			Self::Preview => "👁",
			Self::Console => "💻",
			Self::Assets => "📦",
		}
	}
}
