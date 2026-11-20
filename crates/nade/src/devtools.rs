//! Debug-only helpers for UI work.
//!
//! `NADE_SCREENSHOT=out.png` saves the window as a PNG once the UI settles
//! (`NADE_SCREENSHOT_DELAY_MS`, default 2500) and quits — handy for PR
//! screenshots. `NADE_SCREENSHOT_SCENE` stages the UI first, e.g.
//! `select=Title;workspace=graph;tool=t;time=1.5;loop=1;play=1;menu=2;tab=2:1;addtab=2`
//! (`tab=AREA:INDEX` clicks a header tab, `addtab=AREA` opens the new-tab menu).
//! Media can be staged too: `import=/path/clip.mp4;add=1;solid=1;export=/tmp/out.mp4`, and edits
//! run with `edit=split` (`undo`, `redo`, `duplicate`, `copy`, `cut`, `paste`, `delete`, `precompose`, `work-start`, `work-end`, `move-in`,
//! `move-out`, `trim-in`, `trim-out`, `ease`). Menus open with `menubar=edit` or
//! `context=640,400` (window coordinates).

use std::path::PathBuf;

use iced::Task;
use iced::time::{Duration, Instant};
use iced::window::Screenshot;

use crate::message::{AppPanelMessage, Message, ShellMessage};
use crate::workspace::Workspace;
use nade_viewer_panel::Tool;

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
	use nade_panel_system::PanelSystemMessage;

	let Ok(scene) = std::env::var("NADE_SCREENSHOT_SCENE") else {
		return Vec::new();
	};

	scene
		.split(';')
		.filter_map(|step| step.split_once('='))
		.flat_map(|(key, value)| -> Vec<Message> {
			let value = value.trim();
			match key.trim() {
				"select" => select_layer(value)
					.map(|message| Message::PanelSystem(PanelSystemMessage::AppMessage(message)))
					.into_iter()
					.collect(),
				"workspace" => Workspace::ALL
					.into_iter()
					.find(|workspace| workspace.label().eq_ignore_ascii_case(value))
					.map(|workspace| Message::Shell(ShellMessage::SelectWorkspace(workspace)))
					.into_iter()
					.collect(),
				"tool" => Tool::from_key(value)
					.map(|tool| Message::Shell(ShellMessage::SelectTool(tool)))
					.into_iter()
					.collect(),
				"time" => value
					.parse()
					.ok()
					.map(|time| Message::Timeline(nade_timeline_panel::TimelineMessage::PlayheadChanged(time)))
					.into_iter()
					.collect(),
				"loop" => vec![Message::Transport(crate::message::TransportMessage::ToggleLoop)],
				"menu" => value
					.parse()
					.ok()
					.map(|area| Message::PanelSystem(PanelSystemMessage::ToggleEditorMenu(area)))
					.into_iter()
					.collect(),
				"menubar" => crate::shell::menu::MenuKind::BAR
					.into_iter()
					.find(|kind| kind.title().eq_ignore_ascii_case(value))
					.map(|kind| Message::Menu(crate::message::MenuMessage::ToggleBar(kind)))
					.into_iter()
					.collect(),
				"context" => value
					.split_once(',')
					.and_then(|(x, y)| {
						let point = iced::Point::new(x.trim().parse().ok()?, y.trim().parse().ok()?);
						Some(Message::Menu(crate::message::MenuMessage::OpenContext(point)))
					})
					.into_iter()
					.collect(),
				"addtab" => value
					.parse()
					.ok()
					.map(|area| Message::PanelSystem(PanelSystemMessage::ToggleAddTabMenu(area)))
					.into_iter()
					.collect(),
				// A click: press and release without moving.
				"tab" => value
					.split_once(':')
					.and_then(|(area, index)| Some((area.parse().ok()?, index.parse().ok()?)))
					.map(|(area, index)| {
						vec![
							Message::PanelSystem(PanelSystemMessage::TabPress(area, index)),
							Message::PanelSystem(PanelSystemMessage::TabDragEnd),
						]
					})
					.unwrap_or_default(),
				"play" => vec![Message::Transport(crate::message::TransportMessage::TogglePlay)],
				"import" => {
					let path = PathBuf::from(value);
					let info = nade_media::probe(&path).map_err(|error| format!("{error:#}"));
					vec![Message::MediaImported(vec![(path, info)])]
				}
				"add" => vec![Message::PanelSystem(PanelSystemMessage::AppMessage(
					AppPanelMessage::Assets(nade_asset_browser::AssetBrowserMessage::AddToTimeline),
				))],
				"solid" => vec![Message::Shell(ShellMessage::NewSolid)],
				"export" => vec![Message::ExportTo(Some(PathBuf::from(value)))],
				"save" => vec![Message::SaveProjectTo(Some(PathBuf::from(value)))],
				"open" => vec![Message::ProjectChosen(Some(PathBuf::from(value)))],
				"edit" => {
					use crate::message::EditCommand;
					let command = match value {
						"undo" => EditCommand::Undo,
						"redo" => EditCommand::Redo,
						"split" => EditCommand::Split,
						"duplicate" => EditCommand::Duplicate,
						"copy" => EditCommand::Copy,
						"cut" => EditCommand::Cut,
						"paste" => EditCommand::Paste,
						"delete" => EditCommand::Delete,
						"precompose" => EditCommand::Precompose,
						"work-start" => EditCommand::WorkAreaStart,
						"work-end" => EditCommand::WorkAreaEnd,
						"move-in" => EditCommand::MoveInPoint,
						"move-out" => EditCommand::MoveOutPoint,
						"trim-in" => EditCommand::TrimInPoint,
						"trim-out" => EditCommand::TrimOutPoint,
						"ease" => EditCommand::EasyEase,
						_ => return Vec::new(),
					};
					vec![Message::Edit(command)]
				}
				_ => Vec::new(),
			}
		})
		.collect()
}
