//! # Object Browser
//!
//! Composition → object (instance) outline for Iced.

mod message;
mod state;
mod widget;

pub use message::{ObjectId, ProjectPaneMessage, ProjectPaneUpdate};
pub use state::ProjectPaneState;
pub use widget::ProjectPaneWidget;
