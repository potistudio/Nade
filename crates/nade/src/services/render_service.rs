use crate::message::Message;
use core::FrameData;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use std::thread;

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
		32,
		move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
			let (async_tx, mut async_rx) = tokio::sync::mpsc::channel::<Option<FrameData>>(32);

			thread::Builder::new()
				.name("nade-render-bridge".into())
				.spawn(move || {
					loop {
						crossbeam_channel::select! {
							recv(rx) -> msg => {
								match msg {
									Ok(frame) => {
										if async_tx.blocking_send(Some(frame)).is_err() {
											break;
										}
									}
									Err(_) => break,
								}
							}
							recv(shutdown_rx) -> _ => break,
						}
					}
					let _ = async_tx.blocking_send(None);
				})
				.expect("failed to spawn render bridge thread");

			while let Some(msg) = async_rx.recv().await {
				let Some(frame) = msg else { break };
				if output.send(Message::RenderCompleted(frame)).await.is_err() {
					break;
				}
			}
		},
	)
	.boxed()
}
