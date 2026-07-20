//! ドラッグ状態の定義

use iced::Point;

use crate::node::SplitDirection;

/// コーナー操作のプレビュー
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerAction {
	/// 左右分割
	SplitHorizontal,
	/// 上下分割
	SplitVertical,
}

impl CornerAction {
	pub fn direction(self) -> SplitDirection {
		match self {
			Self::SplitHorizontal => SplitDirection::Horizontal,
			Self::SplitVertical => SplitDirection::Vertical,
		}
	}
}

/// ドラッグ状態
#[derive(Debug, Clone, Default)]
pub enum DragState {
	#[default]
	None,
	/// リサイズハンドルをドラッグ中
	Resizing {
		path: Vec<usize>,
		direction: SplitDirection,
		start_pos: Point,
		current_pos: Point,
		start_ratio: f32,
	},
	/// エリア角をドラッグ中（分割）
	CornerDrag {
		area_id: usize,
		start_pos: Point,
		current_pos: Point,
		action: Option<CornerAction>,
		/// 分割比率（first / 全体）。`action` があるとき有効
		ratio: f32,
	},
}

impl DragState {
	pub fn corner_preview(&self) -> Option<(usize, CornerAction, f32)> {
		match self {
			Self::CornerDrag {
				area_id,
				action: Some(action),
				ratio,
				..
			} => Some((*area_id, *action, *ratio)),
			_ => None,
		}
	}
}
