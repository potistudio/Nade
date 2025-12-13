//! Status bar component for Iced
//!
//! Provides a simple status bar widget that displays version information.

use iced::widget::{container, row, text};
use iced::{Element, Length, Right};

/// Status bar component
#[derive(Default, Debug, Clone)]
pub struct StatusBar {
	version: String,
}

impl StatusBar {
	/// Create a new status bar with the default version
	pub fn new() -> Self {
		Self {
			version: "v0.0.1a".to_string(),
		}
	}

	/// Create a status bar with a custom version string
	pub fn with_version(version: impl Into<String>) -> Self {
		Self {
			version: version.into(),
		}
	}

	/// Render the status bar as an Iced Element
	pub fn view<Message: 'static>(&self) -> Element<'_, Message> {
		container(
			row![text(&self.version).size(12)]
				.width(Length::Fill)
				.align_y(iced::Alignment::Center),
		)
		.width(Length::Fill)
		.padding([4, 8])
		.align_x(Right)
		.into()
	}
}
