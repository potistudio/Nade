//! # Panel System
//!
//! Blender 風のエリア分割システム。
//! 各エリアは単一エディタを表示し、コーナー分割・結合・種別切替・リサイズをサポートします。

mod consts;
mod container;
mod drag;
mod node;
mod system;

pub use container::Area;
pub use drag::{CornerAction, DragState};
pub use node::{DockNode, SplitDirection};
pub use system::{LayoutBuilder, PanelSystem, PanelSystemMessage};

/// エリアに表示できるエディタ種別
pub trait AreaKind: Clone + std::fmt::Debug + PartialEq + Eq + ToString + 'static {
	/// 選択可能な全種別
	fn all() -> Vec<Self>;
}
