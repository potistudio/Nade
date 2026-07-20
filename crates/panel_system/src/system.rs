//! パネルシステムのメイン実装
//!
//! パネルの管理、レイアウト、インタラクションを提供します。

use iced::{
	Color, Element, Length, Point, Size,
	widget::{Column, Row, Space, button, column, container, mouse_area, row, stack, text},
};

use crate::{
	consts::{DRAG_THRESHOLD, DROP_ZONE_SIZE, RESIZE_HANDLE_SIZE, TAB_HEIGHT, colors},
	container::{Panel, TabContainer},
	drag::{DragState, DropPosition, DropZone},
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
	TabClicked(usize, usize),
	TabDragStart(usize, usize),
	MouseMove(Point),
	TabDragEnd,
	TabClose(usize, usize),
	ResizeStart(Vec<usize>, SplitDirection),
	ResizeEnd,
	DropZoneHover(Option<DropZone>),
	ResizeHandleHover(Option<(Vec<usize>, SplitDirection)>),
	WindowResized(Size),
	/// パネルコンテンツ識別子
	Content(C),
	/// アプリケーション固有のメッセージ
	AppMessage(M),
}

// =============================================================================
// パネルシステム
// =============================================================================

/// パネルシステム
#[derive(Debug)]
pub struct PanelSystem<C: Clone + std::fmt::Debug + PartialEq + Eq + 'static> {
	root: DockNode<C>,
	next_panel_id: usize,
	next_container_id: usize,
	drag_state: DragState,
	hover_drop_zone: Option<DropZone>,
	last_mouse_pos: Point,
	window_size: Size,
	hover_resize_handle: Option<(Vec<usize>, SplitDirection)>,
}

impl<C: Clone + std::fmt::Debug + PartialEq + Eq + 'static> PanelSystem<C> {
	pub fn new() -> Self {
		Self {
			root: DockNode::Leaf(TabContainer::new(0)),
			next_panel_id: 0,
			next_container_id: 1,
			drag_state: DragState::None,
			hover_drop_zone: None,
			last_mouse_pos: Point::ORIGIN,
			window_size: Size::new(800.0, 600.0),
			hover_resize_handle: None,
		}
	}

	/// パネルを追加
	pub fn add_panel(&mut self, title: &str, content: C) {
		let panel_id = self.next_panel_id;
		self.next_panel_id += 1;

		let panel = Panel::new(panel_id, title, content);

		if let DockNode::Leaf(container) = &mut self.root {
			container.add_panel(panel);
		}
	}

	/// 指定したコンテンツで最初のレイアウトを設定
	pub fn with_layout(mut self, layout: DockNode<C>) -> Self {
		self.root = layout;
		self
	}

	/// `LayoutBuilder` で採番済みの次IDを同期する
	pub fn with_ids(mut self, next_panel_id: usize, next_container_id: usize) -> Self {
		self.next_panel_id = next_panel_id;
		self.next_container_id = next_container_id;
		self
	}

	/// ルートノードへの参照を取得
	pub fn root(&self) -> &DockNode<C> {
		&self.root
	}

	/// ルートノードへの可変参照を取得
	pub fn root_mut(&mut self) -> &mut DockNode<C> {
		&mut self.root
	}

	/// メッセージを処理
	pub fn update<M>(&mut self, message: PanelSystemMessage<C, M>)
	where
		M: Clone + std::fmt::Debug,
	{
		match message {
			PanelSystemMessage::TabClicked(container_id, tab_index) => {
				self.set_active_tab(container_id, tab_index);
			}

			PanelSystemMessage::TabDragStart(container_id, tab_index) => {
				if let Some(panel_id) = self.get_panel_id_at(container_id, tab_index) {
					self.drag_state = DragState::PendingDrag {
						source_container: container_id,
						panel_id,
						tab_index,
						start_pos: self.last_mouse_pos,
					};
				}
			}

			PanelSystemMessage::MouseMove(pos) => {
				self.last_mouse_pos = pos;

				match self.drag_state.clone() {
					DragState::PendingDrag {
						source_container,
						panel_id,
						start_pos,
						..
					} => {
						let distance =
							((pos.x - start_pos.x).powi(2) + (pos.y - start_pos.y).powi(2)).sqrt();
						if distance > DRAG_THRESHOLD {
							self.drag_state = DragState::DraggingTab {
								source_container,
								panel_id,
								start_pos,
								current_pos: pos,
							};
						}
					}
					DragState::DraggingTab {
						source_container,
						panel_id,
						start_pos,
						..
					} => {
						self.drag_state = DragState::DraggingTab {
							source_container,
							panel_id,
							start_pos,
							current_pos: pos,
						};
					}
					DragState::Resizing {
						path,
						direction,
						start_pos,
						start_ratio,
						..
					} => {
						let new_ratio = self.calculate_new_ratio(
							&path,
							start_pos,
							pos,
							start_ratio,
							direction,
						);
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

			PanelSystemMessage::TabDragEnd => {
				match &self.drag_state {
					DragState::PendingDrag {
						source_container,
						tab_index,
						..
					} => {
						self.set_active_tab(*source_container, *tab_index);
					}
					DragState::DraggingTab {
						source_container,
						panel_id,
						..
					} => {
						let source = *source_container;
						let panel = *panel_id;

						if let Some(drop_zone) = self.hover_drop_zone.take() {
							self.handle_drop(source, panel, drop_zone);
						}
					}
					_ => {}
				}
				self.drag_state = DragState::None;
				self.hover_drop_zone = None;
			}

			PanelSystemMessage::TabClose(container_id, tab_index) => {
				if let Some(panel_id) = self.get_panel_id_at(container_id, tab_index) {
					self.remove_panel_from_container(container_id, panel_id);
					self.cleanup_empty_nodes();
				}
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

			PanelSystemMessage::Content(_) => {}

			PanelSystemMessage::AppMessage(_) => {}

			PanelSystemMessage::DropZoneHover(zone) => {
				self.hover_drop_zone = zone;
			}

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

	/// パスで指すノードの領域サイズを、ウィンドウサイズと分割比から算出する
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

	/// ビューを生成
	pub fn view<'a, F, M>(&'a self, content_view: F) -> Element<'a, PanelSystemMessage<C, M>>
	where
		F: Fn(usize, &C) -> Element<'a, PanelSystemMessage<C, M>> + Copy,
		M: Clone + std::fmt::Debug + 'static,
		C: 'a,
	{
		let main_content = self.view_node(&self.root, vec![], content_view);

		if let DragState::DraggingTab {
			panel_id,
			current_pos,
			..
		} = &self.drag_state
		{
			if let Some(panel) = self.find_panel(*panel_id) {
				let floating = self.view_floating_panel(panel, *current_pos);
				let overlay = self.view_drop_zone_overlay();

				let main_with_mouse = mouse_area(
					container(main_content)
						.width(Length::Fill)
						.height(Length::Fill)
						.style(|_| container::Style {
							background: Some(colors::BACKGROUND.into()),
							..Default::default()
						}),
				)
				.on_move(PanelSystemMessage::MouseMove)
				.on_release(PanelSystemMessage::TabDragEnd);

				stack![main_with_mouse, overlay, floating,]
					.width(Length::Fill)
					.height(Length::Fill)
					.into()
			} else {
				self.view_main_container(main_content)
			}
		} else {
			self.view_main_container(main_content)
		}
	}

	fn view_main_container<'a, M>(
		&self,
		content: Element<'a, PanelSystemMessage<C, M>>,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		if let DragState::Resizing { .. } = &self.drag_state {
			mouse_area(
				container(content)
					.width(Length::Fill)
					.height(Length::Fill)
					.style(|_| container::Style {
						background: Some(colors::BACKGROUND.into()),
						..Default::default()
					}),
			)
			.on_move(PanelSystemMessage::MouseMove)
			.on_release(PanelSystemMessage::ResizeEnd)
			.into()
		} else {
			mouse_area(
				container(content)
					.width(Length::Fill)
					.height(Length::Fill)
					.style(|_| container::Style {
						background: Some(colors::BACKGROUND.into()),
						..Default::default()
					}),
			)
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

			DockNode::Leaf(container) => self.view_tab_container(container, content_view),

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

	fn view_tab_container<'a, F, M>(
		&'a self,
		tab_container: &'a TabContainer<C>,
		content_view: F,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		F: Fn(usize, &C) -> Element<'a, PanelSystemMessage<C, M>> + Copy,
		M: Clone + std::fmt::Debug + 'static,
	{
		let container_id = tab_container.id;
		let dragging_panel = self.drag_state.dragging_panel_id();

		let tabs: Vec<Element<PanelSystemMessage<C, M>>> = tab_container
			.panels
			.iter()
			.enumerate()
			.map(|(index, panel)| {
				let is_active = index == tab_container.active_tab;
				let is_being_dragged = dragging_panel == Some(panel.id);
				self.view_tab(container_id, index, panel, is_active, is_being_dragged)
			})
			.collect();

		let tab_bar = Row::with_children(tabs).spacing(1).padding([0, 4]);

		let tab_bar_container = container(tab_bar)
			.width(Length::Fill)
			.height(TAB_HEIGHT)
			.style(|_| container::Style {
				background: Some(colors::TAB_BAR_BG.into()),
				border: iced::Border {
					color: colors::BORDER,
					width: 0.0,
					radius: 0.0.into(),
				},
				..Default::default()
			});

		let content = if let Some(panel) = tab_container.get_active_panel() {
			content_view(panel.id, &panel.content)
		} else {
			self.view_empty_content()
		};

		let content_with_zones = if self.drag_state.is_dragging() {
			self.view_content_with_drop_zones(container_id, content)
		} else {
			content
		};

		let content_container = container(content_with_zones)
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

		column![tab_bar_container, content_container]
			.spacing(0)
			.into()
	}

	fn view_tab<'a, M>(
		&self,
		container_id: usize,
		index: usize,
		panel: &Panel<C>,
		is_active: bool,
		is_being_dragged: bool,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let bg_color = if is_active {
			colors::TAB_ACTIVE
		} else {
			colors::TAB_BG
		};

		let opacity = if is_being_dragged { 0.4 } else { 1.0 };

		let title = panel.title.clone();

		let text_color = if is_active {
			Color {
				a: opacity,
				..colors::TEXT_PRIMARY
			}
		} else {
			Color {
				a: opacity,
				..colors::TEXT_SECONDARY
			}
		};

		let close_color = Color {
			a: opacity,
			..colors::TEXT_SECONDARY
		};

		let tab_content = row![
			text(title).size(12).color(text_color),
			Space::new().width(Length::Fill),
			button(text("×").size(12).color(close_color))
				.padding([0, 4])
				.style(|_, _| button::Style {
					background: None,
					text_color: colors::TEXT_SECONDARY,
					..Default::default()
				})
				.on_press(PanelSystemMessage::TabClose(container_id, index)),
		]
		.spacing(4)
		.align_y(iced::Alignment::Center)
		.padding([6, 10]);

		let tab_button = container(tab_content).style(move |_| {
			let bg = Color {
				a: opacity,
				..bg_color
			};
			container::Style {
				background: Some(bg.into()),
				border: iced::Border {
					color: if is_active {
						colors::ACCENT
					} else {
						Color::TRANSPARENT
					},
					width: 0.0,
					radius: iced::border::Radius::new(4.0)
						.top_left(4.0)
						.top_right(4.0)
						.bottom_left(0.0)
						.bottom_right(0.0),
				},
				..Default::default()
			}
		});

		mouse_area(tab_button)
			.on_press(PanelSystemMessage::TabDragStart(container_id, index))
			.on_move(PanelSystemMessage::MouseMove)
			.on_release(PanelSystemMessage::TabDragEnd)
			.into()
	}

	fn view_floating_panel<'a, M>(
		&self,
		panel: &Panel<C>,
		pos: Point,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let title = panel.title.clone();

		let floating_content =
			container(row![text(title).size(12).color(colors::TEXT_PRIMARY),].padding([8, 12]))
				.style(|_| container::Style {
					background: Some(colors::FLOATING_PANEL_BG.into()),
					border: iced::Border {
						color: colors::ACCENT,
						width: 2.0,
						radius: 6.0.into(),
					},
					shadow: iced::Shadow {
						color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
						offset: iced::Vector::new(4.0, 4.0),
						blur_radius: 10.0,
					},
					..Default::default()
				});

		Column::new()
			.push(Space::new().height(pos.y.max(0.0)))
			.push(
				Row::new()
					.push(Space::new().width(pos.x.max(0.0)))
					.push(floating_content),
			)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn view_drop_zone_overlay<'a, M>(&self) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		if let Some(drop_zone) = &self.hover_drop_zone {
			let position_text = match drop_zone.position {
				DropPosition::Center => "タブとして追加",
				DropPosition::Left => "← 左に配置",
				DropPosition::Right => "右に配置 →",
				DropPosition::Top => "↑ 上に配置",
				DropPosition::Bottom => "下に配置 ↓",
			};

			container(
				container(text(position_text).size(16).color(colors::TEXT_PRIMARY))
					.padding([12, 20])
					.style(|_| container::Style {
						background: Some(colors::FLOATING_PANEL_BG.into()),
						border: iced::Border {
							color: colors::ACCENT,
							width: 2.0,
							radius: 8.0.into(),
						},
						..Default::default()
					}),
			)
			.width(Length::Fill)
			.height(Length::Fill)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.into()
		} else {
			Space::new().into()
		}
	}

	fn view_content_with_drop_zones<'a, M>(
		&'a self,
		container_id: usize,
		content: Element<'a, PanelSystemMessage<C, M>>,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let center_zone = self.view_drop_zone_indicator(container_id, DropPosition::Center);
		let left_zone = self.view_drop_zone_indicator(container_id, DropPosition::Left);
		let right_zone = self.view_drop_zone_indicator(container_id, DropPosition::Right);
		let top_zone = self.view_drop_zone_indicator(container_id, DropPosition::Top);
		let bottom_zone = self.view_drop_zone_indicator(container_id, DropPosition::Bottom);

		let drop_zones = container(
			column![
				Space::new().height(Length::FillPortion(1)),
				row![
					Space::new().width(Length::FillPortion(1)),
					top_zone,
					Space::new().width(Length::FillPortion(1)),
				],
				row![
					Space::new().width(Length::FillPortion(1)),
					left_zone,
					Space::new().width(10),
					center_zone,
					Space::new().width(10),
					right_zone,
					Space::new().width(Length::FillPortion(1)),
				]
				.align_y(iced::Alignment::Center),
				row![
					Space::new().width(Length::FillPortion(1)),
					bottom_zone,
					Space::new().width(Length::FillPortion(1)),
				],
				Space::new().height(Length::FillPortion(1)),
			]
			.align_x(iced::Alignment::Center)
			.spacing(10),
		)
		.width(Length::Fill)
		.height(Length::Fill)
		.center_x(Length::Fill)
		.center_y(Length::Fill);

		stack![content, drop_zones,]
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn view_drop_zone_indicator<'a, M>(
		&self,
		container_id: usize,
		position: DropPosition,
	) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		let is_hovered = self
			.hover_drop_zone
			.as_ref()
			.map(|z| z.container_id == container_id && z.position == position)
			.unwrap_or(false);

		let bg_color = if is_hovered {
			colors::DROP_ZONE_HIGHLIGHT
		} else {
			Color::from_rgba(0.2, 0.2, 0.25, 0.7)
		};

		let border_color = if is_hovered {
			colors::DROP_ZONE_BORDER
		} else {
			Color::from_rgba(0.4, 0.4, 0.5, 0.5)
		};

		let icon = match position {
			DropPosition::Center => "⊕",
			DropPosition::Left => "◀",
			DropPosition::Right => "▶",
			DropPosition::Top => "▲",
			DropPosition::Bottom => "▼",
		};

		let zone_content = container(text(icon).size(18).color(if is_hovered {
			colors::TEXT_PRIMARY
		} else {
			colors::TEXT_SECONDARY
		}))
		.width(DROP_ZONE_SIZE)
		.height(DROP_ZONE_SIZE)
		.center_x(DROP_ZONE_SIZE)
		.center_y(DROP_ZONE_SIZE)
		.style(move |_| container::Style {
			background: Some(bg_color.into()),
			border: iced::Border {
				color: border_color,
				width: 2.0,
				radius: 8.0.into(),
			},
			..Default::default()
		});

		let drop_zone = DropZone {
			container_id,
			position,
		};

		mouse_area(zone_content)
			.on_enter(PanelSystemMessage::DropZoneHover(Some(drop_zone)))
			.on_exit(PanelSystemMessage::DropZoneHover(None))
			.on_release(PanelSystemMessage::TabDragEnd)
			.into()
	}

	fn view_empty_content<'a, M>(&self) -> Element<'a, PanelSystemMessage<C, M>>
	where
		M: Clone + std::fmt::Debug + 'static,
	{
		container(
			text("Drop a tab here")
				.color(colors::TEXT_SECONDARY)
				.size(14),
		)
		.width(Length::Fill)
		.height(Length::Fill)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
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

		let handle_size = if is_active || is_hovered { 4.0 } else { 2.0 };

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
						border: iced::Border {
							radius: 2.0.into(),
							..Default::default()
						},
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
						border: iced::Border {
							radius: 2.0.into(),
							..Default::default()
						},
						..Default::default()
					}),
			)
			.width(width)
			.height(height)
			.center_x(width)
			.center_y(height),
		};

		let path_clone = path.clone();
		let path_for_press = path.clone();

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
	// ヘルパーメソッド
	// ==========================================================================

	fn find_panel(&self, panel_id: usize) -> Option<&Panel<C>> {
		Self::find_panel_recursive(&self.root, panel_id)
	}

	fn find_panel_recursive(node: &DockNode<C>, panel_id: usize) -> Option<&Panel<C>> {
		match node {
			DockNode::Leaf(container) => container.get_panel(panel_id),
			DockNode::Split { first, second, .. } => Self::find_panel_recursive(first, panel_id)
				.or_else(|| Self::find_panel_recursive(second, panel_id)),
			_ => None,
		}
	}

	fn set_active_tab(&mut self, container_id: usize, tab_index: usize) {
		Self::set_active_tab_recursive(&mut self.root, container_id, tab_index);
	}

	fn set_active_tab_recursive(node: &mut DockNode<C>, container_id: usize, tab_index: usize) {
		match node {
			DockNode::Leaf(container) if container.id == container_id => {
				if tab_index < container.panels.len() {
					container.active_tab = tab_index;
				}
			}
			DockNode::Split { first, second, .. } => {
				Self::set_active_tab_recursive(first, container_id, tab_index);
				Self::set_active_tab_recursive(second, container_id, tab_index);
			}
			_ => {}
		}
	}

	fn get_panel_id_at(&self, container_id: usize, tab_index: usize) -> Option<usize> {
		Self::get_panel_id_recursive(&self.root, container_id, tab_index)
	}

	fn get_panel_id_recursive(
		node: &DockNode<C>,
		container_id: usize,
		tab_index: usize,
	) -> Option<usize> {
		match node {
			DockNode::Leaf(container) if container.id == container_id => {
				container.panels.get(tab_index).map(|p| p.id)
			}
			DockNode::Split { first, second, .. } => {
				Self::get_panel_id_recursive(first, container_id, tab_index)
					.or_else(|| Self::get_panel_id_recursive(second, container_id, tab_index))
			}
			_ => None,
		}
	}

	fn remove_panel_from_container(&mut self, container_id: usize, panel_id: usize) {
		Self::remove_panel_recursive(&mut self.root, container_id, panel_id);
	}

	fn remove_panel_recursive(node: &mut DockNode<C>, container_id: usize, panel_id: usize) {
		match node {
			DockNode::Leaf(container) if container.id == container_id => {
				container.remove_panel(panel_id);
			}
			DockNode::Split { first, second, .. } => {
				Self::remove_panel_recursive(first, container_id, panel_id);
				Self::remove_panel_recursive(second, container_id, panel_id);
			}
			_ => {}
		}
	}

	fn add_panel_to_container(&mut self, container_id: usize, panel: Panel<C>) {
		Self::add_panel_recursive(&mut self.root, container_id, panel);
	}

	fn add_panel_recursive(node: &mut DockNode<C>, container_id: usize, panel: Panel<C>) -> bool {
		match node {
			DockNode::Leaf(container) if container.id == container_id => {
				container.add_panel(panel);
				true
			}
			DockNode::Split { first, second, .. } => {
				Self::add_panel_recursive(first, container_id, panel.clone())
					|| Self::add_panel_recursive(second, container_id, panel)
			}
			_ => false,
		}
	}

	fn handle_drop(&mut self, source_container: usize, panel_id: usize, drop_zone: DropZone) {
		let target_container = drop_zone.container_id;

		// パネルを見つけてクローン
		let panel = match Self::find_panel_recursive(&self.root, panel_id) {
			Some(p) => p.clone(),
			None => return,
		};

		match drop_zone.position {
			DropPosition::Center => {
				if source_container == target_container {
					return;
				}
				self.remove_panel_from_container(source_container, panel_id);
				self.add_panel_to_container(target_container, panel);
				self.cleanup_empty_nodes();
			}
			position => {
				self.split_container(source_container, panel_id, target_container, position);
			}
		}
	}

	fn split_container(
		&mut self,
		source_container: usize,
		panel_id: usize,
		target_container: usize,
		position: DropPosition,
	) {
		// パネルを見つけてクローン
		let panel = match Self::find_panel_recursive(&self.root, panel_id) {
			Some(p) => p.clone(),
			None => return,
		};

		self.remove_panel_from_container(source_container, panel_id);

		let new_container_id = self.next_container_id;
		self.next_container_id += 1;

		let mut new_container = TabContainer::new(new_container_id);
		new_container.add_panel(panel);

		let direction = match position {
			DropPosition::Left | DropPosition::Right => SplitDirection::Horizontal,
			DropPosition::Top | DropPosition::Bottom => SplitDirection::Vertical,
			DropPosition::Center => return,
		};

		let new_is_first = matches!(position, DropPosition::Left | DropPosition::Top);

		Self::split_node_recursive(
			&mut self.root,
			target_container,
			new_container,
			direction,
			new_is_first,
		);

		self.cleanup_empty_nodes();
	}

	fn split_node_recursive(
		node: &mut DockNode<C>,
		target_container_id: usize,
		new_container: TabContainer<C>,
		direction: SplitDirection,
		new_is_first: bool,
	) -> bool {
		match node {
			DockNode::Leaf(container) if container.id == target_container_id => {
				let existing = std::mem::replace(node, DockNode::Empty);
				let new_leaf = DockNode::Leaf(new_container);

				*node = if new_is_first {
					DockNode::Split {
						direction,
						ratio: 0.5,
						first: Box::new(new_leaf),
						second: Box::new(existing),
					}
				} else {
					DockNode::Split {
						direction,
						ratio: 0.5,
						first: Box::new(existing),
						second: Box::new(new_leaf),
					}
				};
				true
			}
			DockNode::Split { first, second, .. } => {
				Self::split_node_recursive(
					first,
					target_container_id,
					new_container.clone(),
					direction,
					new_is_first,
				) || Self::split_node_recursive(
					second,
					target_container_id,
					new_container,
					direction,
					new_is_first,
				)
			}
			_ => false,
		}
	}

	fn cleanup_empty_nodes(&mut self) {
		Self::cleanup_recursive(&mut self.root);
	}

	fn cleanup_recursive(node: &mut DockNode<C>) {
		match node {
			DockNode::Leaf(container) if container.is_empty() => {
				*node = DockNode::Empty;
			}
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

impl<C: Clone + std::fmt::Debug + PartialEq + Eq + 'static> Default for PanelSystem<C> {
	fn default() -> Self {
		Self::new()
	}
}

// =============================================================================
// レイアウトビルダー
// =============================================================================

/// レイアウトを簡単に構築するためのビルダー
pub struct LayoutBuilder<C: Clone + std::fmt::Debug> {
	next_container_id: usize,
	next_panel_id: usize,
	_marker: std::marker::PhantomData<C>,
}

impl<C: Clone + std::fmt::Debug + PartialEq + Eq> LayoutBuilder<C> {
	pub fn new() -> Self {
		Self {
			next_container_id: 0,
			next_panel_id: 0,
			_marker: std::marker::PhantomData,
		}
	}

	/// 単一パネルのリーフノードを作成
	pub fn panel(&mut self, title: &str, content: C) -> DockNode<C> {
		let container_id = self.next_container_id;
		self.next_container_id += 1;

		let panel_id = self.next_panel_id;
		self.next_panel_id += 1;

		let mut container = TabContainer::new(container_id);
		container.add_panel(Panel::new(panel_id, title, content));

		DockNode::Leaf(container)
	}

	/// 水平分割ノードを作成
	pub fn hsplit(first: DockNode<C>, second: DockNode<C>, ratio: f32) -> DockNode<C> {
		DockNode::Split {
			direction: SplitDirection::Horizontal,
			ratio,
			first: Box::new(first),
			second: Box::new(second),
		}
	}

	/// 垂直分割ノードを作成
	pub fn vsplit(first: DockNode<C>, second: DockNode<C>, ratio: f32) -> DockNode<C> {
		DockNode::Split {
			direction: SplitDirection::Vertical,
			ratio,
			first: Box::new(first),
			second: Box::new(second),
		}
	}

	/// 次のコンテナID
	pub fn next_container_id(&self) -> usize {
		self.next_container_id
	}

	/// 次のパネルID
	pub fn next_panel_id(&self) -> usize {
		self.next_panel_id
	}
}

impl<C: Clone + std::fmt::Debug + PartialEq + Eq> Default for LayoutBuilder<C> {
	fn default() -> Self {
		Self::new()
	}
}
