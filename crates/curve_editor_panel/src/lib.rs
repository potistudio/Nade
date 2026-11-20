//! Iced animation curve editor panel.

mod helpers;
mod message;
mod state;
mod widget;

pub use message::{CurveCanvasEvent, CurveEditorMessage, CurveEditorUpdate};
pub use state::CurveEditorState;
pub use widget::CurveEditorWidget;

const SIDEBAR_WIDTH: f32 = 148.0;
const TOOLBAR_HEIGHT: f32 = nade_constants::style::ROW_HEIGHT;
const HIT_RADIUS: f32 = 8.0;
const KEY_RADIUS: f32 = 4.5;
const HANDLE_RADIUS: f32 = 3.5;
const CURVE_SAMPLES: usize = 48;
