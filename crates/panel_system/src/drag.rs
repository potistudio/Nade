//! ドラッグ状態の定義
//!
//! パネルのドラッグ状態とドロップゾーンを管理します。

use iced::Point;

use crate::node::SplitDirection;

/// ドラッグ状態
#[derive(Debug, Clone, Default)]
pub enum DragState {
	#[default]
	None,
	/// タブをドラッグ中（まだ閾値を超えていない）
	PendingDrag {
		source_container: usize,
		panel_id: usize,
		tab_index: usize,
		start_pos: Point,
	},
	/// タブをドラッグ中（フローティングパネルとして表示）
	DraggingTab {
		source_container: usize,
		panel_id: usize,
		start_pos: Point,
		current_pos: Point,
	},
	/// リサイズハンドルをドラッグ中
	Resizing {
		path: Vec<usize>,
		direction: SplitDirection,
		start_pos: Point,
		current_pos: Point,
		start_ratio: f32,
	},
}

impl DragState {
	pub fn is_dragging(&self) -> bool {
		matches!(self, DragState::DraggingTab { .. })
	}

	pub fn dragging_panel_id(&self) -> Option<usize> {
		match self {
			DragState::DraggingTab { panel_id, .. } => Some(*panel_id),
			_ => None,
		}
	}
}

/// ドロップゾーン
#[derive(Debug, Clone, PartialEq)]
pub struct DropZone {
	pub container_id: usize,
	pub position: DropPosition,
}

/// ドロップ位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition {
	/// 同じコンテナのタブとして追加
	Center,
	/// 左側に新しいパネルを作成
	Left,
	/// 右側に新しいパネルを作成
	Right,
	/// 上側に新しいパネルを作成
	Top,
	/// 下側に新しいパネルを作成
	Bottom,
}
