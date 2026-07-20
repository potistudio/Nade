//! Blender 風エリアシステムのメイン実装
//!
//! 単一エディタのエリア分割・結合・種別切替・リサイズを提供します。

use iced::{
	Color, Element, Length, Point, Rectangle, Size,
	widget::{Space, button, column, container, mouse_area, pick_list, row, stack, text},
};

use crate::{
	AreaKind,
	consts::{CORNER_DRAG_THRESHOLD, CORNER_SIZE, HEADER_HEIGHT, RESIZE_HANDLE_SIZE, colors},
	container::Area,
	drag::{CornerAction, DragState},
	node::{DockNode, SplitDirection},
};

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
			}

			PanelSystemMessage::CornerDragStart(area_id) => {
				self.drag_state = DragState::CornerDrag {
					area_id,
					start_pos: self.last_mouse_pos,
					current_pos: self.last_mouse_pos,
					action: None,
					ratio: 0.5,
					target_area_id: None,
				};
			}

			PanelSystemMessage::MouseMove(pos) => {
				self.last_mouse_pos = pos;

				match self.drag_state.clone() {
					DragState::CornerDrag {
						area_id,
						start_pos,
						..
					} => {
						let (action, ratio, target_area_id) =
							self.corner_drag_update(area_id, start_pos, pos);
						self.drag_state = DragState::CornerDrag {
							area_id,
							start_pos,
							current_pos: pos,
							action,
							ratio,
							target_area_id,
						};
					}
					DragState::Resizing {
						path,
						direction,
						start_pos,
						start_ratio,
						..
					} => {
						let new_ratio =
							self.calculate_new_ratio(&path, start_pos, pos, start_ratio, direction);
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
					..
				} = self.drag_state.clone()
				{
					match action {
						CornerAction::Move => {
							if let Some(target) = target_area_id {
								self.swap_areas(area_id, target);
							}
						}
						CornerAction::SplitHorizontal | CornerAction::SplitVertical => {
							if let Some(direction) = action.direction() {
								self.split_area(area_id, direction, ratio);
							}
						}
					}
				}
				self.drag_state = DragState::None;
			}

			PanelSystemMessage::JoinArea(area_id) => {
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
	fn corner_drag_update(
		&self,
		source_id: usize,
		start_pos: Point,
		pos: Point,
	) -> (Option<CornerAction>, f32, Option<usize>) {
		// 他エリア上 → 移動（入れ替え）
		if let Some(target_id) = self.area_at_point(pos) {
			if target_id != source_id {
				return (Some(CornerAction::Move), 0.5, Some(target_id));
			}
		}

		let dx = pos.x - start_pos.x;
		let dy = pos.y - start_pos.y;
		let distance = (dx * dx + dy * dy).sqrt();

		// 角からエリア内側（左上方向）へ十分な距離 → 分割
		if distance <= CORNER_DRAG_THRESHOLD || (dx >= 0.0 && dy >= 0.0) {
			return (None, 0.5, None);
		}

		let action = if dx.abs() > dy.abs() {
			CornerAction::SplitHorizontal
		} else {
			CornerAction::SplitVertical
		};

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

		(Some(action), ratio, None)
	}

	fn area_at_point(&self, pos: Point) -> Option<usize> {
		Self::find_area_at_point(
			&self.root,
			pos,
			Rectangle::new(Point::ORIGIN, self.window_size),
		)
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
						bounds =
							Rectangle::new(bounds.position(), Size::new(bounds.width, first_height));
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
			.style(|_| container::Style {
				background: Some(colors::BACKGROUND.into()),
				..Default::default()
			});

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

			stack![base, overlay]
				.width(Length::Fill)
				.height(Length::Fill)
				.into()
		} else {
			mouse_area(base)
				.on_move(PanelSystemMessage::MouseMove)
				.into()
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
				let first_portion =
					((*ratio * TOTAL_PORTIONS as f32).round() as u16).clamp(1, TOTAL_PORTIONS - 1);
				let second_portion = TOTAL_PORTIONS - first_portion;

				let first_len = Length::FillPortion(first_portion);
				let second_len = Length::FillPortion(second_portion);
				let resize_path = path;

				match direction {
					SplitDirection::Horizontal => row![
						container(first_view).width(first_len).height(Length::Fill),
						self.view_resize_handle(*direction, resize_path),
						container(second_view)
							.width(second_len)
							.height(Length::Fill),
					]
					.into(),
					SplitDirection::Vertical => column![
						container(first_view).width(Length::Fill).height(first_len),
						self.view_resize_handle(*direction, resize_path),
						container(second_view)
							.width(Length::Fill)
							.height(second_len),
					]
					.into(),
				}
			}
		}
	}

	fn view_area<'a, F, M>(
		&'a self,
		area: &'a Area<C>,
		content_view: F,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		F: Fn(usize, &C) -> Element<'a, PanelSystemMessage<C, M>> + Copy,
		M: Clone + std::fmt::Debug + 'static,
	{
		let area_id = area.id;
		let can_join = self.area_has_sibling(area_id);

		let editor_picker = pick_list(C::all(), Some(area.content.clone()), move |content| {
			PanelSystemMessage::ChangeEditor(area_id, content)
		})
		.placeholder("Editor")
		.text_size(12)
		.padding([4, 8])
		.width(Length::Shrink);

		let join_button: Element<'_, PanelSystemMessage<C, M>> = if can_join {
			button(text("Join").size(11).color(colors::TEXT_SECONDARY))
				.padding([4, 8])
				.style(|_: &iced::Theme, status| button_style(status))
				.on_press(PanelSystemMessage::JoinArea(area_id))
				.into()
		} else {
			Space::new().width(0).into()
		};

		let header = container(
			row![
				editor_picker,
				Space::new().width(Length::Fill),
				join_button,
			]
			.spacing(6)
			.align_y(iced::Alignment::Center)
			.padding([0, 6]),
		)
		.width(Length::Fill)
		.height(HEADER_HEIGHT)
		.style(|_| container::Style {
			background: Some(colors::HEADER_BG.into()),
			border: iced::Border {
				color: colors::BORDER,
				width: 0.0,
				radius: 0.0.into(),
			},
			..Default::default()
		});

		let body = content_view(area.id, &area.content);

		let overlay_preview = self.drag_state.corner_preview().and_then(
			|(source_id, action, ratio, target_id)| match action {
				CornerAction::Move => {
					if target_id == Some(area_id) {
						Some(self.view_move_preview())
					} else if source_id == area_id {
						Some(self.view_move_source_preview())
					} else {
						None
					}
				}
				CornerAction::SplitHorizontal | CornerAction::SplitVertical
					if source_id == area_id =>
				{
					Some(self.view_split_preview(action, ratio))
				}
				_ => None,
			},
		);

		let corner = self.view_corner(area_id);

		let body_stack = if let Some(preview) = overlay_preview {
			stack![body, preview, self.view_corner_overlay(corner),]
				.width(Length::Fill)
				.height(Length::Fill)
		} else {
			stack![body, self.view_corner_overlay(corner),]
				.width(Length::Fill)
				.height(Length::Fill)
		};

		let body_container = container(body_stack)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(colors::PANEL_BG.into()),
				border: iced::Border {
					color: colors::BORDER,
					width: 1.0,
					radius: 0.0.into(),
				},
				..Default::default()
			});

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

		let color = if active {
			colors::CORNER_ACTIVE
		} else {
			colors::CORNER
		};

		// 右下の三角形っぽいコーナーウィジェット
		let widget = container(
			text("◢").size(14).color(color),
		)
		.width(CORNER_SIZE)
		.height(CORNER_SIZE)
		.center_x(CORNER_SIZE)
		.center_y(CORNER_SIZE);

		mouse_area(widget)
			.on_press(PanelSystemMessage::CornerDragStart(area_id))
			.on_move(PanelSystemMessage::MouseMove)
			.into()
	}

	fn view_move_preview<'a, M>(&self) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		container(
			container(text("Move here").size(14).color(colors::TEXT_SECONDARY))
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

	fn view_split_preview<'a, M>(
		&self,
		action: CornerAction,
		ratio: f32,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		const TOTAL: u16 = 10000;
		let first_portion = ((ratio * TOTAL as f32).round() as u16).clamp(1, TOTAL - 1);
		let second_portion = TOTAL - first_portion;

		// 新規エリア側（second）をハイライトし、分割線位置を示す
		let preview: Element<'_, PanelSystemMessage<C, M>> = match action {
			CornerAction::SplitHorizontal => row![
				container(Space::new())
					.width(Length::FillPortion(first_portion))
					.height(Length::Fill),
				container(Space::new())
					.width(Length::FillPortion(second_portion))
					.height(Length::Fill)
					.style(|_| container::Style {
						background: Some(colors::SPLIT_PREVIEW.into()),
						..Default::default()
					}),
			]
			.width(Length::Fill)
			.height(Length::Fill)
			.into(),
			CornerAction::SplitVertical => column![
				container(Space::new())
					.width(Length::Fill)
					.height(Length::FillPortion(first_portion)),
				container(Space::new())
					.width(Length::Fill)
					.height(Length::FillPortion(second_portion))
					.style(|_| container::Style {
						background: Some(colors::SPLIT_PREVIEW.into()),
						..Default::default()
					}),
			]
			.width(Length::Fill)
			.height(Length::Fill)
			.into(),
			CornerAction::Move => Space::new().into(),
		};

		container(preview)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
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
				path: p,
				direction: d,
				..
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
			.on_enter(PanelSystemMessage::ResizeHandleHover(Some((
				path_clone, direction,
			))))
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
			DockNode::Split { first, second, .. } => Self::find_content_recursive(first, area_id)
				.or_else(|| Self::find_content_recursive(second, area_id)),
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

	fn find_area_clone(node: &DockNode<C>, area_id: usize) -> Option<Area<C>> {
		match node {
			DockNode::Leaf(area) if area.id == area_id => Some(area.clone()),
			DockNode::Split { first, second, .. } => Self::find_area_clone(first, area_id)
				.or_else(|| Self::find_area_clone(second, area_id)),
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

	/// 2つのエリアをレイアウト上で入れ替える
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

	fn split_area(&mut self, area_id: usize, direction: SplitDirection, ratio: f32) {
		let Some(content) = self.find_area_content(area_id) else {
			return;
		};

		let new_id = self.next_area_id;
		self.next_area_id += 1;
		let new_area = Area::new(new_id, content);
		let ratio = ratio.clamp(0.15, 0.85);

		Self::split_area_recursive(&mut self.root, area_id, new_area, direction, ratio);
	}

	fn split_area_recursive(
		node: &mut DockNode<C>,
		area_id: usize,
		new_area: Area<C>,
		direction: SplitDirection,
		ratio: f32,
	) -> bool {
		match node {
			DockNode::Leaf(area) if area.id == area_id => {
				let existing = std::mem::replace(node, DockNode::Empty);
				*node = DockNode::Split {
					direction,
					ratio,
					first: Box::new(existing),
					second: Box::new(DockNode::Leaf(new_area)),
				};
				true
			}
			DockNode::Split { first, second, .. } => {
				Self::split_area_recursive(first, area_id, new_area.clone(), direction, ratio)
					|| Self::split_area_recursive(second, area_id, new_area, direction, ratio)
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

fn button_style(status: iced::widget::button::Status) -> iced::widget::button::Style {
	use iced::widget::button;
	let background = match status {
		button::Status::Hovered => Some(Color::from_rgb(0.25, 0.25, 0.28).into()),
		button::Status::Pressed => Some(colors::ACCENT.into()),
		_ => None,
	};
	button::Style {
		background,
		text_color: colors::TEXT_SECONDARY,
		border: iced::Border {
			radius: 3.0.into(),
			..Default::default()
		},
		..Default::default()
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
