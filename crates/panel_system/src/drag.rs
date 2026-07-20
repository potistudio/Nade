//! ドラッグ状態の定義

use iced::Point;

use crate::node::SplitDirection;

/// コーナー操作の種類
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerAction {
	/// 左右分割
	SplitHorizontal,
	/// 上下分割
	SplitVertical,
	/// 他エリアへ移動（サイズ付きドック）
	Move,
}

impl CornerAction {
	pub fn direction(self) -> Option<SplitDirection> {
		match self {
			Self::SplitHorizontal => Some(SplitDirection::Horizontal),
			Self::SplitVertical => Some(SplitDirection::Vertical),
			Self::Move => None,
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
	/// エリア角をドラッグ中（分割 or 移動）
	CornerDrag {
		area_id: usize,
		start_pos: Point,
		current_pos: Point,
		action: Option<CornerAction>,
		/// 分割比率（first / 全体）
		ratio: f32,
		/// 移動先エリア。Move のとき有効
		target_area_id: Option<usize>,
		/// Move / Split の分割方向
		direction: Option<SplitDirection>,
		/// 移動先で新しいエリアを first 側に置くか
		new_is_first: bool,
	},
}

impl DragState {
	pub fn corner_preview(&self) -> Option<(usize, CornerAction, f32, Option<usize>, Option<SplitDirection>, bool)> {
		match self {
			Self::CornerDrag {
				area_id,
				action: Some(action),
				ratio,
				target_area_id,
				direction,
				new_is_first,
				..
			} => Some((*area_id, *action, *ratio, *target_area_id, *direction, *new_is_first)),
			_ => None,
		}
	}
}
