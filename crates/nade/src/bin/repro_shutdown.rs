//! Bug:

use iced::futures::{SinkExt, StreamExt};
use iced::widget::{column, text};
use iced::{Element, Subscription, Task, time};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task;

pub fn main() -> iced::Result {
	iced::application(Repro::new, Repro::update, Repro::view)
		.subscription(Repro::subscription)
		.run()
}

struct Repro {
	rx: Arc<Mutex<crossbeam_channel::Receiver<()>>>,
	#[allow(dead_code)]
	tx: crossbeam_channel::Sender<()>,
	window_id: Option<iced::window::Id>,
}

#[derive(Debug, Clone)]
enum Message {
	Event(()),
	WindowOpened(iced::window::Id),
	WindowClosed(iced::window::Id),
	AutoClose,
}

#[derive(Clone)]
struct Connection(Arc<Mutex<crossbeam_channel::Receiver<()>>>);

impl Hash for Connection {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(Arc::as_ptr(&self.0) as usize).hash(state);
	}
}

impl PartialEq for Connection {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

impl Eq for Connection {}

fn build_stream(conn: &Connection) -> iced::futures::stream::BoxStream<'static, Message> {
	let rx = conn.0.lock().unwrap().clone();
	iced::stream::channel(
		100,
		move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
			loop {
				let rx = rx.clone();
				// Simulate blocking wait
				let result = task::spawn_blocking(move || rx.recv()).await;

				match result {
					Ok(Ok(_)) => {
						output.send(Message::Event(())).await.ok();
					}
					Ok(Err(_)) => {
						println!("Stream: Channel disconnected");
						break;
					}
					Err(_) => {
						break;
					}
				}
			}
		},
	)
	.boxed()
}

impl Repro {
	fn new() -> (Self, Task<Message>) {
		let (tx, rx) = crossbeam_channel::unbounded();
		(
			Self {
				rx: Arc::new(Mutex::new(rx)),
				tx,
				window_id: None,
			},
			Task::none(),
		)
	}

	fn update(&mut self, message: Message) -> Task<Message> {
		match message {
			Message::Event(_) => Task::none(),
			Message::WindowOpened(id) => {
				self.window_id = Some(id);
				Task::none()
			}
			Message::AutoClose => {
				if let Some(id) = self.window_id {
					println!("Auto-closing window...");
					iced::window::close(id)
				} else {
					Task::none()
				}
			}
			Message::WindowClosed(id) => {
				println!("App: WindowClosed received. Window ID: {:?}", id);
				// SIMULATE BUG: Do not drop tx. Do not send shutdown.
				// Just close window.
				iced::window::close(id)
			}
		}
	}

	fn view(&self) -> Element<Message> {
		column![text("Auto-closing in 1 second..."),].into()
	}

	fn subscription(&self) -> Subscription<Message> {
		let conn = Connection(self.rx.clone());
		let core_subscription = Subscription::run_with(conn, build_stream);

		let window_subscription = iced::event::listen_with(|event, _status, id| match event {
			iced::Event::Window(iced::window::Event::Opened { .. }) => {
				Some(Message::WindowOpened(id))
			}
			iced::Event::Window(iced::window::Event::CloseRequested) => {
				Some(Message::WindowClosed(id))
			}
			_ => None,
		});

		let auto_close = time::every(Duration::from_secs(1)).map(|_| Message::AutoClose);

		Subscription::batch([core_subscription, window_subscription, auto_close])
	}
}
