//! Viewer panel: composition header, pixel viewport with interactive
//! overlays and a transport bar.

mod message;
mod settings;
mod state;
mod tool;
mod view;
mod widgets;

pub use message::{PreviewMessage, TransportMessage, ViewerRequest};
pub use settings::{Resolution, ViewerSettings};
pub use state::ViewerState;
pub use tool::Tool;
pub use view::{ViewerProps, view};
pub use widgets::viewer_overlay::LayerBox;
pub use widgets::viewport::{self, Viewport};
