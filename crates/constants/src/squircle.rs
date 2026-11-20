//! Squircle paths for canvas drawing.
//!
//! Quad backgrounds (buttons, fields, menus, chips, …) get the same corner
//! shape from the patched `iced_wgpu` quad shader. Custom canvas paths use
//! this module so they match.

use iced::Rectangle;
use iced::widget::canvas::Path;

/// Ratio between an iced-style circular radius and the extent of the
/// continuous corner drawn by the quad shader and [`path`].
pub const EXTENT: f32 = 2.25;

/// Builds a squircle path with the same radius semantics as `iced-squircle`:
/// the corner looks as deep as a circular arc of `radius`.
pub fn path(rect: Rectangle, radius: f32) -> Path {
	iced_squircle::path(rect, radius)
}
