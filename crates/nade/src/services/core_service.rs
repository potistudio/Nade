use crate::message::Message;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use core::Model;
use std::sync::{Arc, Mutex};

/// Coreへの接続状態（Subscription用）
#[derive(Clone)]
pub struct CoreConnection(
	pub Arc<Mutex<crossbeam_channel::Receiver<Model>>>,
	pub Arc<Mutex<crossbeam_channel::Receiver<()>>>,
);

// Mutexで包むことで、自動的に Sync が実装されるため unsafe は不要
// ロック競合が発生するのは「Subscription生成時の一瞬」だけで、
// メッセージ受信ループ(60fps)には一切影響しません。

impl std::hash::Hash for CoreConnection {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(Arc::as_ptr(&self.0) as usize).hash(state);
		(Arc::as_ptr(&self.1) as usize).hash(state);
	}
}

impl PartialEq for CoreConnection {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0) && Arc::ptr_eq(&self.1, &other.1)
	}
}

impl Eq for CoreConnection {}

/// Coreストリームの構築
///
/// CoreからのModel更新を監視し、Message::CoreUpdatedとして発行します。
pub fn build_core_stream(conn: &CoreConnection) -> BoxStream<'static, Message> {
	// 両方のReceiverをロックしてクローン
	let rx = conn.0.lock().unwrap().clone();
	let shutdown_rx = conn.1.lock().unwrap().clone();

	iced::stream::channel(
		100,
		move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
			loop {
				// select! を使用したブロッキング待機
				let task_result = tokio::task::spawn_blocking({
					let rx = rx.clone();
					let shutdown_rx = shutdown_rx.clone();
					move || {
						crossbeam_channel::select! {
							recv(rx) -> msg => Some(msg),
							recv(shutdown_rx) -> _ => None, // シャットダウン信号
						}
					}
				})
				.await;

				match task_result {
					Ok(Some(Ok(model))) => {
						// Model受信成功
						if output.send(Message::CoreUpdated(model)).await.is_err() {
							log::info!("Service: Iced channel closed.");
							break;
						}
					}
					Ok(Some(Err(_))) => {
						log::info!("Service: Core channel disconnected.");
						break;
					}
					Ok(None) => {
						log::info!("Service: Shutdown signal received.");
						break;
					}
					Err(_) => {
						log::info!("Service: Task join error.");
						break;
					}
				}
			}
		},
	)
	.boxed()
}
