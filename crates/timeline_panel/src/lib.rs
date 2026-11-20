//! # Timeline Widget
//!
//! Icedアプリケーション用のタイムラインウィジェット。
//! トラック、クリップ、再生ヘッドの管理と表示を提供します。

mod icons;
mod interaction;
mod message;
mod widget;

pub use interaction::{DropSlot, TimelineInteraction};
pub use message::{TimelineCanvasEvent, TimelineMessage, TimelineUpdate};
pub use nade_core::{TimelineClip, TimelineModel, TimelineTrack, TrackVisibilityDisplay};
pub use widget::{DropPreview, TimelineWidget};
