//! タッチパッドのピンチジェスチャーをicedのSubscriptionとして提供する
//!
//! macOSでは`NSEvent`のローカルモニターを使ってウィンドウ内のマグニファイイベントを
//! 捕捉し、`futures::channel::mpsc`経由でicedのメッセージに変換する。

use std::sync::Mutex;

use iced::Subscription;
use iced::futures::SinkExt;
use iced::futures::StreamExt;
use iced::futures::channel::mpsc;

use crate::message::Message;
use timeline_panel::TimelineMessage;

static PINCH_SENDER: Mutex<Option<mpsc::UnboundedSender<f32>>> = Mutex::new(None);

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn nade_setup_pinch_monitor(callback: unsafe extern "C" fn(f64));
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn pinch_callback(delta: f64) {
    if let Ok(guard) = PINCH_SENDER.lock() {
        if let Some(sender) = guard.as_ref() {
            let _ = sender.unbounded_send(delta as f32);
        }
    }
}

#[cfg(target_os = "macos")]
fn pinch_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(10, async |mut output| {
        let (tx, mut rx) = mpsc::unbounded::<f32>();

        *PINCH_SENDER.lock().expect("pinch sender lock") = Some(tx);

        unsafe {
            nade_setup_pinch_monitor(pinch_callback);
        }

        while let Some(delta) = rx.next().await {
            let _ = output.send(Message::Timeline(TimelineMessage::PinchZoom(delta))).await;
        }
    })
}

pub fn pinch_subscription() -> Subscription<Message> {
    #[cfg(target_os = "macos")]
    return Subscription::run(pinch_stream);

    #[cfg(not(target_os = "macos"))]
    return Subscription::none();
}
