//! Viewer display settings (After Effects style resolution and guides).

use std::fmt;

/// Preview render resolution relative to the composition size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Resolution {
	#[default]
	Full,
	Half,
	Third,
	Quarter,
}

impl Resolution {
	pub const ALL: [Self; 4] = [Self::Full, Self::Half, Self::Third, Self::Quarter];

	/// Render scale factor.
	pub fn factor(self) -> f32 {
		match self {
			Self::Full => 1.0,
			Self::Half => 0.5,
			Self::Third => 1.0 / 3.0,
			Self::Quarter => 0.25,
		}
	}
}

impl fmt::Display for Resolution {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Full => "Full",
			Self::Half => "Half",
			Self::Third => "Third",
			Self::Quarter => "Quarter",
		})
	}
}

/// Composition facts shown in the viewer and status bar.
#[derive(Debug, Clone, Copy)]
pub struct CompositionSummary<'a> {
	pub name: &'a str,
	pub width: u32,
	pub height: u32,
	pub fps: f32,
	pub duration: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ViewerSettings {
	pub show_grid: bool,
	pub show_safe_areas: bool,
	pub resolution: Resolution,
}
