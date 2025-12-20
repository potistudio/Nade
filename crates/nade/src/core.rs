//! # Core Loop
//!
//! nade_core のロジックループを実行するモジュールです。

use crossbeam_channel::{Receiver, Sender};
use nade_core::{CoreEffect, FrameData, Model, Msg, update};

use crate::renderer;

/// Coreロジックのメインループ
///
/// メッセージを受信し、ロジックを更新し、エフェクトを実行してUIに状態を送信します。
pub fn core_loop(rx: Receiver<Msg>, tx: Sender<Model>) {
	let mut model = Model::default();
	// 初期状態送信
	tx.send(model.clone()).ok();

	loop {
		// メッセージ待機
		let msg = match rx.recv() {
			Ok(m) => m,
			Err(_) => break, // Sender drops
		};

		if let Msg::Shutdown = msg {
			break;
		}

		// ロジック更新 (Pure)
		let (next_model, effects) = update(model, msg);
		model = next_model;

		// Effect 実行 (Impure)
		for effect in effects {
			match effect {
				CoreEffect::RenderFrame {
					time,
					width,
					height,
				} => {
					// レンダリング実行
					// ここで renderer::render_frame を呼ぶ
					// フレーム番号換算
					let frame_num = (time * 60.0) as u32;
					let img_buffer = renderer::render_frame(frame_num, width, height);
					let raw = img_buffer.into_raw();
					// Arc化してFrameData作成
					let frame_data = FrameData {
						width,
						height,
						pixels: bytes::Bytes::from(raw),
					};

					// レンダリング結果をメッセージとして自分自身(Core Logic)に戻すか、
					// 直接 Model に反映して送るか。
					// update関数は FrameRendered メッセージを受け付けるので、
					// ここで再帰的に update を呼ぶか、次のループで処理するか。
					// メッセージとして投げ直すのが本来の Elm Architecture だが、
					// チャンネル経由だと非同期になる。
					// synchronous に反映したいならここで update を呼ぶ。

					// Simple approach: Apply directly to model for now
					model.preview.frame = Some(frame_data);
				}
			}
		}

		// 更新された状態をUIへ送信
		tx.send(model.clone()).ok();
	}
}
