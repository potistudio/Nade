//! # Timeline Widget
//!
//! Icedアプリケーション用のタイムラインウィジェット。
//! トラック、クリップ、再生ヘッドの管理と表示を提供します。

mod icons;
mod interaction;
mod message;
mod widget;

pub use core::{TimelineClip, TimelineModel, TimelineTrack, TrackVisibilityDisplay};
pub use interaction::TimelineInteraction;
pub use message::{TimelineCanvasEvent, TimelineMessage, TimelineUpdate};
pub use widget::TimelineWidget;
