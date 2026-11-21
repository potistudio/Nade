//! Squircle paths for canvas drawing.
//!
//! Quad backgrounds (buttons, fields, menus, chips, …) get the same corner
//! shape from the patched `iced_wgpu` quad shader. Custom canvas paths use
//! this module so they match.

use iced::Rectangle;
use iced::widget::canvas::Path;

/// Builds a squircle path from a corner extent: the length each corner
/// consumes from its two adjacent edges, as in `iced-squircle` 1.0.
pub fn path(rect: Rectangle, extent: f32) -> Path {
	iced_squircle::path(rect, extent)
}
