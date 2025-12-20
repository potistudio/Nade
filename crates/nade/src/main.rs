//! # Nade - Entry Point
//!
//! Nadeアプリケーションのエントリーポイント。

mod app;
mod composition;
mod core;
mod encoder;
mod message;
mod panel_content;
mod renderer;

use std::thread;

use crossbeam_channel::unbounded;
use nade_core::{Model, Msg};

/// Entry point of the application (desktop)
pub fn main() -> iced::Result {
	#[allow(unsafe_code)]
	unsafe {
		//FIXME: RUST_LOGをアプリケーションから分離する
		std::env::set_var("RUST_LOG", "debug");
		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}

	// TODO: debug_assertions でログレベル分岐（優先度：中）

	// TODO: bounded channel 検討（優先度：低・将来）
	// 現状は問題ないが、Coreのレンダリングが重くなってUIが追いつかない場合、
	// unboundedだとメモリを消費し続ける。将来的にはboundedにしてbackpressureをかけるか、
	// 最新のModelだけ保持する仕組みを検討する。
	env_logger::init();

	// Create channels for communication between UI and core logic
	let (ui_tx, core_rx) = unbounded::<Msg>();
	let (core_tx, ui_rx) = unbounded::<Model>();

	// Shutdown用にSenderを保持
	// NOTE: NadeApp内のSenderはicedがdropするタイミングが不定のため、
	// チャネルcloseに依存せず明示的にMsg::Shutdownを送る方式を採用
	let shutdown_tx = ui_tx.clone();

	// Start core logic in a separate thread
	let core_handle = thread::spawn(move || {
		core::core_loop(core_rx, core_tx);
	});

	// Start UI in the main thread
	let ui_result = app::run_ui(ui_tx, ui_rx);

	// TODO: Graceful Shutdown改善（優先度：低）
	// 現在Subscription内でrecv_timeout(100ms)を使用してシャットダウンを検知している。
	// これは実質ポーリングであり、以下の改善案がある:
	// - shutdown専用channelを追加し、futures::select!で両方を監視する
	// - tokio::sync::watch等のbroadcast channelを使用する
	// ただし現状で実用上問題ないため、複雑化を避けて保留。
	log::debug!("UI finished, sending Shutdown...");
	shutdown_tx.send(Msg::Shutdown).ok();

	// Wait for the core thread to finish
	log::debug!("Waiting for core thread to finish...");
	if let Err(e) = core_handle.join() {
		log::error!("Core thread joined with error: {:?}", e);
	} else {
		log::info!("Core thread shutdown successfully.");
	}

	log::debug!("Shutdown completed.");
	ui_result
}
