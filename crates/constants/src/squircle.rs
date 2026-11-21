//! Squircle paths for canvas drawing.
//!
//! Quad backgrounds (buttons, fields, menus, chips, …) get the same corner
//! shape from the patched `iced_wgpu` quad shader. Custom canvas paths use
//! this module so they match.

use iced::Rectangle;
use iced::widget::canvas::Path;

/// Ratio between the app's circular-style radius tokens and the extent of a
/// continuous corner. The quad shader and [`path`] both use it.
pub const EXTENT: f32 = 2.25;

/// Builds a squircle path with the app's radius semantics: the corner looks as
/// deep as a circular arc of `radius`. `iced-squircle` 1.0 takes the corner
/// extent directly, so scale the radius by [`EXTENT`].
pub fn path(rect: Rectangle, radius: f32) -> Path {
	iced_squircle::path(rect, radius * EXTENT)
}
