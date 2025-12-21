use crate::message::Message;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use nade_core::Model;
use std::sync::{Arc, Mutex};

/// Coreへの接続状態（Subscription用）
#[derive(Clone)]
pub struct CoreConnection(pub Arc<Mutex<crossbeam_channel::Receiver<Model>>>);

// Mutexで包むことで、自動的に Sync が実装されるため unsafe は不要
// ロック競合が発生するのは「Subscription生成時の一瞬」だけで、
// メッセージ受信ループ(60fps)には一切影響しません。

impl std::hash::Hash for CoreConnection {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		(Arc::as_ptr(&self.0) as usize).hash(state);
	}
}

impl PartialEq for CoreConnection {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

impl Eq for CoreConnection {}

/// Coreストリームの構築
///
/// CoreからのModel更新を監視し、Message::CoreUpdatedとして発行します。
pub fn build_core_stream(conn: &CoreConnection) -> BoxStream<'static, Message> {
	// 【重要】ここで一度だけロックして、Receiverをクローンする
	// クローンされた rx はこのタスクの所有物になるため、以降ロックは不要
	// crossbeam_channel::Receiver は Clone 可能で、同じチャネルへの参照を持つ新しいハンドルを作成します。
	let rx = conn.0.lock().unwrap().clone();

	iced::stream::channel(
		100,
		move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
			loop {
				// ここから先はロックコスト・ゼロ
				let task_result = tokio::task::spawn_blocking({
					let rx = rx.clone();
					// ブロッキング待機 (recv)
					move || rx.recv()
				})
				.await;

				match task_result {
					Ok(Ok(model)) => {
						// Model受信成功
						if output.send(Message::CoreUpdated(model)).await.is_err() {
							// iced側がチャンネルを閉じた
							break;
						}
					}
					Ok(Err(_)) => {
						// チャンネル切断 (RecvError)
						break;
					}
					Err(_) => {
						// タスク実行エラー (JoinError)
						// シャットダウン時など
						break;
					}
				}
			}
		},
	)
	.boxed()
}
