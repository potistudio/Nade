//! ドラッグ状態の定義

use iced::Point;

use crate::node::SplitDirection;

/// コーナー操作のプレビュー
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
	},
}

impl DragState {
	pub fn corner_action(&self) -> Option<(usize, CornerAction)> {
		match self {
			Self::CornerDrag {
				area_id,
				action: Some(action),
				..
			} => Some((*area_id, *action)),
			_ => None,
		}
	}
}
