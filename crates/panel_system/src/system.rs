//! Blender 風エリアシステムのメイン実装
//!
//! 単一エディタのエリア分割・結合・種別切替・リサイズを提供します。

use std::time::Instant;

use iced::{
	Color, Element, Length, Point, Rectangle, Size, Vector, mouse,
	widget::{Canvas, Space, button, column, container, float, mouse_area, row, stack, svg},
};

use crate::{
	AreaKind,
	chrome::{CornerGrip, PanelChrome, squircle_panel_stack},
	consts::{
		CORNER_DRAG_THRESHOLD, CORNER_SIZE, HEADER_HEIGHT, HEADER_PADDING, MIN_PANEL_SIZE, MOVE_CENTER_ZONE, PANEL_GAP,
		PANEL_OUTLINE_WIDTH, RESIZE_HIT_SIZE, colors,
	},
	container::Area,
	drag::{CornerAction, CornerPreview, DragState},
	node::{DockNode, SplitDirection},
};

/// Editor-type dropdown open/close duration.
const EDITOR_MENU_ANIM_SECS: f32 = 0.14;
/// Vertical slide distance (px) while the menu fades in.
const EDITOR_MENU_SLIDE_Y_PX: f32 = 6.0;
/// Focus ring cross-fade duration.
const FOCUS_ANIM_SECS: f32 = constants::style::MOTION_STANDARD_MS as f32 / 1000.0;

#[derive(Debug, Clone)]
struct EditorMenuState {
	area_id: usize,
	/// Openness at animation start.
	from: f32,
	/// Openness target (`1.0` open, `0.0` closed).
	to: f32,
	started: Instant,
}

impl EditorMenuState {
	fn value(&self) -> f32 {
		let t = self.started.elapsed().as_secs_f32() / EDITOR_MENU_ANIM_SECS;
		if t >= 1.0 {
			self.to
		} else {
			self.from + (self.to - self.from) * ease_out_cubic(t)
		}
	}

	fn is_finished(&self) -> bool {
		self.started.elapsed().as_secs_f32() >= EDITOR_MENU_ANIM_SECS
	}
}

fn ease_out_cubic(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	1.0 - (1.0 - t).powi(3)
}

fn editor_menu_transition(area_id: usize, from: f32, to: f32) -> EditorMenuState {
	EditorMenuState {
		area_id,
		from,
		to,
		started: Instant::now(),
	}
}

/// Child bounds for a split. `first_size` is pixels, or a ratio when `<= 1.0`.
fn split_child_bounds(bounds: Rectangle, direction: SplitDirection, first_size: f32) -> (Rectangle, Rectangle) {
	match direction {
		SplitDirection::Horizontal => {
			let available = (bounds.width - PANEL_GAP).max(0.0);
			let first_width = leading_size(first_size, available);
			let second_width = (available - first_width).max(0.0);
			(
				Rectangle::new(bounds.position(), Size::new(first_width, bounds.height)),
				Rectangle::new(
					Point::new(bounds.x + first_width + PANEL_GAP, bounds.y),
					Size::new(second_width, bounds.height),
				),
			)
		}
		SplitDirection::Vertical => {
			let available = (bounds.height - PANEL_GAP).max(0.0);
			let first_height = leading_size(first_size, available);
			let second_height = (available - first_height).max(0.0);
			(
				Rectangle::new(bounds.position(), Size::new(bounds.width, first_height)),
				Rectangle::new(
					Point::new(bounds.x, bounds.y + first_height + PANEL_GAP),
					Size::new(bounds.width, second_height),
				),
			)
		}
	}
}

fn leading_size(first_size: f32, available: f32) -> f32 {
	if available <= 0.0 {
		return 0.0;
	}
	let min = MIN_PANEL_SIZE.min(available * 0.5);
	let max = (available - min).max(min);
	let raw = if first_size <= 1.0 {
		available * first_size.clamp(0.0, 1.0)
	} else {
		first_size
	};
	raw.clamp(min, max)
}

fn available_along(bounds: Rectangle, direction: SplitDirection) -> f32 {
	match direction {
		SplitDirection::Horizontal => (bounds.width - PANEL_GAP).max(0.0),
		SplitDirection::Vertical => (bounds.height - PANEL_GAP).max(0.0),
	}
}

struct CornerDragUpdate {
	action: Option<CornerAction>,
	ratio: f32,
	target_area_id: Option<usize>,
	direction: Option<SplitDirection>,
	new_is_first: bool,
}

impl CornerDragUpdate {
	fn none() -> Self {
		Self {
			action: None,
			ratio: 0.5,
			target_area_id: None,
			direction: None,
			new_is_first: false,
		}
	}
}

enum DropLayout {
	/// 中央ドロップ: エリア全体を入れ替え
	FullMove,
	/// 中央から離れた方向へ分割ドック
	Split {
		direction: SplitDirection,
		ratio: f32,
		new_is_first: bool,
	},
}

// =============================================================================
// メッセージ
// =============================================================================

/// パネルシステムのメッセージ
#[derive(Debug, Clone)]
pub enum PanelSystemMessage<C, M = ()>
where
	C: Clone + std::fmt::Debug,
	M: Clone + std::fmt::Debug,
{
	/// エリアのエディタ種別を変更
	ChangeEditor(usize, C),
	/// エディタ種別メニューの開閉
	ToggleEditorMenu(usize),
	/// エディタ種別メニューを閉じる
	CloseEditorMenu,
	/// エディタ種別メニューの開閉アニメーションを進める
	AnimTick,
	/// 角ドラッグ開始
	CornerDragStart(usize),
	MouseMove(Point),
	CornerDragEnd,
	/// このエリアを削除し、兄弟と結合
	JoinArea(usize),
	ResizeStart(Vec<usize>, SplitDirection),
	ResizeEnd,
	WindowResized(Size),
	/// 現在のポインタ位置にあるエリアへフォーカスを移す（マウス押下時）
	FocusPointer,
	/// ポインタがパネルシステムの外へ出た
	PointerExited,
	/// リサイズガターのホバー開始／終了
	GutterHover(Vec<usize>, bool),
	/// アプリケーション固有のメッセージ
	AppMessage(M),
}

// =============================================================================
// パネルシステム
// =============================================================================

/// Blender 風エリアシステム
#[derive(Debug)]
pub struct PanelSystem<C: AreaKind> {
	root: DockNode<C>,
	next_area_id: usize,
	drag_state: DragState,
	last_mouse_pos: Point,
	window_size: Size,
	/// エディタ種別メニュー（開閉アニメーション付き）
	editor_menu: Option<EditorMenuState>,
	/// フォーカス中のエリア（アクセントのリングで示す）
	focused_area: Option<usize>,
	/// 直前にフォーカスされていたエリア（リングのフェードアウト用）
	previous_focus: Option<usize>,
	/// フォーカスが切り替わった時刻
	focus_changed: Instant,
	/// ホバー中のガター (split path, 重なったヒット領域の数)
	gutter_hover: Option<(Vec<usize>, u32)>,
	/// ポインタがパネルシステム上にあるか
	pointer_inside: bool,
}

impl<C: AreaKind> PanelSystem<C> {
	pub fn new() -> Self {
		Self {
			root: DockNode::Empty,
			next_area_id: 0,
			drag_state: DragState::None,
			last_mouse_pos: Point::ORIGIN,
			window_size: Size::new(800.0, 600.0),
			editor_menu: None,
			focused_area: None,
			previous_focus: None,
			focus_changed: Instant::now(),
			gutter_hover: None,
			pointer_inside: false,
		}
	}

	/// Whether the editor menu open/close animation still needs ticks.
	pub fn is_editor_menu_animating(&self) -> bool {
		self.editor_menu.as_ref().is_some_and(|menu| !menu.is_finished())
	}

	/// Whether any chrome animation (menu, focus ring) still needs ticks.
	pub fn is_animating(&self) -> bool {
		self.is_editor_menu_animating() || self.focus_changed.elapsed().as_secs_f32() < FOCUS_ANIM_SECS
	}

	/// Content of the focused area, if any.
	pub fn focused_content(&self) -> Option<C> {
		self.focused_area.and_then(|id| self.find_area_content(id))
	}

	/// Replace the whole layout (workspace switch). Ratios are materialized
	/// against the current window size.
	pub fn set_layout(&mut self, layout: DockNode<C>, next_area_id: usize) {
		self.root = layout;
		self.next_area_id = self.next_area_id.max(next_area_id);
		self.drag_state = DragState::None;
		self.editor_menu = None;
		self.gutter_hover = None;
		self.focused_area = None;
		self.previous_focus = None;
		self.materialize_split_sizes();
	}

	fn focus_area(&mut self, area_id: usize) {
		if self.focused_area == Some(area_id) {
			return;
		}
		self.previous_focus = self.focused_area;
		self.focused_area = Some(area_id);
		self.focus_changed = Instant::now();
	}

	/// Focus ring progress for an area (`0.0` → `1.0`).
	fn focus_progress(&self, area_id: usize) -> f32 {
		let t = ease_out_cubic(self.focus_changed.elapsed().as_secs_f32() / FOCUS_ANIM_SECS);
		if self.focused_area == Some(area_id) {
			t
		} else if self.previous_focus == Some(area_id) {
			1.0 - t
		} else {
			0.0
		}
	}

	fn is_gutter_hot(&self, path: &[usize]) -> bool {
		let resizing = matches!(&self.drag_state, DragState::Resizing { path: active, .. } if active == path);
		let hovered = self
			.gutter_hover
			.as_ref()
			.is_some_and(|(hover_path, count)| *count > 0 && hover_path == path);
		resizing || (hovered && matches!(self.drag_state, DragState::None))
	}

	fn open_editor_menu(&mut self, area_id: usize) {
		if let Some(menu) = self.editor_menu.as_ref().filter(|menu| menu.area_id == area_id) {
			let current = menu.value();
			let to = if menu.to > 0.0 { 0.0 } else { 1.0 };
			self.editor_menu = Some(editor_menu_transition(area_id, current, to));
			return;
		}
		self.editor_menu = Some(editor_menu_transition(area_id, 0.0, 1.0));
	}

	fn close_editor_menu(&mut self) {
		let Some(menu) = self.editor_menu.as_ref() else {
			return;
		};
		if menu.to == 0.0 {
			return;
		}
		let area_id = menu.area_id;
		let current = menu.value();
		self.editor_menu = Some(editor_menu_transition(area_id, current, 0.0));
	}

	fn tick_editor_menu(&mut self) {
		if self
			.editor_menu
			.as_ref()
			.is_some_and(|menu| menu.is_finished() && menu.to <= 0.0)
		{
			self.editor_menu = None;
		}
	}

	pub fn with_layout(mut self, layout: DockNode<C>) -> Self {
		self.root = layout;
		self
	}

	pub fn with_ids(mut self, next_area_id: usize) -> Self {
		self.next_area_id = next_area_id;
		self
	}

	pub fn root(&self) -> &DockNode<C> {
		&self.root
	}

	pub fn root_mut(&mut self) -> &mut DockNode<C> {
		&mut self.root
	}

	pub fn update<M>(&mut self, message: PanelSystemMessage<C, M>)
	where
		M: Clone + std::fmt::Debug,
	{
		match message {
			PanelSystemMessage::ChangeEditor(area_id, content) => {
				self.set_area_content(area_id, content);
				self.close_editor_menu();
			}

			PanelSystemMessage::ToggleEditorMenu(area_id) => {
				self.open_editor_menu(area_id);
			}

			PanelSystemMessage::CloseEditorMenu => {
				self.close_editor_menu();
			}

			PanelSystemMessage::AnimTick => {
				self.tick_editor_menu();
			}

			PanelSystemMessage::CornerDragStart(area_id) => {
				self.drag_state = DragState::CornerDrag {
					area_id,
					start_pos: self.last_mouse_pos,
					current_pos: self.last_mouse_pos,
					action: None,
					ratio: 0.5,
					target_area_id: None,
					direction: None,
					new_is_first: false,
				};
			}

			PanelSystemMessage::MouseMove(pos) => {
				self.last_mouse_pos = pos;
				self.pointer_inside = true;

				match self.drag_state.clone() {
					DragState::CornerDrag { area_id, start_pos, .. } => {
						let update = self.corner_drag_update(area_id, start_pos, pos);
						self.drag_state = DragState::CornerDrag {
							area_id,
							start_pos,
							current_pos: pos,
							action: update.action,
							ratio: update.ratio,
							target_area_id: update.target_area_id,
							direction: update.direction,
							new_is_first: update.new_is_first,
						};
					}
					DragState::Resizing {
						path,
						direction,
						start_pos,
						start_first_size,
						..
					} => {
						let new_size =
							self.calculate_new_first_size(&path, start_pos, pos, start_first_size, direction);
						self.set_first_size_at_path(&path, new_size);
						self.drag_state = DragState::Resizing {
							path,
							direction,
							start_pos,
							current_pos: pos,
							start_first_size,
						};
					}
					_ => {}
				}
			}

			PanelSystemMessage::CornerDragEnd => {
				if let DragState::CornerDrag {
					area_id,
					action: Some(action),
					ratio,
					target_area_id,
					direction,
					new_is_first,
					..
				} = self.drag_state.clone()
				{
					match action {
						CornerAction::Move => {
							if let Some(target) = target_area_id {
								if let Some(direction) = direction {
									self.move_area_into(area_id, target, direction, ratio, new_is_first);
								} else {
									// 中央ドロップ → 全体移動（入れ替え）
									self.swap_areas(area_id, target);
								}
							}
						}
						CornerAction::SplitHorizontal | CornerAction::SplitVertical => {
							if let Some(direction) = action.direction() {
								self.split_area(area_id, direction, ratio, false);
							}
						}
					}
				}
				self.drag_state = DragState::None;
			}

			PanelSystemMessage::JoinArea(area_id) => {
				self.close_editor_menu();
				self.join_area(area_id);
			}

			PanelSystemMessage::ResizeStart(path, direction) => {
				if let Some(first_size) = self.get_first_size_at_path(&path) {
					self.drag_state = DragState::Resizing {
						path,
						direction,
						start_pos: self.last_mouse_pos,
						current_pos: self.last_mouse_pos,
						start_first_size: first_size,
					};
				}
			}

			PanelSystemMessage::ResizeEnd => {
				self.drag_state = DragState::None;
			}

			PanelSystemMessage::FocusPointer => {
				if self.pointer_inside
					&& let Some(area_id) = self.area_at_point(self.last_mouse_pos)
				{
					self.focus_area(area_id);
				}
			}

			PanelSystemMessage::PointerExited => {
				self.pointer_inside = false;
			}

			PanelSystemMessage::GutterHover(path, entered) => match (&mut self.gutter_hover, entered) {
				(Some((hover_path, count)), true) if *hover_path == path => *count += 1,
				(_, true) => self.gutter_hover = Some((path, 1)),
				(Some((hover_path, count)), false) if *hover_path == path => {
					*count = count.saturating_sub(1);
				}
				(_, false) => {}
			},

			PanelSystemMessage::AppMessage(_) => {}

			PanelSystemMessage::WindowResized(size) => {
				self.window_size = size;
				self.materialize_split_sizes();
			}
		}
	}

	fn calculate_new_first_size(
		&self,
		path: &[usize],
		start_pos: Point,
		current_pos: Point,
		start_first_size: f32,
		direction: SplitDirection,
	) -> f32 {
		let bounds = self.bounds_at_path(path);
		let available = available_along(bounds, direction);
		let min = MIN_PANEL_SIZE.min(available * 0.5);
		let max = (available - min).max(min);
		let delta = match direction {
			SplitDirection::Horizontal => current_pos.x - start_pos.x,
			SplitDirection::Vertical => current_pos.y - start_pos.y,
		};
		let materialized = if start_first_size <= 1.0 {
			available * start_first_size.clamp(0.0, 1.0)
		} else {
			start_first_size
		};
		(materialized + delta).clamp(min, max)
	}

	/// Convert ratio-sized splits to pixels and clamp to the current window.
	fn materialize_split_sizes(&mut self) {
		let root_bounds = Rectangle::new(Point::ORIGIN, self.window_size);
		Self::materialize_node(&mut self.root, root_bounds);
	}

	fn materialize_node(node: &mut DockNode<C>, bounds: Rectangle) {
		let DockNode::Split {
			direction,
			first_size,
			first,
			second,
		} = node
		else {
			return;
		};
		let available = available_along(bounds, *direction);
		// Only convert ratio → px. Keep stored px on window resize so the Fill side absorbs change.
		if *first_size <= 1.0 {
			*first_size = leading_size(*first_size, available);
		}
		let (first_bounds, second_bounds) = split_child_bounds(bounds, *direction, *first_size);
		Self::materialize_node(first, first_bounds);
		Self::materialize_node(second, second_bounds);
	}

	/// コーナードラッグ中の操作（分割 or 他エリアへの移動）を算出する
	fn corner_drag_update(&self, source_id: usize, start_pos: Point, pos: Point) -> CornerDragUpdate {
		// 他エリア上 → 中央基準の移動（中央=全体 / 外側=その方向へ分割）
		if let Some(target_id) = self.area_at_point(pos)
			&& target_id != source_id
			&& let Some(drop) = self.drop_layout_at(target_id, pos)
		{
			return match drop {
				DropLayout::FullMove => CornerDragUpdate {
					action: Some(CornerAction::Move),
					ratio: 0.5,
					target_area_id: Some(target_id),
					direction: None,
					new_is_first: false,
				},
				DropLayout::Split {
					direction,
					ratio,
					new_is_first,
				} => CornerDragUpdate {
					action: Some(CornerAction::Move),
					ratio,
					target_area_id: Some(target_id),
					direction: Some(direction),
					new_is_first,
				},
			};
		}

		let dx = pos.x - start_pos.x;
		let dy = pos.y - start_pos.y;
		let distance = (dx * dx + dy * dy).sqrt();

		// 角からエリア内側（左上方向）へ十分な距離 → 分割
		if distance <= CORNER_DRAG_THRESHOLD || (dx >= 0.0 && dy >= 0.0) {
			return CornerDragUpdate::none();
		}

		let action = if dx.abs() > dy.abs() {
			CornerAction::SplitHorizontal
		} else {
			CornerAction::SplitVertical
		};
		let direction = action.direction();

		let ratio = self
			.area_path(source_id)
			.map(|path| {
				let bounds = self.bounds_at_path(&path);
				match action {
					CornerAction::SplitHorizontal => {
						if bounds.width > 1.0 {
							((pos.x - bounds.x) / bounds.width).clamp(0.15, 0.85)
						} else {
							0.5
						}
					}
					CornerAction::SplitVertical => {
						if bounds.height > 1.0 {
							((pos.y - bounds.y) / bounds.height).clamp(0.15, 0.85)
						} else {
							0.5
						}
					}
					CornerAction::Move => 0.5,
				}
			})
			.unwrap_or(0.5);

		CornerDragUpdate {
			action: Some(action),
			ratio,
			target_area_id: None,
			direction,
			new_is_first: false,
		}
	}

	/// ドロップ位置を中央基準で解釈する
	///
	/// - 中央付近 → 全体移動
	/// - 中央から上/下/左/右へ離れる → その方向への分割（分割線はカーソル位置）
	fn drop_layout_at(&self, target_id: usize, pos: Point) -> Option<DropLayout> {
		let path = self.area_path(target_id)?;
		let bounds = self.bounds_at_path(&path);
		if bounds.width <= 1.0 || bounds.height <= 1.0 {
			return None;
		}

		let rx = ((pos.x - bounds.x) / bounds.width).clamp(0.0, 1.0);
		let ry = ((pos.y - bounds.y) / bounds.height).clamp(0.0, 1.0);
		let dx = rx - 0.5;
		let dy = ry - 0.5;

		// 中央ゾーン → 全体移動
		if dx.abs() < MOVE_CENTER_ZONE && dy.abs() < MOVE_CENTER_ZONE {
			return Some(DropLayout::FullMove);
		}

		// 中央から離れた主軸方向へ分割（分割位置は中央基準で倍率マップ）
		if dx.abs() > dy.abs() {
			let new_is_first = dx < 0.0; // 左 = first
			Some(DropLayout::Split {
				direction: SplitDirection::Horizontal,
				ratio: ratio_from_center(rx, new_is_first),
				new_is_first,
			})
		} else {
			let new_is_first = dy < 0.0; // 上 = first
			Some(DropLayout::Split {
				direction: SplitDirection::Vertical,
				ratio: ratio_from_center(ry, new_is_first),
				new_is_first,
			})
		}
	}

	fn area_at_point(&self, pos: Point) -> Option<usize> {
		Self::find_area_at_point(&self.root, pos, Rectangle::new(Point::ORIGIN, self.window_size))
	}

	fn find_area_at_point(node: &DockNode<C>, pos: Point, bounds: Rectangle) -> Option<usize> {
		match node {
			DockNode::Leaf(area) => {
				if bounds.contains(pos) {
					Some(area.id)
				} else {
					None
				}
			}
			DockNode::Split {
				direction,
				first_size,
				first,
				second,
			} => {
				let (first_bounds, second_bounds) = split_child_bounds(bounds, *direction, *first_size);
				Self::find_area_at_point(first, pos, first_bounds)
					.or_else(|| Self::find_area_at_point(second, pos, second_bounds))
			}
			DockNode::Empty => None,
		}
	}

	fn area_path(&self, area_id: usize) -> Option<Vec<usize>> {
		let mut path = Vec::new();
		if Self::find_area_path(&self.root, area_id, &mut path) {
			Some(path)
		} else {
			None
		}
	}

	fn find_area_path(node: &DockNode<C>, area_id: usize, path: &mut Vec<usize>) -> bool {
		match node {
			DockNode::Leaf(area) => area.id == area_id,
			DockNode::Split { first, second, .. } => {
				path.push(0);
				if Self::find_area_path(first, area_id, path) {
					return true;
				}
				path.pop();
				path.push(1);
				if Self::find_area_path(second, area_id, path) {
					return true;
				}
				path.pop();
				false
			}
			DockNode::Empty => false,
		}
	}

	fn bounds_at_path(&self, path: &[usize]) -> Rectangle {
		let mut bounds = Rectangle::new(Point::ORIGIN, self.window_size);
		let mut node = &self.root;

		for &index in path {
			let DockNode::Split {
				direction,
				first_size,
				first,
				second,
			} = node
			else {
				break;
			};

			match direction {
				SplitDirection::Horizontal | SplitDirection::Vertical => {
					let (first_bounds, second_bounds) = split_child_bounds(bounds, *direction, *first_size);
					if index == 0 {
						bounds = first_bounds;
						node = first;
					} else {
						bounds = second_bounds;
						node = second;
					}
				}
			}
		}

		bounds
	}

	pub fn view<'a, F, M>(&'a self, content_view: F) -> Element<'a, PanelSystemMessage<C, M>>
	where
		F: Fn(usize, &C) -> Element<'a, PanelSystemMessage<C, M>> + Copy,
		M: Clone + std::fmt::Debug + 'static,
		C: 'a,
	{
		let main_content = self.view_node(&self.root, vec![], content_view);

		let base = container(main_content)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(constants::widgets::window);

		// Editor menu is drawn at the window root so short panels (timeline) cannot
		// clip it — Float still inherits the leaf viewport when nested inside an area.
		let base: Element<'_, PanelSystemMessage<C, M>> = if let Some(menu_layer) = self.view_editor_menu_layer() {
			stack![base, menu_layer].width(Length::Fill).height(Length::Fill).into()
		} else {
			base.into()
		};

		// ドラッグ中は全面オーバーレイで他パネル上でも追従できるようにする
		if matches!(
			self.drag_state,
			DragState::CornerDrag { .. } | DragState::Resizing { .. }
		) {
			let release = match &self.drag_state {
				DragState::CornerDrag { .. } => PanelSystemMessage::CornerDragEnd,
				_ => PanelSystemMessage::ResizeEnd,
			};

			let cursor = match &self.drag_state {
				DragState::Resizing {
					direction: SplitDirection::Horizontal,
					..
				} => mouse::Interaction::ResizingHorizontally,
				DragState::Resizing {
					direction: SplitDirection::Vertical,
					..
				} => mouse::Interaction::ResizingVertically,
				DragState::CornerDrag { .. } => mouse::Interaction::Grabbing,
				_ => mouse::Interaction::Idle,
			};

			let overlay = mouse_area(
				container(Space::new())
					.width(Length::Fill)
					.height(Length::Fill)
					.style(|_| container::Style {
						background: Some(Color::TRANSPARENT.into()),
						..Default::default()
					}),
			)
			.interaction(cursor)
			.on_move(PanelSystemMessage::MouseMove)
			.on_release(release);

			stack![base, overlay].width(Length::Fill).height(Length::Fill).into()
		} else {
			mouse_area(base)
				.on_move(PanelSystemMessage::MouseMove)
				.on_exit(PanelSystemMessage::PointerExited)
				.into()
		}
	}

	/// Window-level editor menu, positioned from the host area's absolute bounds.
	fn view_editor_menu_layer<'a, M>(&'a self) -> Option<Element<'a, PanelSystemMessage<C, M>>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let menu_state = self.editor_menu.as_ref()?;
		let anim = menu_state.value();
		if anim <= 0.001 {
			return None;
		}
		let area = Self::find_area_clone(&self.root, menu_state.area_id)?;
		let path = self.area_path(menu_state.area_id)?;
		let bounds = self.bounds_at_path(&path);
		let menu = self.view_editor_menu(area.id, area.content, anim);

		let slide_y = (1.0 - anim) * -EDITOR_MENU_SLIDE_Y_PX;
		let floating = float(menu).translate(move |_bounds, _viewport| Vector::new(0.0, slide_y));

		Some(
			column![
				Space::new().height(bounds.y + HEADER_HEIGHT),
				row![
					Space::new().width(bounds.x + constants::style::SPACE_2),
					floating,
					Space::new().width(Length::Fill),
				]
				.height(Length::Shrink),
				Space::new().height(Length::Fill),
			]
			.width(Length::Fill)
			.height(Length::Fill)
			.into(),
		)
	}

	fn view_editor_menu<'a, M>(&'a self, area_id: usize, content: C, anim: f32) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		use constants::style::{
			BORDER_SUBTLE_COLOR, FONT_TINY, FONT_UI, ICON_SIZE, SPACE_2, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR,
		};

		const COL_WIDTH: f32 = 136.0;
		const ROW_HEIGHT: f32 = constants::style::ROW_HEIGHT;

		let columns = row(C::menu_columns()
			.into_iter()
			.map(|(category, kinds)| {
				let content = content.clone();
				let header = container(
					constants::widgets::ui_label(category.to_uppercase(), FONT_TINY)
						.color(TEXT_MUTED_COLOR.scale_alpha(anim)),
				)
				.padding(iced::Padding {
					top: 2.0,
					right: 6.0,
					bottom: 4.0,
					left: 6.0,
				});

				let items = column(
					kinds
						.into_iter()
						.map(move |kind| {
							let selected = kind == content;
							let check: Element<'_, PanelSystemMessage<C, M>> = if selected {
								constants::icons::icon(
									constants::icons::Icon::Check,
									10.0,
									TEXT_PRIMARY_COLOR.scale_alpha(anim),
								)
							} else {
								Space::new().width(10.0).into()
							};

							let row_content = row![
								panel_icon_svg_colored(
									kind.icon(),
									ICON_SIZE,
									TEXT_PRIMARY_COLOR.scale_alpha(0.85 * anim),
									anim
								),
								constants::widgets::ui_label(kind.label(), FONT_UI),
								Space::new().width(Length::Fill),
								check,
							]
							.spacing(8)
							.align_y(iced::Alignment::Center)
							.width(Length::Fill);

							button(constants::widgets::button_body(row_content))
								.padding([0.0, 6.0])
								.width(Length::Fill)
								.height(ROW_HEIGHT)
								.style(constants::widgets::button_menu_item_faded(selected, anim))
								.on_press(PanelSystemMessage::ChangeEditor(area_id, kind))
								.into()
						})
						.collect::<Vec<_>>(),
				)
				.spacing(1);

				container(column![header, items].height(Length::Shrink))
					.width(COL_WIDTH)
					.height(Length::Shrink)
					.padding(SPACE_2)
					.into()
			})
			.collect::<Vec<Element<'_, PanelSystemMessage<C, M>>>>()
			.into_iter()
			.enumerate()
			.flat_map(|(index, column)| {
				let divider: Option<Element<'_, PanelSystemMessage<C, M>>> = (index > 0).then(|| {
					container(Space::new().width(1).height(Length::Fill))
						.padding([8, 0])
						.style(move |_| container::Style {
							background: Some(BORDER_SUBTLE_COLOR.scale_alpha(anim).into()),
							..Default::default()
						})
						.into()
				});
				divider.into_iter().chain(std::iter::once(column))
			})
			.collect::<Vec<_>>())
		.spacing(0)
		.align_y(iced::Alignment::Start)
		.height(Length::Shrink);

		container(columns)
			.height(Length::Shrink)
			.padding(2)
			.style(move |theme| constants::widgets::editor_menu_panel_faded(theme, anim))
			.into()
	}

	fn view_node<'a, F, M>(
		&'a self,
		node: &'a DockNode<C>,
		path: Vec<usize>,
		content_view: F,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		F: Fn(usize, &C) -> Element<'a, PanelSystemMessage<C, M>> + Copy,
		M: Clone + std::fmt::Debug + 'static,
	{
		match node {
			DockNode::Empty => Space::new().width(Length::Fill).height(Length::Fill).into(),

			DockNode::Leaf(area) => self.view_area(area, content_view),

			DockNode::Split {
				direction,
				first_size,
				first,
				second,
			} => {
				let mut first_path = path.clone();
				first_path.push(0);
				let mut second_path = path.clone();
				second_path.push(1);
				let direction = *direction;
				let available = available_along(self.bounds_at_path(&path), direction);
				let leading = leading_size(*first_size, available);
				let overhang = Length::Fixed(((RESIZE_HIT_SIZE - PANEL_GAP) * 0.5).max(0.0));
				let gutter_path = path.clone();
				let hit_path = path;

				match direction {
					SplitDirection::Horizontal => row![
						stack![
							container(self.view_node(first, first_path, content_view))
								.width(Length::Fill)
								.height(Length::Fill),
							row![
								Space::new().width(Length::Fill),
								self.view_resize_hit(direction, hit_path.clone(), overhang, Length::Fill),
							]
							.width(Length::Fill)
							.height(Length::Fill),
						]
						.width(Length::Fixed(leading))
						.height(Length::Fill),
						self.view_panel_gutter(direction, gutter_path),
						stack![
							container(self.view_node(second, second_path, content_view))
								.width(Length::Fill)
								.height(Length::Fill),
							row![
								self.view_resize_hit(direction, hit_path, overhang, Length::Fill),
								Space::new().width(Length::Fill),
							]
							.width(Length::Fill)
							.height(Length::Fill),
						]
						.width(Length::Fill)
						.height(Length::Fill),
					]
					.width(Length::Fill)
					.height(Length::Fill)
					.into(),
					SplitDirection::Vertical => column![
						stack![
							container(self.view_node(first, first_path, content_view))
								.width(Length::Fill)
								.height(Length::Fill),
							column![
								Space::new().height(Length::Fill),
								self.view_resize_hit(direction, hit_path.clone(), Length::Fill, overhang),
							]
							.width(Length::Fill)
							.height(Length::Fill),
						]
						.width(Length::Fill)
						.height(Length::Fixed(leading)),
						self.view_panel_gutter(direction, gutter_path),
						stack![
							container(self.view_node(second, second_path, content_view))
								.width(Length::Fill)
								.height(Length::Fill),
							column![
								self.view_resize_hit(direction, hit_path, Length::Fill, overhang),
								Space::new().height(Length::Fill),
							]
							.width(Length::Fill)
							.height(Length::Fill),
						]
						.width(Length::Fill)
						.height(Length::Fill),
					]
					.width(Length::Fill)
					.height(Length::Fill)
					.into(),
				}
			}
		}
	}

	fn view_area<'a, F, M>(&'a self, area: &'a Area<C>, content_view: F) -> Element<'a, PanelSystemMessage<C, M>>
	where
		F: Fn(usize, &C) -> Element<'a, PanelSystemMessage<C, M>> + Copy,
		M: Clone + std::fmt::Debug + 'static,
	{
		use constants::style::{
			CONTROL_CONTENT_HEIGHT, FONT_UI, ICON_SIZE, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR,
		};

		let area_id = area.id;
		let can_join = self.area_has_sibling(area_id);
		let focus = self.focus_progress(area_id);
		let menu_open = self
			.editor_menu
			.as_ref()
			.is_some_and(|menu| menu.area_id == area_id && menu.to > 0.0);

		// Editor type chip: [icon] Title [⌄] — the chevron flips while the menu is open.
		let title_color = constants::style::mix(TEXT_SECONDARY_COLOR, TEXT_PRIMARY_COLOR, focus);
		let chevron_turn = self
			.editor_menu
			.as_ref()
			.filter(|menu| menu.area_id == area_id)
			.map_or(0.0, EditorMenuState::value);
		let editor_picker = button(constants::widgets::button_body(
			row![
				panel_icon_svg_colored(area.content.icon(), ICON_SIZE, title_color, 1.0),
				constants::widgets::ui_label(area.content.label(), FONT_UI).color(title_color),
				constants::icons::icon_transformed(
					constants::icons::Icon::ChevronDown,
					10.0,
					if menu_open {
						TEXT_PRIMARY_COLOR
					} else {
						TEXT_MUTED_COLOR
					},
					chevron_turn * std::f32::consts::PI,
					1.0,
				),
			]
			.spacing(HEADER_PADDING)
			.align_y(iced::Alignment::Center),
		))
		.padding([0.0, HEADER_PADDING])
		.width(Length::Shrink)
		.height(CONTROL_CONTENT_HEIGHT)
		.style(constants::widgets::button_editor_type)
		.on_press(PanelSystemMessage::ToggleEditorMenu(area_id));

		let join_button: Element<'_, PanelSystemMessage<C, M>> = if can_join {
			constants::widgets::tip(
				button(constants::widgets::button_body_center(
					svg(constants::icons::Icon::Close.handle())
						.width(12)
						.height(12)
						.style(|_theme, status| svg::Style {
							color: Some(match status {
								svg::Status::Hovered => TEXT_PRIMARY_COLOR,
								svg::Status::Idle => TEXT_MUTED_COLOR,
							}),
						}),
				))
				.padding(0)
				.width(CONTROL_CONTENT_HEIGHT)
				.height(CONTROL_CONTENT_HEIGHT)
				.style(constants::widgets::button_ghost)
				.on_press(PanelSystemMessage::JoinArea(area_id)),
				"Close Panel",
				iced::widget::tooltip::Position::Bottom,
			)
		} else {
			Space::new().width(0).into()
		};

		let header = container(
			row![editor_picker, Space::new().width(Length::Fill), join_button]
				.spacing(HEADER_PADDING)
				.align_y(iced::Alignment::Center)
				.height(Length::Fill),
		)
		.width(Length::Fill)
		.height(HEADER_HEIGHT)
		.padding(HEADER_PADDING)
		.align_y(iced::Alignment::Center);

		let body = content_view(area.id, &area.content);

		let overlay_preview = self.drag_state.corner_preview().and_then(
			|CornerPreview {
			     source_id,
			     action,
			     ratio,
			     target_id,
			     direction,
			     new_is_first,
			 }| match action {
				CornerAction::Move => {
					if target_id == Some(area_id) {
						if let Some(dir) = direction {
							Some(self.view_dock_preview(dir, ratio, new_is_first))
						} else {
							Some(self.view_full_move_preview())
						}
					} else if source_id == area_id {
						Some(self.view_move_source_preview())
					} else {
						None
					}
				}
				CornerAction::SplitHorizontal | CornerAction::SplitVertical if source_id == area_id => {
					Some(self.view_split_preview(action, ratio))
				}
				_ => None,
			},
		);

		let corner = self.view_corner(area_id);

		let body_stack = if let Some(preview) = overlay_preview {
			stack![body, preview, self.view_corner_overlay(corner)]
				.width(Length::Fill)
				.height(Length::Fill)
		} else {
			stack![body, self.view_corner_overlay(corner)]
				.width(Length::Fill)
				.height(Length::Fill)
		};

		// Keep body content inside the rounded bottom corners.
		let body_container = container(body_stack)
			.width(Length::Fill)
			.height(Length::Fill)
			.padding(iced::Padding {
				top: 0.0,
				right: 0.0,
				bottom: 3.0,
				left: 0.0,
			});

		let chrome = PanelChrome {
			header_height: HEADER_HEIGHT,
			outline_width: PANEL_OUTLINE_WIDTH,
			outer_radius: constants::style::PANEL_RADIUS,
			inner_radius: constants::style::PANEL_INNER_RADIUS,
			exponent: constants::style::PANEL_SQUIRCLE_EXPONENT,
			panel_color: constants::style::PANEL_COLOR,
			header_color: colors::HEADER_BG,
			outline_color: colors::PANEL_OUTLINE_COLOR,
			separator_color: colors::HEADER_SEPARATOR,
			focus_color: colors::FOCUS_RING,
			focus,
		};

		squircle_panel_stack(
			chrome,
			container(column![header, body_container].spacing(0))
				.width(Length::Fill)
				.height(Length::Fill)
				.padding(PANEL_OUTLINE_WIDTH),
		)
	}

	fn view_corner_overlay<'a, M>(
		&self,
		corner: Element<'a, PanelSystemMessage<C, M>>,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		column![
			Space::new().height(Length::Fill),
			row![Space::new().width(Length::Fill), corner,],
		]
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}

	fn view_corner<'a, M>(&self, area_id: usize) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let active = matches!(
			&self.drag_state,
			DragState::CornerDrag {
				area_id: id,
				..
			} if *id == area_id
		);

		let color = if active { colors::CORNER_ACTIVE } else { colors::CORNER };

		let grip = Canvas::new(CornerGrip { color }).width(CORNER_SIZE).height(CORNER_SIZE);

		mouse_area(grip)
			.on_press(PanelSystemMessage::CornerDragStart(area_id))
			.on_move(PanelSystemMessage::MouseMove)
			.into()
	}

	fn view_move_source_preview<'a, M>(&self) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		container(Space::new())
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.35).into()),
				..Default::default()
			})
			.into()
	}

	fn view_full_move_preview<'a, M>(&self) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		container(
			container(
				row![
					constants::icons::icon(
						constants::icons::Icon::Transform,
						constants::style::ICON_SIZE,
						constants::style::TEXT_PRIMARY_COLOR
					),
					constants::widgets::ui_label("Swap Panels", constants::style::FONT_UI),
				]
				.spacing(6)
				.align_y(iced::Alignment::Center),
			)
			.padding([8, 14])
			.style(constants::widgets::hud(1.0)),
		)
		.width(Length::Fill)
		.height(Length::Fill)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.style(|_| container::Style {
			background: Some(colors::SPLIT_PREVIEW.into()),
			border: iced::Border {
				color: colors::SPLIT_PREVIEW_BORDER,
				width: 1.5,
				radius: constants::style::PANEL_INNER_RADIUS.into(),
			},
			..Default::default()
		})
		.into()
	}

	fn view_dock_preview<'a, M>(
		&self,
		direction: SplitDirection,
		ratio: f32,
		new_is_first: bool,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		const TOTAL: u16 = 10000;
		let first_portion = ((ratio * TOTAL as f32).round() as u16).clamp(1, TOTAL - 1);
		let second_portion = TOTAL - first_portion;

		let highlight_first = |active: bool| {
			let bg = if active {
				Some(colors::SPLIT_PREVIEW.into())
			} else {
				None
			};
			let border = if active {
				iced::Border {
					color: colors::SPLIT_PREVIEW_BORDER,
					width: 1.5,
					radius: constants::style::PANEL_INNER_RADIUS.into(),
				}
			} else {
				iced::Border::default()
			};
			container(Space::new()).style(move |_| container::Style {
				background: bg,
				border,
				..Default::default()
			})
		};

		let preview: Element<'_, PanelSystemMessage<C, M>> = match direction {
			SplitDirection::Horizontal => row![
				highlight_first(new_is_first)
					.width(Length::FillPortion(first_portion))
					.height(Length::Fill),
				highlight_first(!new_is_first)
					.width(Length::FillPortion(second_portion))
					.height(Length::Fill),
			]
			.width(Length::Fill)
			.height(Length::Fill)
			.into(),
			SplitDirection::Vertical => column![
				highlight_first(new_is_first)
					.width(Length::Fill)
					.height(Length::FillPortion(first_portion)),
				highlight_first(!new_is_first)
					.width(Length::Fill)
					.height(Length::FillPortion(second_portion)),
			]
			.width(Length::Fill)
			.height(Length::Fill)
			.into(),
		};

		container(preview).width(Length::Fill).height(Length::Fill).into()
	}

	fn view_split_preview<'a, M>(&self, action: CornerAction, ratio: f32) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		match action.direction() {
			Some(direction) => self.view_dock_preview(direction, ratio, false),
			None => Space::new().into(),
		}
	}

	fn view_panel_gutter<'a, M>(
		&self,
		direction: SplitDirection,
		path: Vec<usize>,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let (width, height): (Length, Length) = match direction {
			SplitDirection::Horizontal => (Length::Fixed(PANEL_GAP), Length::Fill),
			SplitDirection::Vertical => (Length::Fill, Length::Fixed(PANEL_GAP)),
		};

		// Hovering or dragging a split reveals a slim pill in the gutter.
		let hot = self.is_gutter_hot(&path);
		let indicator: Element<'a, PanelSystemMessage<C, M>> = if hot {
			let (pill_w, pill_h): (Length, Length) = match direction {
				SplitDirection::Horizontal => (Length::Fixed(2.0), Length::Fill),
				SplitDirection::Vertical => (Length::Fill, Length::Fixed(2.0)),
			};
			container(
				container(Space::new())
					.width(pill_w)
					.height(pill_h)
					.style(|_| container::Style {
						background: Some(colors::GUTTER_ACTIVE.into()),
						border: iced::Border {
							radius: 1.0.into(),
							..Default::default()
						},
						..Default::default()
					}),
			)
			.width(width)
			.height(height)
			.padding(match direction {
				SplitDirection::Horizontal => iced::Padding::from([12.0, 0.0]),
				SplitDirection::Vertical => iced::Padding::from([0.0, 12.0]),
			})
			.center_x(width)
			.center_y(height)
			.into()
		} else {
			Space::new().width(width).height(height).into()
		};

		stack![
			container(Space::new())
				.width(width)
				.height(height)
				.style(|_| container::Style {
					background: Some(colors::RESIZE_HANDLE.into()),
					..Default::default()
				}),
			indicator,
			self.view_resize_hit(direction, path, width, height),
		]
		.width(width)
		.height(height)
		.into()
	}

	fn view_resize_hit<'a, M>(
		&self,
		direction: SplitDirection,
		path: Vec<usize>,
		width: Length,
		height: Length,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		if matches!(width, Length::Fixed(w) if w <= 0.0) || matches!(height, Length::Fixed(h) if h <= 0.0) {
			return Space::new().width(width).height(height).into();
		}

		let is_any_resizing = matches!(&self.drag_state, DragState::Resizing { .. });
		let cursor = match direction {
			SplitDirection::Horizontal => mouse::Interaction::ResizingHorizontally,
			SplitDirection::Vertical => mouse::Interaction::ResizingVertically,
		};

		let hit = container(Space::new())
			.width(width)
			.height(height)
			.style(|_| container::Style {
				background: Some(Color::TRANSPARENT.into()),
				..Default::default()
			});

		let ma = mouse_area(hit)
			.interaction(cursor)
			.on_enter(PanelSystemMessage::GutterHover(path.clone(), true))
			.on_exit(PanelSystemMessage::GutterHover(path.clone(), false))
			.on_press(PanelSystemMessage::ResizeStart(path, direction))
			.on_release(PanelSystemMessage::ResizeEnd);

		if is_any_resizing {
			ma.into()
		} else {
			ma.on_move(PanelSystemMessage::MouseMove).into()
		}
	}

	// ==========================================================================
	// ツリー操作
	// ==========================================================================

	fn set_area_content(&mut self, area_id: usize, content: C) {
		Self::set_content_recursive(&mut self.root, area_id, content);
	}

	fn set_content_recursive(node: &mut DockNode<C>, area_id: usize, content: C) -> bool {
		match node {
			DockNode::Leaf(area) if area.id == area_id => {
				area.content = content;
				true
			}
			DockNode::Split { first, second, .. } => {
				Self::set_content_recursive(first, area_id, content.clone())
					|| Self::set_content_recursive(second, area_id, content)
			}
			_ => false,
		}
	}

	fn find_area_content(&self, area_id: usize) -> Option<C> {
		Self::find_content_recursive(&self.root, area_id)
	}

	fn find_content_recursive(node: &DockNode<C>, area_id: usize) -> Option<C> {
		match node {
			DockNode::Leaf(area) if area.id == area_id => Some(area.content.clone()),
			DockNode::Split { first, second, .. } => {
				Self::find_content_recursive(first, area_id).or_else(|| Self::find_content_recursive(second, area_id))
			}
			_ => None,
		}
	}

	fn area_has_sibling(&self, area_id: usize) -> bool {
		Self::can_join_recursive(&self.root, area_id)
	}

	fn can_join_recursive(node: &DockNode<C>, area_id: usize) -> bool {
		match node {
			DockNode::Split { first, second, .. } => {
				if Self::is_direct_leaf(first, area_id) && !second.is_empty() {
					return true;
				}
				if Self::is_direct_leaf(second, area_id) && !first.is_empty() {
					return true;
				}
				Self::can_join_recursive(first, area_id) || Self::can_join_recursive(second, area_id)
			}
			_ => false,
		}
	}

	/// ソースエリアをターゲットへサイズ付きで移動する
	fn move_area_into(
		&mut self,
		source_id: usize,
		target_id: usize,
		direction: SplitDirection,
		ratio: f32,
		new_is_first: bool,
	) {
		if source_id == target_id {
			return;
		}
		let Some(content) = self.find_area_content(source_id) else {
			return;
		};
		let Some(path) = self.area_path(target_id) else {
			return;
		};
		let available = available_along(self.bounds_at_path(&path), direction);
		let first_size = leading_size(ratio.clamp(0.15, 0.85), available);

		let new_id = self.next_area_id;
		self.next_area_id += 1;
		let new_area = Area::new(new_id, content);

		Self::split_area_recursive(&mut self.root, target_id, new_area, direction, first_size, new_is_first);
		self.join_area(source_id);
	}

	/// 2つのエリアをレイアウト上で入れ替える（全体移動）
	fn swap_areas(&mut self, a_id: usize, b_id: usize) {
		if a_id == b_id {
			return;
		}
		let Some(path_a) = self.area_path(a_id) else {
			return;
		};
		let Some(path_b) = self.area_path(b_id) else {
			return;
		};
		let Some(area_a) = Self::find_area_clone(&self.root, a_id) else {
			return;
		};
		let Some(area_b) = Self::find_area_clone(&self.root, b_id) else {
			return;
		};

		Self::set_leaf_at_path(&mut self.root, &path_a, area_b);
		Self::set_leaf_at_path(&mut self.root, &path_b, area_a);
	}

	fn find_area_clone(node: &DockNode<C>, area_id: usize) -> Option<Area<C>> {
		match node {
			DockNode::Leaf(area) if area.id == area_id => Some(area.clone()),
			DockNode::Split { first, second, .. } => {
				Self::find_area_clone(first, area_id).or_else(|| Self::find_area_clone(second, area_id))
			}
			_ => None,
		}
	}

	fn set_leaf_at_path(node: &mut DockNode<C>, path: &[usize], area: Area<C>) {
		if path.is_empty() {
			*node = DockNode::Leaf(area);
			return;
		}

		if let DockNode::Split { first, second, .. } = node {
			match path.first() {
				Some(0) => Self::set_leaf_at_path(first, &path[1..], area),
				Some(1) => Self::set_leaf_at_path(second, &path[1..], area),
				_ => {}
			}
		}
	}

	fn split_area(&mut self, area_id: usize, direction: SplitDirection, ratio: f32, new_is_first: bool) {
		let Some(content) = self.find_area_content(area_id) else {
			return;
		};
		let Some(path) = self.area_path(area_id) else {
			return;
		};
		let bounds = self.bounds_at_path(&path);
		let available = match direction {
			SplitDirection::Horizontal => bounds.width,
			SplitDirection::Vertical => bounds.height,
		};
		let available = (available - PANEL_GAP).max(0.0);
		let first_size = leading_size(ratio.clamp(0.15, 0.85), available);

		let new_id = self.next_area_id;
		self.next_area_id += 1;
		let new_area = Area::new(new_id, content);

		Self::split_area_recursive(&mut self.root, area_id, new_area, direction, first_size, new_is_first);
	}

	fn split_area_recursive(
		node: &mut DockNode<C>,
		area_id: usize,
		new_area: Area<C>,
		direction: SplitDirection,
		first_size: f32,
		new_is_first: bool,
	) -> bool {
		match node {
			DockNode::Leaf(area) if area.id == area_id => {
				let existing = std::mem::replace(node, DockNode::Empty);
				let new_leaf = DockNode::Leaf(new_area);
				*node = if new_is_first {
					DockNode::Split {
						direction,
						first_size,
						first: Box::new(new_leaf),
						second: Box::new(existing),
					}
				} else {
					DockNode::Split {
						direction,
						first_size,
						first: Box::new(existing),
						second: Box::new(new_leaf),
					}
				};
				true
			}
			DockNode::Split { first, second, .. } => {
				Self::split_area_recursive(first, area_id, new_area.clone(), direction, first_size, new_is_first)
					|| Self::split_area_recursive(second, area_id, new_area, direction, first_size, new_is_first)
			}
			_ => false,
		}
	}

	fn join_area(&mut self, area_id: usize) {
		Self::join_area_recursive(&mut self.root, area_id);
		Self::cleanup_recursive(&mut self.root);
	}

	/// 指定エリアを含む split を、兄弟側だけ残す形で潰す
	fn join_area_recursive(node: &mut DockNode<C>, area_id: usize) -> bool {
		match node {
			DockNode::Split { first, second, .. } => {
				if Self::is_direct_leaf(first, area_id) {
					*node = std::mem::replace(second.as_mut(), DockNode::Empty);
					return true;
				}
				if Self::is_direct_leaf(second, area_id) {
					*node = std::mem::replace(first.as_mut(), DockNode::Empty);
					return true;
				}
				Self::join_area_recursive(first, area_id) || Self::join_area_recursive(second, area_id)
			}
			_ => false,
		}
	}

	fn is_direct_leaf(node: &DockNode<C>, area_id: usize) -> bool {
		matches!(node, DockNode::Leaf(area) if area.id == area_id)
	}

	fn cleanup_recursive(node: &mut DockNode<C>) {
		if let DockNode::Split { first, second, .. } = node {
			Self::cleanup_recursive(first);
			Self::cleanup_recursive(second);

			if first.is_empty() && second.is_empty() {
				*node = DockNode::Empty;
			} else if first.is_empty() {
				*node = std::mem::take(second.as_mut());
			} else if second.is_empty() {
				*node = std::mem::take(first.as_mut());
			}
		}
	}

	fn get_first_size_at_path(&self, path: &[usize]) -> Option<f32> {
		Self::get_first_size_recursive(&self.root, path)
	}

	fn get_first_size_recursive(node: &DockNode<C>, path: &[usize]) -> Option<f32> {
		if path.is_empty() {
			if let DockNode::Split { first_size, .. } = node {
				return Some(*first_size);
			}
			return None;
		}

		if let DockNode::Split { first, second, .. } = node {
			match path.first() {
				Some(0) => Self::get_first_size_recursive(first, &path[1..]),
				Some(1) => Self::get_first_size_recursive(second, &path[1..]),
				_ => None,
			}
		} else {
			None
		}
	}

	fn set_first_size_at_path(&mut self, path: &[usize], new_size: f32) {
		Self::set_first_size_recursive(&mut self.root, path, new_size);
	}

	fn set_first_size_recursive(node: &mut DockNode<C>, path: &[usize], new_size: f32) {
		if path.is_empty() {
			if let DockNode::Split { first_size, .. } = node {
				*first_size = new_size;
			}
			return;
		}

		if let DockNode::Split { first, second, .. } = node {
			match path.first() {
				Some(0) => Self::set_first_size_recursive(first, &path[1..], new_size),
				Some(1) => Self::set_first_size_recursive(second, &path[1..], new_size),
				_ => {}
			}
		}
	}
}

impl<C: AreaKind> Default for PanelSystem<C> {
	fn default() -> Self {
		Self::new()
	}
}

fn panel_icon_svg_colored<'a, Message: 'a>(
	handle: svg::Handle,
	size: f32,
	color: Color,
	opacity: f32,
) -> Element<'a, Message> {
	constants::widgets::icon_slot(
		svg(handle)
			.width(size)
			.height(size)
			.opacity(opacity)
			.style(move |_theme, _status| svg::Style { color: Some(color) }),
		size,
	)
}

/// 中央ゾーン外側の座標を、中央=0.5・端=その方向の薄い分割になるよう倍率マップする
fn ratio_from_center(pos: f32, new_is_first: bool) -> f32 {
	const MIN_RATIO: f32 = 0.15;
	const MAX_RATIO: f32 = 0.85;
	let zone = MOVE_CENTER_ZONE;

	if new_is_first {
		// 上/左: [0, 0.5 - zone] → [MIN_RATIO, 0.5]
		// 端に近いほど first（移動先）が薄い
		let boundary = 0.5 - zone;
		let t = if boundary > f32::EPSILON {
			(pos / boundary).clamp(0.0, 1.0)
		} else {
			1.0
		};
		MIN_RATIO + t * (0.5 - MIN_RATIO)
	} else {
		// 下/右: [0.5 + zone, 1] → [0.5, MAX_RATIO]
		// 端に近いほど second（移動先）が薄い
		let boundary = 0.5 + zone;
		let t = if (1.0 - boundary) > f32::EPSILON {
			((pos - boundary) / (1.0 - boundary)).clamp(0.0, 1.0)
		} else {
			0.0
		};
		0.5 + t * (MAX_RATIO - 0.5)
	}
}

// =============================================================================
// レイアウトビルダー
// =============================================================================

/// レイアウトを簡単に構築するためのビルダー
pub struct LayoutBuilder<C: Clone + std::fmt::Debug> {
	next_area_id: usize,
	_marker: std::marker::PhantomData<C>,
}

impl<C: AreaKind> LayoutBuilder<C> {
	pub fn new() -> Self {
		Self {
			next_area_id: 0,
			_marker: std::marker::PhantomData,
		}
	}

	/// 単一エリアのリーフを作成
	pub fn area(&mut self, content: C) -> DockNode<C> {
		let id = self.next_area_id;
		self.next_area_id += 1;
		DockNode::Leaf(Area::new(id, content))
	}

	pub fn hsplit(first: DockNode<C>, second: DockNode<C>, ratio: f32) -> DockNode<C> {
		DockNode::Split {
			direction: SplitDirection::Horizontal,
			first_size: ratio,
			first: Box::new(first),
			second: Box::new(second),
		}
	}

	pub fn vsplit(first: DockNode<C>, second: DockNode<C>, ratio: f32) -> DockNode<C> {
		DockNode::Split {
			direction: SplitDirection::Vertical,
			first_size: ratio,
			first: Box::new(first),
			second: Box::new(second),
		}
	}

	pub fn next_area_id(&self) -> usize {
		self.next_area_id
	}
}

impl<C: AreaKind> Default for LayoutBuilder<C> {
	fn default() -> Self {
		Self::new()
	}
}
