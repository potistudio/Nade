//! Canvas panel module with pan/zoom support for Iced
//!
//! Provides a reusable canvas widget with:
//! - Pan and zoom functionality
//! - Optional grid drawing
//! - Coordinate transformation utilities

pub mod canvas;

pub use canvas::{CanvasProgram, CanvasState, GridConfig};
