//! Squircle paths for canvas drawing.
//!
//! Quad backgrounds (buttons, fields, menus, chips, …) get the same corner
//! shape from the patched `iced_wgpu` quad shader. Custom canvas paths use
//! this module so they match.

use iced::Rectangle;
use iced::widget::canvas::Path;

/// Builds a squircle path with the same radius semantics as `iced-squircle`:
/// the corner looks as deep as a circular arc of `radius`.
pub fn path(rect: Rectangle, radius: f32) -> Path {
	iced_squircle::path(rect, radius)
}
