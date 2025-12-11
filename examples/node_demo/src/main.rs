mod node_graph;

use iced::{Element, Size, Subscription, Task, Theme, application, event, window};
use node_graph::{NodeGraph, NodeGraphMessage};

fn main() -> iced::Result {
	env_logger::init();

	application("Node Graph Demo", App::update, App::view)
		.theme(|_| Theme::Dark)
		.window_size((1400.0, 900.0))
		.subscription(App::subscription)
		.run_with(App::new)
}

struct App {
	node_graph: NodeGraph,
}

#[derive(Debug, Clone)]
enum Message {
	NodeGraph(NodeGraphMessage),
	WindowResized(Size),
}

impl App {
	fn new() -> (Self, Task<Message>) {
		let node_graph = NodeGraph::new();
		(Self { node_graph }, Task::none())
	}

	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::NodeGraph(msg) => {
				self.node_graph.update(msg);
			}
			Message::WindowResized(size) => {
				self.node_graph.set_size(size);
			}
		}
		Task::none()
	}

	fn view(&self) -> Element<'_, Message> {
		self.node_graph.view().map(Message::NodeGraph)
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
