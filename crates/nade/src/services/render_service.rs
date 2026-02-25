use crate::message::Message;
use core::FrameData;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};

/// レンダリング結果の接続状態（Subscription用）
#[derive(Clone)]
pub struct RenderConnection(
	pub Arc<Mutex<crossbeam_channel::Receiver<FrameData>>>,
	pub Arc<Mutex<crossbeam_channel::Receiver<()>>>,
);

impl std::hash::Hash for RenderConnection {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(Arc::as_ptr(&self.0) as usize).hash(state);
		(Arc::as_ptr(&self.1) as usize).hash(state);
	}
}

impl PartialEq for RenderConnection {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0) && Arc::ptr_eq(&self.1, &other.1)
	}
}

impl Eq for RenderConnection {}

/// レンダリング結果ストリームの構築
pub fn build_render_stream(conn: &RenderConnection) -> BoxStream<'static, Message> {
	let rx = conn.0.lock().unwrap().clone();
	let shutdown_rx = conn.1.lock().unwrap().clone();

	iced::stream::channel(
		100,
		move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
			loop {
				let task_result = tokio::task::spawn_blocking({
					let rx = rx.clone();
					let shutdown_rx = shutdown_rx.clone();
					move || {
						crossbeam_channel::select! {
							recv(rx) -> msg => Some(msg),
							recv(shutdown_rx) -> _ => None,
						}
					}
				})
				.await;

				match task_result {
					Ok(Some(Ok(frame))) => {
						if output.send(Message::RenderCompleted(frame)).await.is_err() {
							log::info!("RenderService: Iced channel closed.");
							break;
						}
					}
					Ok(Some(Err(_))) => {
						log::info!("RenderService: Render channel disconnected.");
						break;
					}
					Ok(None) => {
						log::info!("RenderService: Shutdown signal received.");
						break;
					}
					Err(_) => {
						log::info!("RenderService: Task join error.");
						break;
					}
				}
			}
		},
	)
	.boxed()
}
