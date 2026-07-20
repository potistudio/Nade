//! ドックノードの定義
//!
//! エリアの分割レイアウトを表現するツリー構造を提供します。

use crate::container::Area;

/// ドックノード（分割レイアウト）
#[derive(Debug, Clone)]
pub enum DockNode<C: Clone + std::fmt::Debug> {
	/// 空のノード
	Empty,
	/// エリア（リーフ）
	Leaf(Area<C>),
	/// 分割ノード
	Split {
		direction: SplitDirection,
		ratio: f32,
		first: Box<DockNode<C>>,
		second: Box<DockNode<C>>,
	},
}

impl<C: Clone + std::fmt::Debug> DockNode<C> {
	pub fn is_empty(&self) -> bool {
		match self {
			DockNode::Empty => true,
			DockNode::Leaf(_) => false,
			DockNode::Split { first, second, .. } => first.is_empty() && second.is_empty(),
		}
	}
}

impl<C: Clone + std::fmt::Debug> Default for DockNode<C> {
	fn default() -> Self {
		Self::Empty
	}
}

/// 分割方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
	/// 左右に分割
	Horizontal,
	/// 上下に分割
	Vertical,
}
