//! # Core Loop
//!
//! nade_core のロジックループを実行するモジュールです。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender, unbounded};
use nade_core::{CoreEffect, FrameData, Model, Msg, update};

use crate::renderer;

/// レンダリング完了時のコールバックメッセージ
struct RenderResult {
	frame_data: FrameData,
}

/// Coreロジックのメインループ
///
/// メッセージを受信し、ロジックを更新し、エフェクトを実行してUIに状態を送信します。
/// レンダリングは別スレッドで非同期に実行され、UIの応答性を維持します。
pub fn core_loop(rx: Receiver<Msg>, tx: Sender<Model>) {
	let mut model = Model::default();
	// 初期状態送信
	tx.send(model.clone()).ok();

	// レンダリング結果を受け取るチャネル
	let (render_tx, render_rx) = unbounded::<RenderResult>();

	// レンダリング中かどうかを追跡（フレームドロップ用）
	let is_rendering = Arc::new(AtomicBool::new(false));

	// FPS計算用
	let mut frame_count = 0u32;
	let mut fps_update_time = Instant::now();

	// UI更新のレートリミット用
	let mut last_ui_update = Instant::now();
	let mut needs_update = true; // 初期状態を確実に送信
	let min_update_interval = std::time::Duration::from_millis(16); // ~60 FPS

	loop {
		// 非ブロッキングでレンダリング結果をチェック
		while let Ok(result) = render_rx.try_recv() {
			is_rendering.store(false, Ordering::SeqCst);
			model.preview.frame = Some(result.frame_data);
			needs_update = true;

			// FPS計算
			frame_count += 1;
			let now = Instant::now();
			let elapsed = now.duration_since(fps_update_time).as_secs_f32();
			if elapsed >= 0.5 {
				model.preview.fps = frame_count as f32 / elapsed;
				frame_count = 0;
				fps_update_time = now;
			}
		}

		// メッセージ待機（タイムアウト付きで応答性を維持）
		let msg = match rx.recv_timeout(std::time::Duration::from_millis(1)) {
			Ok(m) => Some(m),
			Err(crossbeam_channel::RecvTimeoutError::Timeout) => None,
			Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
		};

		if let Some(Msg::Shutdown) = msg {
			break;
		}

		// メッセージがあればロジック更新
		if let Some(msg) = msg {
			// ロジック更新 (Pure)
			let (next_model, effects) = update(model, msg);
			model = next_model;
			needs_update = true;

			// Effect 実行 (Impure)
			for effect in effects {
				match effect {
					CoreEffect::RenderFrame {
						time,
						width,
						height,
					} => {
						// 既にレンダリング中ならスキップ（フレームドロップ）
						if is_rendering.load(Ordering::SeqCst) {
							log::trace!(
								"Skipping frame render - previous render still in progress"
							);
							continue;
						}

						is_rendering.store(true, Ordering::SeqCst);
						let render_tx = render_tx.clone();

						// 別スレッドでレンダリング実行
						thread::spawn(move || {
							let frame_num = (time * 60.0) as u32;
							let img_buffer = renderer::render_frame(frame_num, width, height);
							let raw = img_buffer.into_raw();

							let frame_data = FrameData {
								width,
								height,
								pixels: bytes::Bytes::from(raw),
							};

							render_tx.send(RenderResult { frame_data }).ok();
						});
					}
				}
			}
		}

		// UIへの更新送信（レートリミット）
		// 更新があり、かつ間隔要件を満たしている場合のみ送信
		let now = Instant::now();
		if needs_update && now.duration_since(last_ui_update) >= min_update_interval {
			tx.send(model.clone()).ok();
			last_ui_update = now;
			needs_update = false;
		}
	}
}
