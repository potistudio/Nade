//! # Nade - Entry Point
//!
//! Nadeアプリケーションのエントリーポイント。

mod app;
mod composition;
mod core;
mod encoder;
mod message;
mod panel_content;
mod panels;
mod renderer;
mod services;
mod theme;
mod widgets;

use std::thread;

use crossbeam_channel::unbounded;
use nade_core::{Model, Msg};

/// Entry point of the application (desktop)
pub fn main() -> iced::Result {
	// TODO: 環境変数の設定を抽出する
	// NOTE: 複雑化した際に検討する
	#[allow(unsafe_code)]
	unsafe {
		if std::env::var("RUST_LOG").is_err() {
			#[cfg(debug_assertions)]
			std::env::set_var("RUST_LOG", "nade=debug");

			#[cfg(not(debug_assertions))]
			std::env::set_var("RUST_LOG", "nade=info");
		}

		std::env::set_var("ICED_PRESENT_MODE", "immediate");
	}

	env_logger::init();

	// TODO: bounded channel 検討（優先度：低・将来）
	// 現状unboundedで問題ないが、レンダリングが重くなり
	// メモリ消費が問題になった場合は以下を検討：
	// - bounded(2) でbackpressure
	// - tokio::sync::watchで最新のみ保持
	// - カスタムLatestModel構造体
	// 計測して問題が出てから対応する。

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

	log::info!("Waiting for shutdown...");
	shutdown_tx.send(Msg::Shutdown).ok();

	// Wait for the core thread to finish
	log::info!("Waiting for core thread to finish...");
	if let Err(e) = core_handle.join() {
		log::error!("Core thread joined with error: {:?}", e);
	} else {
		log::info!("Core thread shutdown successfully.");
	}

	log::info!("Shutdown completed.");
	ui_result
}
