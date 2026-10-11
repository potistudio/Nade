//! Viewer tools (After Effects style tool palette).

use constants::icons::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
	#[default]
	Select,
	Hand,
	Zoom,
	Rotate,
	Text,
}

impl Tool {
	pub const ALL: [Self; 5] = [Self::Select, Self::Hand, Self::Zoom, Self::Rotate, Self::Text];

	pub fn index(self) -> usize {
		Self::ALL.iter().position(|tool| *tool == self).unwrap_or(0)
	}

	pub fn icon(self) -> Icon {
		match self {
			Self::Select => Icon::ToolSelect,
			Self::Hand => Icon::ToolHand,
			Self::Zoom => Icon::ToolZoom,
			Self::Rotate => Icon::ToolRotate,
			Self::Text => Icon::ToolText,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Select => "Selection Tool",
			Self::Hand => "Hand Tool",
			Self::Zoom => "Zoom Tool",
			Self::Rotate => "Rotation Tool",
			Self::Text => "Type Tool",
		}
	}

	pub fn shortcut(self) -> &'static str {
		match self {
			Self::Select => "V",
			Self::Hand => "H",
			Self::Zoom => "Z",
			Self::Rotate => "W",
			Self::Text => "T",
		}
	}

	/// Contextual help shown in the status bar.
	pub fn hint(self) -> &'static str {
		match self {
			Self::Select => "Drag in the viewer to move the selected layer",
			Self::Hand => "Drag to pan the viewer",
			Self::Zoom => "Click to zoom in · Option-click to zoom out",
			Self::Rotate => "Drag around the selected layer to rotate it",
			Self::Text => "Click in the viewer to place a new text layer",
		}
	}

	pub fn tooltip(self) -> String {
		format!("{}  {}", self.label(), self.shortcut())
	}

	pub fn from_key(key: &str) -> Option<Self> {
		Self::ALL
			.into_iter()
			.find(|tool| tool.shortcut().eq_ignore_ascii_case(key))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shortcuts_map_back_to_tools() {
		for tool in Tool::ALL {
			assert_eq!(Tool::from_key(&tool.shortcut().to_lowercase()), Some(tool));
		}
		assert_eq!(Tool::from_key("x"), None);
	}
}
