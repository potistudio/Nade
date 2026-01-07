//! # Timeline Widget
//!
//! Icedアプリケーション用のタイムラインウィジェット。
//! トラック、クリップ、再生ヘッドの管理と表示を提供します。

mod state;
mod widget;

pub use state::{TimelineClip, TimelineState, TimelineTrack};
pub use widget::{TimelineMessage, TimelineWidget};
