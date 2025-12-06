#![allow(dead_code)]

use iced::{
	Color, Element, Length, Point, Size,
	widget::{Column, Row, Space, button, column, container, mouse_area, row, stack, text},
};

use crate::PanelContent;

// =============================================================================
// 定数
// =============================================================================

mod consts {
	use iced::Color;

	pub const TAB_HEIGHT: f32 = 32.0;
	pub const RESIZE_HANDLE_SIZE: f32 = 6.0;
	pub const DROP_ZONE_SIZE: f32 = 50.0;
	pub const DRAG_THRESHOLD: f32 = 5.0;

	pub mod colors {
		use super::Color;

		pub const BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.12);
		pub const PANEL_BG: Color = Color::from_rgb(0.15, 0.15, 0.15);
		pub const TAB_BG: Color = Color::from_rgb(0.18, 0.18, 0.18);
		pub const TAB_ACTIVE: Color = Color::from_rgb(0.25, 0.25, 0.28);
		pub const TAB_HOVER: Color = Color::from_rgb(0.22, 0.22, 0.22);
		pub const TAB_BAR_BG: Color = Color::from_rgb(0.13, 0.13, 0.13);
		pub const BORDER: Color = Color::from_rgb(0.08, 0.08, 0.08);
		pub const TEXT_PRIMARY: Color = Color::from_rgb(0.9, 0.9, 0.9);
		pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.6);
		pub const ACCENT: Color = Color::from_rgb(0.3, 0.5, 0.9);
		pub const RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.1);
		pub const RESIZE_HANDLE_HOVER: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.8);
		pub const RESIZE_HANDLE_ACTIVE: Color = Color::from_rgba(0.4, 0.6, 1.0, 1.0);
		pub const DROP_ZONE_HIGHLIGHT: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.3);
		pub const DROP_ZONE_BORDER: Color = Color::from_rgba(0.3, 0.5, 0.9, 0.8);
		pub const FLOATING_PANEL_BG: Color = Color::from_rgba(0.2, 0.2, 0.22, 0.95);
	}
}

use consts::*;

// =============================================================================
// パネルシステム
// =============================================================================

#[derive(Debug)]
pub struct PanelSystem {
	root: DockNode,
	panels: Vec<Panel>,
	next_panel_id: usize,
	next_container_id: usize,
	drag_state: DragState,
	hover_drop_zone: Option<DropZone>,
	last_mouse_pos: Point,
	window_size: Size,
	hover_resize_handle: Option<(Vec<usize>, SplitDirection)>,
}

#[derive(Debug, Clone)]
pub enum PanelSystemMessage {
	TabClicked(usize, usize),   // (container_id, tab_index)
	TabDragStart(usize, usize), // (container_id, tab_index) - position tracked via MouseMove
	MouseMove(Point),
	TabDragEnd,
	TabClose(usize, usize),
	ResizeStart(Vec<usize>, SplitDirection), // (path to split node, direction)
	ResizeEnd,
	DropZoneHover(Option<DropZone>),
	ResizeHandleHover(Option<(Vec<usize>, SplitDirection)>),
	WindowResized(Size),
}

// =============================================================================
// パネル
// =============================================================================

#[derive(Debug, Clone)]
pub struct Panel {
	id: usize,
	title: String,
	content: PanelContent,
}

impl Panel {
	fn new(id: usize, title: &str, content: PanelContent) -> Self {
		Self {
			id,
			title: title.to_string(),
			content,
		}
	}
}

// =============================================================================
// ドックノード（分割レイアウト）
// =============================================================================

#[derive(Debug, Clone)]
enum DockNode {
	Empty,
	Leaf(TabContainer),
	Split {
		direction: SplitDirection,
		ratio: f32,
		first: Box<DockNode>,
		second: Box<DockNode>,
	},
}

impl DockNode {
	fn is_empty(&self) -> bool {
		match self {
			DockNode::Empty => true,
			DockNode::Leaf(c) => c.is_empty(),
			DockNode::Split { first, second, .. } => first.is_empty() && second.is_empty(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
	Horizontal, // 左右に分割
	Vertical,   // 上下に分割
}

// =============================================================================
// タブコンテナ
// =============================================================================

#[derive(Debug, Clone)]
struct TabContainer {
	id: usize,
	panel_ids: Vec<usize>,
	active_tab: usize,
}

impl TabContainer {
	fn new(id: usize) -> Self {
		Self {
			id,
			panel_ids: Vec::new(),
			active_tab: 0,
		}
	}

	fn add_panel(&mut self, panel_id: usize) {
		self.panel_ids.push(panel_id);
		self.active_tab = self.panel_ids.len() - 1;
	}

	fn remove_panel(&mut self, panel_id: usize) -> bool {
		if let Some(pos) = self.panel_ids.iter().position(|&id| id == panel_id) {
			self.panel_ids.remove(pos);
			if self.active_tab >= self.panel_ids.len() && !self.panel_ids.is_empty() {
				self.active_tab = self.panel_ids.len() - 1;
			}
			true
		} else {
			false
		}
	}

	fn is_empty(&self) -> bool {
		self.panel_ids.is_empty()
	}
}

// =============================================================================
// ドラッグ状態
// =============================================================================

#[derive(Debug, Clone, Default)]
enum DragState {
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
	fn is_dragging(&self) -> bool {
		matches!(self, DragState::DraggingTab { .. })
	}

	fn dragging_panel_id(&self) -> Option<usize> {
		match self {
			DragState::DraggingTab { panel_id, .. } => Some(*panel_id),
			_ => None,
		}
	}
}

// =============================================================================
// ドロップゾーン
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct DropZone {
	pub container_id: usize,
	pub position: DropPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPosition {
	Center, // 同じコンテナのタブとして追加
	Left,   // 左側に新しいパネルを作成
	Right,  // 右側に新しいパネルを作成
	Top,    // 上側に新しいパネルを作成
	Bottom, // 下側に新しいパネルを作成
}

// =============================================================================
// 実装
// =============================================================================

impl PanelSystem {
	pub fn new() -> Self {
		Self {
			root: DockNode::Leaf(TabContainer::new(0)),
			panels: Vec::new(),
			next_panel_id: 0,
			next_container_id: 1,
			drag_state: DragState::None,
			hover_drop_zone: None,
			last_mouse_pos: Point::ORIGIN,
			window_size: Size::new(800.0, 600.0),
			hover_resize_handle: None,
		}
	}

	pub fn add_panel(&mut self, title: &str, content: PanelContent) {
		let panel_id = self.next_panel_id;
		self.next_panel_id += 1;

		let panel = Panel::new(panel_id, title, content);
		self.panels.push(panel);

		// ルートコンテナに追加
		if let DockNode::Leaf(container) = &mut self.root {
			container.add_panel(panel_id);
		}
	}

	pub fn update(&mut self, message: PanelSystemMessage) {
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
						// ドラッグ閾値を超えたらドラッグ開始
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
						let new_ratio =
							self.calculate_new_ratio(start_pos, pos, start_ratio, direction);
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
						// ドラッグ閾値を超えていない場合はタブクリックとして処理
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

			PanelSystemMessage::DropZoneHover(zone) => {
				self.hover_drop_zone = zone;
			}

			PanelSystemMessage::ResizeHandleHover(handle) => {
				// リサイズ中はホバー状態の変更を無視（ちらつき防止）
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
		start_pos: Point,
		current_pos: Point,
		start_ratio: f32,
		direction: SplitDirection,
	) -> f32 {
		let size = match direction {
			SplitDirection::Horizontal => self.window_size.width.max(100.0),
			SplitDirection::Vertical => self.window_size.height.max(100.0),
		};
		let delta = match direction {
			SplitDirection::Horizontal => (current_pos.x - start_pos.x) / size,
			SplitDirection::Vertical => (current_pos.y - start_pos.y) / size,
		};
		(start_ratio + delta).clamp(0.15, 0.85)
	}

	pub fn view(&self) -> Element<'_, PanelSystemMessage> {
		let main_content = self.view_node(&self.root, vec![]);

		// ドラッグ中のフローティングパネルとオーバーレイを表示
		if let DragState::DraggingTab {
			panel_id,
			current_pos,
			..
		} = &self.drag_state
		{
			if let Some(panel) = self.panels.iter().find(|p| p.id == *panel_id) {
				let floating = self.view_floating_panel(panel, *current_pos);

				// ドロップゾーンのオーバーレイ
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

	fn view_main_container<'a>(
		&self,
		content: Element<'a, PanelSystemMessage>,
	) -> Element<'a, PanelSystemMessage> {
		// リサイズ中はマウスイベントをキャプチャ
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

	fn view_node(&self, node: &DockNode, path: Vec<usize>) -> Element<'_, PanelSystemMessage> {
		match node {
			DockNode::Empty => Space::new(Length::Fill, Length::Fill).into(),

			DockNode::Leaf(container) => self.view_tab_container(container),

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

				let first_view = self.view_node(first, first_path);
				let second_view = self.view_node(second, second_path);

				// 丸め誤差を防ぐため、first_portionを計算してからsecondを差分で計算
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

	fn view_tab_container(&self, tab_container: &TabContainer) -> Element<'_, PanelSystemMessage> {
		let container_id = tab_container.id;
		let dragging_panel = self.drag_state.dragging_panel_id();

		// タブバー
		let tabs: Vec<Element<PanelSystemMessage>> = tab_container
			.panel_ids
			.iter()
			.enumerate()
			.map(|(index, &panel_id)| {
				let panel = self.panels.iter().find(|p| p.id == panel_id);
				let is_active = index == tab_container.active_tab;
				let is_being_dragged = dragging_panel == Some(panel_id);

				if let Some(panel) = panel {
					self.view_tab(container_id, index, panel, is_active, is_being_dragged)
				} else {
					Space::new(0, 0).into()
				}
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

		// アクティブパネルのコンテンツ
		let content = if let Some(&panel_id) = tab_container.panel_ids.get(tab_container.active_tab)
		{
			if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
				self.view_panel_content(panel)
			} else {
				self.view_empty_content()
			}
		} else {
			self.view_empty_content()
		};

		// ドラッグ中はドロップゾーンを表示
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

	fn view_tab(
		&self,
		container_id: usize,
		index: usize,
		panel: &Panel,
		is_active: bool,
		is_being_dragged: bool,
	) -> Element<'_, PanelSystemMessage> {
		let bg_color = if is_active {
			colors::TAB_ACTIVE
		} else {
			colors::TAB_BG
		};

		let opacity = if is_being_dragged { 0.4 } else { 1.0 };

		let icon = panel.content.icon();
		let title = &panel.title;

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
			text(format!("{} {}", icon, title))
				.size(12)
				.color(text_color),
			Space::with_width(Length::Fill),
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

		// マウスエリアでドラッグを検出
		mouse_area(tab_button)
			.on_press(PanelSystemMessage::TabDragStart(container_id, index))
			.on_move(PanelSystemMessage::MouseMove)
			.on_release(PanelSystemMessage::TabDragEnd)
			.into()
	}

	fn view_floating_panel(&self, panel: &Panel, pos: Point) -> Element<'_, PanelSystemMessage> {
		let icon = panel.content.icon();
		let title = &panel.title;

		let floating_content = container(
			row![
				text(format!("{} {}", icon, title))
					.size(12)
					.color(colors::TEXT_PRIMARY),
			]
			.padding([8, 12]),
		)
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

		// フローティングパネルを左上からのオフセットで配置
		Column::new()
			.push(Space::with_height(pos.y.max(0.0)))
			.push(
				Row::new()
					.push(Space::with_width(pos.x.max(0.0)))
					.push(floating_content),
			)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn view_drop_zone_overlay(&self) -> Element<'_, PanelSystemMessage> {
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
			// 透明なオーバーレイ（ドラッグイベントをキャプチャ）
			Space::new(0, 0).into()
		}
	}

	fn view_content_with_drop_zones<'a>(
		&'a self,
		container_id: usize,
		content: Element<'a, PanelSystemMessage>,
	) -> Element<'a, PanelSystemMessage> {
		// ドラッグ中はドロップゾーンインジケーターを表示
		let center_zone = self.view_drop_zone_indicator(container_id, DropPosition::Center);
		let left_zone = self.view_drop_zone_indicator(container_id, DropPosition::Left);
		let right_zone = self.view_drop_zone_indicator(container_id, DropPosition::Right);
		let top_zone = self.view_drop_zone_indicator(container_id, DropPosition::Top);
		let bottom_zone = self.view_drop_zone_indicator(container_id, DropPosition::Bottom);

		// ドロップゾーンを中央に十字に配置
		let drop_zones = container(
			column![
				Space::with_height(Length::FillPortion(1)),
				row![
					Space::with_width(Length::FillPortion(1)),
					top_zone,
					Space::with_width(Length::FillPortion(1)),
				],
				row![
					Space::with_width(Length::FillPortion(1)),
					left_zone,
					Space::with_width(10),
					center_zone,
					Space::with_width(10),
					right_zone,
					Space::with_width(Length::FillPortion(1)),
				]
				.align_y(iced::Alignment::Center),
				row![
					Space::with_width(Length::FillPortion(1)),
					bottom_zone,
					Space::with_width(Length::FillPortion(1)),
				],
				Space::with_height(Length::FillPortion(1)),
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

	fn view_drop_zone_indicator(
		&self,
		container_id: usize,
		position: DropPosition,
	) -> Element<'_, PanelSystemMessage> {
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

		// マウスエリアでホバーを検出
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

	fn view_panel_content(&self, panel: &Panel) -> Element<'_, PanelSystemMessage> {
		let content = match panel.content {
			PanelContent::Explorer => self.view_explorer_content(),
			PanelContent::Properties => self.view_properties_content(),
			PanelContent::Timeline => self.view_timeline_content(),
			PanelContent::Preview => self.view_preview_content(),
			PanelContent::Console => self.view_console_content(),
			PanelContent::Assets => self.view_assets_content(),
		};

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.padding(10)
			.into()
	}

	fn view_empty_content(&self) -> Element<'_, PanelSystemMessage> {
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

	fn view_resize_handle(
		&self,
		direction: SplitDirection,
		path: Vec<usize>,
	) -> Element<'_, PanelSystemMessage> {
		// リサイズ中かどうか（どのハンドルでも）
		let is_any_resizing = matches!(&self.drag_state, DragState::Resizing { .. });

		// このハンドルがアクティブ（リサイズ中）かどうか
		let is_active = match &self.drag_state {
			DragState::Resizing {
				path: p,
				direction: d,
				..
			} => *p == path && *d == direction,
			_ => false,
		};

		// ホバー中かどうか（リサイズ中は無視）
		let is_hovered = if is_any_resizing {
			false // リサイズ中はホバー状態を使わない
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

		// アクティブまたはホバー時はハンドルを大きく表示
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
				container(Space::new(0, 0))
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
				container(Space::new(0, 0))
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

		// リサイズ中はハンドル内のon_moveを無効化（座標の不整合を防ぐ）
		let ma = mouse_area(inner_handle)
			.on_press(PanelSystemMessage::ResizeStart(path_for_press, direction))
			.on_release(PanelSystemMessage::ResizeEnd)
			.on_enter(PanelSystemMessage::ResizeHandleHover(Some((
				path_clone, direction,
			))))
			.on_exit(PanelSystemMessage::ResizeHandleHover(None));

		// リサイズ中でない場合のみon_moveを設定
		if is_any_resizing {
			ma.into()
		} else {
			ma.on_move(PanelSystemMessage::MouseMove).into()
		}
	}

	// サンプルコンテンツ
	fn view_explorer_content(&self) -> Element<'_, PanelSystemMessage> {
		let items = [
			"📁 src",
			"  📄 main.rs",
			"  📄 lib.rs",
			"📁 assets",
			"  📁 textures",
			"  📁 sounds",
			"📄 Cargo.toml",
		];

		column(
			items
				.iter()
				.map(|item| text(*item).size(12).color(colors::TEXT_PRIMARY).into())
				.collect::<Vec<_>>(),
		)
		.spacing(4)
		.into()
	}

	fn view_properties_content(&self) -> Element<'_, PanelSystemMessage> {
		column![
			text("Transform").size(14).color(colors::ACCENT),
			row![
				text("Position:").size(12).color(colors::TEXT_SECONDARY),
				text("0, 0, 0").size(12).color(colors::TEXT_PRIMARY),
			]
			.spacing(8),
			row![
				text("Rotation:").size(12).color(colors::TEXT_SECONDARY),
				text("0°, 0°, 0°").size(12).color(colors::TEXT_PRIMARY),
			]
			.spacing(8),
			row![
				text("Scale:").size(12).color(colors::TEXT_SECONDARY),
				text("1, 1, 1").size(12).color(colors::TEXT_PRIMARY),
			]
			.spacing(8),
		]
		.spacing(8)
		.into()
	}

	fn view_timeline_content(&self) -> Element<'_, PanelSystemMessage> {
		column![
			text("Timeline").size(14).color(colors::ACCENT),
			text("0:00 ─────────────●───── 1:00")
				.size(12)
				.color(colors::TEXT_PRIMARY),
			text("▶ Track 1: Keyframes")
				.size(12)
				.color(colors::TEXT_SECONDARY),
			text("▶ Track 2: Audio")
				.size(12)
				.color(colors::TEXT_SECONDARY),
		]
		.spacing(8)
		.into()
	}

	fn view_preview_content(&self) -> Element<'_, PanelSystemMessage> {
		container(
			column![
				text("Preview").size(14).color(colors::ACCENT),
				Space::with_height(20),
				text("🎥").size(48),
				text("No preview available")
					.size(12)
					.color(colors::TEXT_SECONDARY),
			]
			.spacing(8)
			.align_x(iced::Alignment::Center),
		)
		.width(Length::Fill)
		.height(Length::Fill)
		.center_x(Length::Fill)
		.center_y(Length::Fill)
		.into()
	}

	fn view_console_content(&self) -> Element<'_, PanelSystemMessage> {
		column![
			text("[INFO] Application started")
				.size(11)
				.color(Color::from_rgb(0.4, 0.8, 0.4)),
			text("[DEBUG] Loading assets...")
				.size(11)
				.color(colors::TEXT_SECONDARY),
			text("[INFO] Assets loaded successfully")
				.size(11)
				.color(Color::from_rgb(0.4, 0.8, 0.4)),
			text("[WARN] Deprecated API usage detected")
				.size(11)
				.color(Color::from_rgb(0.9, 0.7, 0.3)),
		]
		.spacing(4)
		.into()
	}

	fn view_assets_content(&self) -> Element<'_, PanelSystemMessage> {
		let assets = [
			"📷 texture_01.png",
			"📷 texture_02.png",
			"🎵 bgm.mp3",
			"🎵 sfx_click.wav",
			"📦 model.obj",
		];

		column(
			assets
				.iter()
				.map(|item| text(*item).size(12).color(colors::TEXT_PRIMARY).into())
				.collect::<Vec<_>>(),
		)
		.spacing(4)
		.into()
	}

	// ==========================================================================
	// ヘルパーメソッド
	// ==========================================================================

	fn set_active_tab(&mut self, container_id: usize, tab_index: usize) {
		Self::set_active_tab_recursive(&mut self.root, container_id, tab_index);
	}

	fn set_active_tab_recursive(node: &mut DockNode, container_id: usize, tab_index: usize) {
		match node {
			DockNode::Leaf(container) if container.id == container_id => {
				if tab_index < container.panel_ids.len() {
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
		node: &DockNode,
		container_id: usize,
		tab_index: usize,
	) -> Option<usize> {
		match node {
			DockNode::Leaf(container) if container.id == container_id => {
				container.panel_ids.get(tab_index).copied()
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

	fn remove_panel_recursive(node: &mut DockNode, container_id: usize, panel_id: usize) {
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

	fn add_panel_to_container(&mut self, container_id: usize, panel_id: usize) {
		Self::add_panel_recursive(&mut self.root, container_id, panel_id);
	}

	fn add_panel_recursive(node: &mut DockNode, container_id: usize, panel_id: usize) {
		match node {
			DockNode::Leaf(container) if container.id == container_id => {
				container.add_panel(panel_id);
			}
			DockNode::Split { first, second, .. } => {
				Self::add_panel_recursive(first, container_id, panel_id);
				Self::add_panel_recursive(second, container_id, panel_id);
			}
			_ => {}
		}
	}

	fn handle_drop(&mut self, source_container: usize, panel_id: usize, drop_zone: DropZone) {
		let target_container = drop_zone.container_id;

		match drop_zone.position {
			DropPosition::Center => {
				// 同じコンテナへのドロップは無視
				if source_container == target_container {
					return;
				}
				// ソースから削除してターゲットに追加
				self.remove_panel_from_container(source_container, panel_id);
				self.add_panel_to_container(target_container, panel_id);
				self.cleanup_empty_nodes();
			}
			position => {
				// 新しい分割を作成
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
		// ソースコンテナからパネルを削除
		self.remove_panel_from_container(source_container, panel_id);

		// 新しいコンテナを作成
		let new_container_id = self.next_container_id;
		self.next_container_id += 1;

		let mut new_container = TabContainer::new(new_container_id);
		new_container.add_panel(panel_id);

		// ターゲットコンテナを分割
		let direction = match position {
			DropPosition::Left | DropPosition::Right => SplitDirection::Horizontal,
			DropPosition::Top | DropPosition::Bottom => SplitDirection::Vertical,
			DropPosition::Center => return, // Centerは上で処理済み
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
		node: &mut DockNode,
		target_container_id: usize,
		new_container: TabContainer,
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

	fn cleanup_recursive(node: &mut DockNode) {
		match node {
			DockNode::Leaf(container) if container.is_empty() => {
				*node = DockNode::Empty;
			}
			DockNode::Split { first, second, .. } => {
				Self::cleanup_recursive(first);
				Self::cleanup_recursive(second);

				// 両方が空なら空に
				if first.is_empty() && second.is_empty() {
					*node = DockNode::Empty;
				}
				// 片方だけ空なら、もう片方で置き換え
				else if first.is_empty() {
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

	fn get_ratio_recursive(node: &DockNode, path: &[usize]) -> Option<f32> {
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

	fn set_ratio_recursive(node: &mut DockNode, path: &[usize], new_ratio: f32) {
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

impl Default for PanelSystem {
	fn default() -> Self {
		Self::new()
	}
}
