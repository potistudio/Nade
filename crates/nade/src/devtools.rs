//! Debug-only helpers for UI work.
//!
//! `NADE_SCREENSHOT=out.png` saves the window as a PNG once the UI settles
//! (`NADE_SCREENSHOT_DELAY_MS`, default 2500) and quits — handy for PR
//! screenshots. `NADE_SCREENSHOT_SCENE` stages the UI first, e.g.
//! `select=Title;workspace=graph;tool=t;time=1.5;loop=1;play=1;menu=2`.

use std::path::PathBuf;

use iced::Task;
use iced::time::{Duration, Instant};
use iced::window::Screenshot;

use crate::message::{AppPanelMessage, Message, ShellMessage};
use crate::tool::Tool;
use crate::workspace::Workspace;

#[derive(Debug)]
pub struct Capture {
	path: PathBuf,
	due: Instant,
	requested: bool,
}

impl Capture {
	pub fn from_env() -> Option<Self> {
		let path = std::env::var_os("NADE_SCREENSHOT")?;
		let delay = std::env::var("NADE_SCREENSHOT_DELAY_MS")
			.ok()
			.and_then(|value| value.parse().ok())
			.unwrap_or(2500);
		Some(Self {
			path: PathBuf::from(path),
			due: Instant::now() + Duration::from_millis(delay),
			requested: false,
		})
	}

	pub fn is_pending(&self) -> bool {
		!self.requested
	}

	/// Screenshot task once the delay has elapsed (fires only once).
	pub fn poll(&mut self, now: Instant) -> Option<Task<Message>> {
		if self.requested || now < self.due {
			return None;
		}
		self.requested = true;
		Some(
			iced::window::latest()
				.and_then(iced::window::screenshot)
				.map(Message::DevScreenshot),
		)
	}

	pub fn save(&self, screenshot: &Screenshot) {
		let Some(image) =
			image::RgbaImage::from_raw(screenshot.size.width, screenshot.size.height, screenshot.rgba.to_vec())
		else {
			log::error!("devtools: screenshot buffer size mismatch");
			return;
		};
		match image.save(&self.path) {
			Ok(()) => log::info!("devtools: saved screenshot to {}", self.path.display()),
			Err(error) => log::error!("devtools: failed to save screenshot: {error}"),
		}
	}
}

/// Messages that stage the UI described by `NADE_SCREENSHOT_SCENE`.
pub fn scene_messages(select_layer: impl Fn(&str) -> Option<AppPanelMessage>) -> Vec<Message> {
	let Ok(scene) = std::env::var("NADE_SCREENSHOT_SCENE") else {
		return Vec::new();
	};

	scene
		.split(';')
		.filter_map(|step| step.split_once('='))
		.filter_map(|(key, value)| match key.trim() {
			"select" => select_layer(value.trim())
				.map(|message| Message::PanelSystem(panel_system::PanelSystemMessage::AppMessage(message))),
			"workspace" => Workspace::ALL
				.into_iter()
				.find(|workspace| workspace.label().eq_ignore_ascii_case(value.trim()))
				.map(|workspace| Message::Shell(ShellMessage::SelectWorkspace(workspace))),
			"tool" => Tool::from_key(value.trim()).map(|tool| Message::Shell(ShellMessage::SelectTool(tool))),
			"time" => value
				.trim()
				.parse()
				.ok()
				.map(|time| Message::Timeline(timeline_panel::TimelineMessage::PlayheadChanged(time))),
			"loop" => Some(Message::Transport(crate::message::TransportMessage::ToggleLoop)),
			"menu" => value
				.trim()
				.parse()
				.ok()
				.map(|area| Message::PanelSystem(panel_system::PanelSystemMessage::ToggleEditorMenu(area))),
			"play" => Some(Message::Transport(crate::message::TransportMessage::TogglePlay)),
			_ => None,
		})
		.collect()
}
