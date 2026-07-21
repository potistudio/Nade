//! Blender 風エリアシステムのメイン実装
//!
//! 単一エディタのエリア分割・結合・種別切替・リサイズを提供します。

use std::time::Instant;

use iced::{
	Color, Element, Length, Point, Rectangle, Size, Vector,
	widget::{Space, button, column, container, float, mouse_area, row, rule, stack, svg, text},
};

use crate::{
	AreaKind,
	consts::{CORNER_DRAG_THRESHOLD, CORNER_SIZE, HEADER_HEIGHT, MOVE_CENTER_ZONE, RESIZE_HANDLE_SIZE, colors},
	container::Area,
	drag::{CornerAction, DragState},
	node::{DockNode, SplitDirection},
};

/// Editor-type dropdown open/close duration.
const EDITOR_MENU_ANIM_SECS: f32 = 0.14;
/// Slide distance (px) while the menu fades in.
const EDITOR_MENU_SLIDE_PX: f32 = 6.0;

#[derive(Debug, Clone)]
struct EditorMenuState {
	area_id: usize,
	/// 0.0 = fully closed, 1.0 = fully open
	progress: f32,
	opening: bool,
	last_tick: Instant,
}

fn ease_out_cubic(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	1.0 - (1.0 - t).powi(3)
}

/// Visual menu openness: ease-out toward open, and ease-out toward closed.
fn menu_anim_value(progress: f32, opening: bool) -> f32 {
	if opening {
		ease_out_cubic(progress)
	} else {
		// Progress falls 1→0; ease-out the close so it decelerates into fully closed.
		1.0 - ease_out_cubic(1.0 - progress)
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
	ResizeHandleHover(Option<(Vec<usize>, SplitDirection)>),
	WindowResized(Size),
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
	hover_resize_handle: Option<(Vec<usize>, SplitDirection)>,
	/// エディタ種別メニュー（開閉アニメーション付き）
	editor_menu: Option<EditorMenuState>,
}

impl<C: AreaKind> PanelSystem<C> {
	pub fn new() -> Self {
		Self {
			root: DockNode::Empty,
			next_area_id: 0,
			drag_state: DragState::None,
			last_mouse_pos: Point::ORIGIN,
			window_size: Size::new(800.0, 600.0),
			hover_resize_handle: None,
			editor_menu: None,
		}
	}

	/// Whether the editor menu open/close animation still needs ticks.
	pub fn is_editor_menu_animating(&self) -> bool {
		self.editor_menu
			.as_ref()
			.is_some_and(|menu| (menu.opening && menu.progress < 1.0) || (!menu.opening && menu.progress > 0.0))
	}

	fn open_editor_menu(&mut self, area_id: usize) {
		if let Some(menu) = &mut self.editor_menu
			&& menu.area_id == area_id
		{
			menu.opening = !menu.opening;
			menu.last_tick = Instant::now();
			return;
		}
		self.editor_menu = Some(EditorMenuState {
			area_id,
			progress: 0.0,
			opening: true,
			last_tick: Instant::now(),
		});
	}

	fn close_editor_menu(&mut self) {
		if let Some(menu) = &mut self.editor_menu {
			menu.opening = false;
			menu.last_tick = Instant::now();
		}
	}

	fn tick_editor_menu(&mut self) {
		let finished = {
			let Some(menu) = self.editor_menu.as_mut() else {
				return;
			};
			let now = Instant::now();
			let dt = now.duration_since(menu.last_tick).as_secs_f32().min(0.05);
			menu.last_tick = now;
			let delta = dt / EDITOR_MENU_ANIM_SECS;
			if menu.opening {
				menu.progress = (menu.progress + delta).min(1.0);
				false
			} else {
				menu.progress = (menu.progress - delta).max(0.0);
				menu.progress <= 0.0
			}
		};
		if finished {
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
						start_ratio,
						..
					} => {
						let new_ratio = self.calculate_new_ratio(&path, start_pos, pos, start_ratio, direction);
						self.set_ratio_at_path(&path, new_ratio);
						self.drag_state = DragState::Resizing {
							path,
							direction,
							start_pos,
							current_pos: pos,
							start_ratio,
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
				if let Some(ratio) = self.get_ratio_at_path(&path) {
					self.drag_state = DragState::Resizing {
						path,
						direction,
						start_pos: self.last_mouse_pos,
						current_pos: self.last_mouse_pos,
						start_ratio: ratio,
					};
				}
			}

			PanelSystemMessage::ResizeEnd => {
				self.drag_state = DragState::None;
			}

			PanelSystemMessage::AppMessage(_) => {}

			PanelSystemMessage::ResizeHandleHover(handle) => {
				if !matches!(self.drag_state, DragState::Resizing { .. }) {
					self.hover_resize_handle = handle;
				}
			}

			PanelSystemMessage::WindowResized(size) => {
				self.window_size = size;
			}
		}
	}

	fn calculate_new_ratio(
		&self,
		path: &[usize],
		start_pos: Point,
		current_pos: Point,
		start_ratio: f32,
		direction: SplitDirection,
	) -> f32 {
		let region = self.size_at_path(path);
		let size = match direction {
			SplitDirection::Horizontal => region.width.max(100.0),
			SplitDirection::Vertical => region.height.max(100.0),
		};
		let delta = match direction {
			SplitDirection::Horizontal => (current_pos.x - start_pos.x) / size,
			SplitDirection::Vertical => (current_pos.y - start_pos.y) / size,
		};
		(start_ratio + delta).clamp(0.15, 0.85)
	}

	fn size_at_path(&self, path: &[usize]) -> Size {
		let mut size = self.window_size;
		let mut node = &self.root;

		for &index in path {
			let DockNode::Split {
				direction,
				ratio,
				first,
				second,
			} = node
			else {
				break;
			};

			match direction {
				SplitDirection::Horizontal => {
					let first_width = size.width * *ratio;
					let second_width = size.width * (1.0 - *ratio);
					if index == 0 {
						size = Size::new(first_width, size.height);
						node = first;
					} else {
						size = Size::new(second_width, size.height);
						node = second;
					}
				}
				SplitDirection::Vertical => {
					let first_height = size.height * *ratio;
					let second_height = size.height * (1.0 - *ratio);
					if index == 0 {
						size = Size::new(size.width, first_height);
						node = first;
					} else {
						size = Size::new(size.width, second_height);
						node = second;
					}
				}
			}
		}

		size
	}

	/// コーナードラッグ中の操作（分割 or 他エリアへの移動）を算出する
	fn corner_drag_update(&self, source_id: usize, start_pos: Point, pos: Point) -> CornerDragUpdate {
		// 他エリア上 → 中央基準の移動（中央=全体 / 外側=その方向へ分割）
		if let Some(target_id) = self.area_at_point(pos) {
			if target_id != source_id {
				if let Some(drop) = self.drop_layout_at(target_id, pos) {
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
			}
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
				ratio,
				first,
				second,
			} => {
				let (first_bounds, second_bounds) = match direction {
					SplitDirection::Horizontal => {
						let first_width = bounds.width * *ratio;
						(
							Rectangle::new(bounds.position(), Size::new(first_width, bounds.height)),
							Rectangle::new(
								Point::new(bounds.x + first_width, bounds.y),
								Size::new(bounds.width * (1.0 - *ratio), bounds.height),
							),
						)
					}
					SplitDirection::Vertical => {
						let first_height = bounds.height * *ratio;
						(
							Rectangle::new(bounds.position(), Size::new(bounds.width, first_height)),
							Rectangle::new(
								Point::new(bounds.x, bounds.y + first_height),
								Size::new(bounds.width, bounds.height * (1.0 - *ratio)),
							),
						)
					}
				};
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
				ratio,
				first,
				second,
			} = node
			else {
				break;
			};

			match direction {
				SplitDirection::Horizontal => {
					let first_width = bounds.width * *ratio;
					let second_width = bounds.width * (1.0 - *ratio);
					if index == 0 {
						bounds = Rectangle::new(bounds.position(), Size::new(first_width, bounds.height));
						node = first;
					} else {
						bounds = Rectangle::new(
							Point::new(bounds.x + first_width, bounds.y),
							Size::new(second_width, bounds.height),
						);
						node = second;
					}
				}
				SplitDirection::Vertical => {
					let first_height = bounds.height * *ratio;
					let second_height = bounds.height * (1.0 - *ratio);
					if index == 0 {
						bounds = Rectangle::new(bounds.position(), Size::new(bounds.width, first_height));
						node = first;
					} else {
						bounds = Rectangle::new(
							Point::new(bounds.x, bounds.y + first_height),
							Size::new(bounds.width, second_height),
						);
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

		// ドラッグ中は全面オーバーレイで他パネル上でも追従できるようにする
		if matches!(
			self.drag_state,
			DragState::CornerDrag { .. } | DragState::Resizing { .. }
		) {
			let release = match &self.drag_state {
				DragState::CornerDrag { .. } => PanelSystemMessage::CornerDragEnd,
				_ => PanelSystemMessage::ResizeEnd,
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
			.on_move(PanelSystemMessage::MouseMove)
			.on_release(release);

			stack![base, overlay].width(Length::Fill).height(Length::Fill).into()
		} else {
			mouse_area(base).on_move(PanelSystemMessage::MouseMove).into()
		}
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
				ratio,
				first,
				second,
			} => {
				let mut first_path = path.clone();
				first_path.push(0);
				let mut second_path = path.clone();
				second_path.push(1);

				let first_view = self.view_node(first, first_path, content_view);
				let second_view = self.view_node(second, second_path, content_view);

				const TOTAL_PORTIONS: u16 = 10000;
				let first_portion = ((*ratio * TOTAL_PORTIONS as f32).round() as u16).clamp(1, TOTAL_PORTIONS - 1);
				let second_portion = TOTAL_PORTIONS - first_portion;

				let first_len = Length::FillPortion(first_portion);
				let second_len = Length::FillPortion(second_portion);
				let resize_path = path;

				match direction {
					SplitDirection::Horizontal => row![
						container(first_view).width(first_len).height(Length::Fill),
						self.view_resize_handle(*direction, resize_path),
						container(second_view).width(second_len).height(Length::Fill),
					]
					.into(),
					SplitDirection::Vertical => column![
						container(first_view).width(Length::Fill).height(first_len),
						self.view_resize_handle(*direction, resize_path),
						container(second_view).width(Length::Fill).height(second_len),
					]
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
		let area_id = area.id;
		let can_join = self.area_has_sibling(area_id);
		let menu_progress = self
			.editor_menu
			.as_ref()
			.filter(|menu| menu.area_id == area_id)
			.map(|menu| menu_anim_value(menu.progress, menu.opening));
		let menu_open = menu_progress.is_some_and(|p| p > 0.001);
		let anim = menu_progress.unwrap_or(0.0);

		// Blender-style editor type chip: [icon] [▾]
		const ICON_SIZE: f32 = 14.0;
		const CHEVRON_SIZE: f32 = 7.0;
		let editor_picker = button(constants::widgets::button_body(
			row![
				constants::widgets::icon_slot(panel_icon_svg(area.content.icon(), ICON_SIZE), ICON_SIZE),
				constants::widgets::icon_slot(panel_icon_svg(chevron_down_handle(), CHEVRON_SIZE), CHEVRON_SIZE),
			]
			.spacing(3)
			.align_y(iced::Alignment::Center),
		))
		.padding(iced::Padding {
			top: 0.0,
			right: 4.0,
			bottom: 0.0,
			left: 3.0,
		})
		.width(Length::Shrink)
		.height(18.0)
		.style(|theme, status| constants::widgets::button_editor_type(theme, status))
		.on_press(PanelSystemMessage::ToggleEditorMenu(area_id));

		let join_button: Element<'_, PanelSystemMessage<C, M>> = if can_join {
			button(constants::widgets::button_body(
				constants::widgets::ui_label("Join", constants::style::FONT_UI)
					.color(constants::style::TEXT_SECONDARY_COLOR),
			))
			.padding(constants::style::PAD_BUTTON)
			.width(Length::Shrink)
			.height(18.0)
			.style(|theme, status| constants::widgets::button_ghost(theme, status))
			.on_press(PanelSystemMessage::JoinArea(area_id))
			.into()
		} else {
			Space::new().width(0).into()
		};

		let header = container(
			row![editor_picker, Space::new().width(Length::Fill), join_button,]
				.spacing(constants::style::SPACE_2)
				.align_y(iced::Alignment::Center)
				.height(Length::Fill)
				.padding(iced::Padding {
					top: 0.0,
					right: constants::style::SPACE_2,
					bottom: 0.0,
					left: constants::style::SPACE_2,
				}),
		)
		.width(Length::Fill)
		.height(HEADER_HEIGHT)
		.align_y(iced::Alignment::Center)
		.style(constants::widgets::panel_header);

		let body = content_view(area.id, &area.content);

		let editor_menu: Element<'_, PanelSystemMessage<C, M>> = if menu_open {
			const MENU_ICON: f32 = 12.0;
			const COL_WIDTH: f32 = 104.0;
			const ROW_HEIGHT: f32 = 18.0;

			let columns = row(C::menu_columns()
				.into_iter()
				.map(|(category, kinds)| {
					let header = column![
						constants::widgets::ui_label(category, constants::style::FONT_TINY)
							.color(constants::style::TEXT_SECONDARY_COLOR.scale_alpha(anim)),
						rule::horizontal(1).style(move |_theme| rule::Style {
							color: constants::style::BORDER_SUBTLE_COLOR.scale_alpha(anim),
							radius: 0.0.into(),
							fill_mode: rule::FillMode::Full,
							snap: true,
						}),
					]
					.spacing(2)
					.width(Length::Fill);

					let items = column(
						kinds
							.into_iter()
							.map(|kind| {
								let selected = kind == area.content;
								let icon_color = if selected {
									constants::style::TEXT_PRIMARY_COLOR_INVERTED.scale_alpha(anim)
								} else {
									constants::style::TEXT_PRIMARY_COLOR.scale_alpha(anim)
								};
								let label_color = if selected {
									constants::style::TEXT_PRIMARY_COLOR_INVERTED.scale_alpha(anim)
								} else {
									constants::style::TEXT_PRIMARY_COLOR.scale_alpha(anim)
								};

								let row_content = constants::widgets::icon_label_row(
									panel_icon_svg_colored(kind.icon(), MENU_ICON, icon_color, anim),
									kind.label(),
									constants::style::FONT_UI,
									label_color,
									MENU_ICON,
									4.0,
								);

								button(constants::widgets::button_body(row_content))
									.padding(iced::Padding {
										top: 0.0,
										right: 4.0,
										bottom: 0.0,
										left: 2.0,
									})
									.width(Length::Fill)
									.height(ROW_HEIGHT)
									.style(constants::widgets::button_menu_item_faded(selected, anim))
									.on_press(PanelSystemMessage::ChangeEditor(area_id, kind))
									.into()
							})
							.collect::<Vec<_>>(),
					)
					.spacing(0);

					container(column![header, items].spacing(2).height(Length::Shrink))
						.width(COL_WIDTH)
						.height(Length::Shrink)
						.padding(iced::Padding {
							top: 4.0,
							right: 3.0,
							bottom: 4.0,
							left: 3.0,
						})
						.into()
				})
				.collect::<Vec<_>>())
			.spacing(0)
			.align_y(iced::Alignment::Start)
			.height(Length::Shrink);

			let slide = (1.0 - anim) * -EDITOR_MENU_SLIDE_PX;

			float(
				container(columns)
					.height(Length::Shrink)
					.style(move |theme| constants::widgets::editor_menu_panel_faded(theme, anim)),
			)
			.translate(move |_bounds, _viewport| Vector::new(0.0, slide))
			.into()
		} else {
			Space::new().width(0).height(0).into()
		};

		let overlay_preview = self.drag_state.corner_preview().and_then(
			|(source_id, action, ratio, target_id, direction, new_is_first)| match action {
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

		// Keep the menu content-sized — Fill height in the stack was stretching the
		// selected row to the full panel height.
		let menu_overlay = column![
			Space::new().height(HEADER_HEIGHT),
			row![editor_menu, Space::new().width(Length::Fill)].height(Length::Shrink),
			Space::new().height(Length::Fill),
		]
		.width(Length::Fill)
		.height(Length::Fill);

		let body_stack = if let Some(preview) = overlay_preview {
			stack![body, preview, self.view_corner_overlay(corner), menu_overlay,]
				.width(Length::Fill)
				.height(Length::Fill)
		} else {
			stack![body, self.view_corner_overlay(corner), menu_overlay,]
				.width(Length::Fill)
				.height(Length::Fill)
		};

		let body_container = container(body_stack)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(constants::widgets::panel_body);

		column![header, body_container].spacing(0).into()
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

		// 右下の三角形っぽいコーナーウィジェット
		let widget = container(text("◢").size(14).color(color))
			.width(CORNER_SIZE)
			.height(CORNER_SIZE)
			.center_x(CORNER_SIZE)
			.center_y(CORNER_SIZE);

		mouse_area(widget)
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
			container(text("Move").size(14).color(colors::TEXT_SECONDARY))
				.padding([8, 14])
				.style(|_| container::Style {
					background: Some(colors::HEADER_BG.into()),
					border: iced::Border {
						color: colors::ACCENT,
						width: 1.0,
						radius: 4.0.into(),
					},
					..Default::default()
				}),
		)
		.width(Length::Fill)
		.height(Length::Fill)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.style(|_| container::Style {
			background: Some(colors::SPLIT_PREVIEW.into()),
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
			container(Space::new()).style(move |_| container::Style {
				background: bg,
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

	fn view_resize_handle<'a, M>(
		&self,
		direction: SplitDirection,
		path: Vec<usize>,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let is_any_resizing = matches!(&self.drag_state, DragState::Resizing { .. });

		let is_active = match &self.drag_state {
			DragState::Resizing {
				path: p, direction: d, ..
			} => *p == path && *d == direction,
			_ => false,
		};

		let is_hovered = if is_any_resizing {
			false
		} else {
			self.hover_resize_handle
				.as_ref()
				.map(|(p, d)| *p == path && *d == direction)
				.unwrap_or(false)
		};

		let (width, height): (Length, Length) = match direction {
			SplitDirection::Horizontal => (Length::Fixed(RESIZE_HANDLE_SIZE), Length::Fill),
			SplitDirection::Vertical => (Length::Fill, Length::Fixed(RESIZE_HANDLE_SIZE)),
		};

		let handle_size = if is_active || is_hovered { 3.0 } else { 1.0 };

		let bg_color = if is_active {
			colors::RESIZE_HANDLE_ACTIVE
		} else if is_hovered {
			colors::RESIZE_HANDLE_HOVER
		} else {
			colors::RESIZE_HANDLE
		};

		let inner_handle = match direction {
			SplitDirection::Horizontal => container(
				container(Space::new())
					.width(Length::Fixed(handle_size))
					.height(Length::Fill)
					.style(move |_| container::Style {
						background: Some(bg_color.into()),
						..Default::default()
					}),
			)
			.width(width)
			.height(height)
			.center_x(width)
			.center_y(height),
			SplitDirection::Vertical => container(
				container(Space::new())
					.width(Length::Fill)
					.height(Length::Fixed(handle_size))
					.style(move |_| container::Style {
						background: Some(bg_color.into()),
						..Default::default()
					}),
			)
			.width(width)
			.height(height)
			.center_x(width)
			.center_y(height),
		};

		let path_clone = path.clone();
		let path_for_press = path;

		let ma = mouse_area(inner_handle)
			.on_press(PanelSystemMessage::ResizeStart(path_for_press, direction))
			.on_release(PanelSystemMessage::ResizeEnd)
			.on_enter(PanelSystemMessage::ResizeHandleHover(Some((path_clone, direction))))
			.on_exit(PanelSystemMessage::ResizeHandleHover(None));

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

		let new_id = self.next_area_id;
		self.next_area_id += 1;
		let new_area = Area::new(new_id, content);
		let ratio = ratio.clamp(0.15, 0.85);

		Self::split_area_recursive(&mut self.root, target_id, new_area, direction, ratio, new_is_first);
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

		let new_id = self.next_area_id;
		self.next_area_id += 1;
		let new_area = Area::new(new_id, content);
		let ratio = ratio.clamp(0.15, 0.85);

		Self::split_area_recursive(&mut self.root, area_id, new_area, direction, ratio, new_is_first);
	}

	fn split_area_recursive(
		node: &mut DockNode<C>,
		area_id: usize,
		new_area: Area<C>,
		direction: SplitDirection,
		ratio: f32,
		new_is_first: bool,
	) -> bool {
		match node {
			DockNode::Leaf(area) if area.id == area_id => {
				let existing = std::mem::replace(node, DockNode::Empty);
				let new_leaf = DockNode::Leaf(new_area);
				*node = if new_is_first {
					DockNode::Split {
						direction,
						ratio,
						first: Box::new(new_leaf),
						second: Box::new(existing),
					}
				} else {
					DockNode::Split {
						direction,
						ratio,
						first: Box::new(existing),
						second: Box::new(new_leaf),
					}
				};
				true
			}
			DockNode::Split { first, second, .. } => {
				Self::split_area_recursive(first, area_id, new_area.clone(), direction, ratio, new_is_first)
					|| Self::split_area_recursive(second, area_id, new_area, direction, ratio, new_is_first)
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
		match node {
			DockNode::Split { first, second, .. } => {
				Self::cleanup_recursive(first);
				Self::cleanup_recursive(second);

				if first.is_empty() && second.is_empty() {
					*node = DockNode::Empty;
				} else if first.is_empty() {
					*node = std::mem::replace(second.as_mut(), DockNode::Empty);
				} else if second.is_empty() {
					*node = std::mem::replace(first.as_mut(), DockNode::Empty);
				}
			}
			_ => {}
		}
	}

	fn get_ratio_at_path(&self, path: &[usize]) -> Option<f32> {
		Self::get_ratio_recursive(&self.root, path)
	}

	fn get_ratio_recursive(node: &DockNode<C>, path: &[usize]) -> Option<f32> {
		if path.is_empty() {
			if let DockNode::Split { ratio, .. } = node {
				return Some(*ratio);
			}
			return None;
		}

		if let DockNode::Split { first, second, .. } = node {
			match path.first() {
				Some(0) => Self::get_ratio_recursive(first, &path[1..]),
				Some(1) => Self::get_ratio_recursive(second, &path[1..]),
				_ => None,
			}
		} else {
			None
		}
	}

	fn set_ratio_at_path(&mut self, path: &[usize], new_ratio: f32) {
		Self::set_ratio_recursive(&mut self.root, path, new_ratio);
	}

	fn set_ratio_recursive(node: &mut DockNode<C>, path: &[usize], new_ratio: f32) {
		if path.is_empty() {
			if let DockNode::Split { ratio, .. } = node {
				*ratio = new_ratio;
			}
			return;
		}

		if let DockNode::Split { first, second, .. } = node {
			match path.first() {
				Some(0) => Self::set_ratio_recursive(first, &path[1..], new_ratio),
				Some(1) => Self::set_ratio_recursive(second, &path[1..], new_ratio),
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

fn chevron_down_handle() -> svg::Handle {
	svg::Handle::from_memory(include_bytes!("../../../assets/icons/panels/chevron_down.svg").as_slice())
}

fn panel_icon_svg<'a, Message: 'a>(handle: svg::Handle, size: f32) -> Element<'a, Message> {
	panel_icon_svg_colored(handle, size, constants::style::TEXT_PRIMARY_COLOR, 1.0)
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
			ratio,
			first: Box::new(first),
			second: Box::new(second),
		}
	}

	pub fn vsplit(first: DockNode<C>, second: DockNode<C>, ratio: f32) -> DockNode<C> {
		DockNode::Split {
			direction: SplitDirection::Vertical,
			ratio,
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
