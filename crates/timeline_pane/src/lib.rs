//! # Timeline Widget
//!
//! Icedアプリケーション用のタイムラインウィジェット。
//! トラック、クリップ、再生ヘッドの管理と表示を提供します。

mod interaction;
mod utils;
mod widget;

pub use interaction::TimelineInteraction;
pub use nade_core::{TimelineClip, TimelineModel, TimelineTrack};
pub use widget::{TimelineCanvasEvent, TimelineMessage, TimelineUpdate, TimelineWidget};
