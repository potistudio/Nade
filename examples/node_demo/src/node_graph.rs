use iced::{
	Color, Element, Length, Point, Rectangle, Size, Theme, Vector,
	keyboard::{self, Key},
	mouse,
	widget::{
		button,
		canvas::{self, Cache, Canvas, Event, Frame, Geometry, Path, Stroke, Text},
		column, container, row, text,
	},
};
use std::collections::HashMap;

// =============================================================================
// 定数
// =============================================================================

mod consts {
	use iced::Color;

	// ノード
	pub const NODE_WIDTH: f32 = 180.0;
	pub const NODE_HEADER_HEIGHT: f32 = 28.0;
	pub const NODE_PORT_HEIGHT: f32 = 24.0;
	pub const NODE_PORT_RADIUS: f32 = 6.0;
	pub const NODE_PORT_PADDING: f32 = 12.0;
	pub const NODE_SHADOW_OFFSET: f32 = 4.0;

	// グリッド
	pub const GRID_SIZE: f32 = 20.0;
	pub const GRID_SIZE_LARGE: f32 = 100.0;

	// 接続
	pub const CONNECTION_THICKNESS: f32 = 2.5;

	// ズーム
	pub const MIN_ZOOM: f32 = 0.25;
	pub const MAX_ZOOM: f32 = 2.0;
	pub const ZOOM_STEP: f32 = 0.1;

	// 色
	pub mod colors {
		use super::Color;

		pub const BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.14);
		pub const GRID_LINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.03);
		pub const GRID_LINE_LARGE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.06);

		pub const NODE_BG: Color = Color::from_rgb(0.22, 0.22, 0.24);
		pub const NODE_HEADER_BG: Color = Color::from_rgb(0.35, 0.45, 0.65);
		pub const NODE_HEADER_TEXT: Color = Color::from_rgb(1.0, 1.0, 1.0);
		pub const NODE_BORDER: Color = Color::from_rgb(0.1, 0.1, 0.12);
		pub const NODE_SELECTED_BORDER: Color = Color::from_rgb(0.5, 0.7, 1.0);
		pub const NODE_SHADOW: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.4);

		pub const PORT_INPUT: Color = Color::from_rgb(0.3, 0.7, 0.4);
		pub const PORT_FLOAT: Color = Color::from_rgb(0.5, 0.7, 0.9);
		pub const PORT_COLOR: Color = Color::from_rgb(0.9, 0.7, 0.3);
		pub const PORT_BOOL: Color = Color::from_rgb(0.9, 0.4, 0.5);
		pub const PORT_ANY: Color = Color::from_rgb(0.7, 0.7, 0.7);
		pub const PORT_HOVER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.3);

		pub const CONNECTION: Color = Color::from_rgb(0.6, 0.6, 0.7);
		pub const CONNECTION_ACTIVE: Color = Color::from_rgb(0.5, 0.7, 1.0);

		pub const SELECTION_BOX: Color = Color::from_rgba(0.4, 0.6, 1.0, 0.2);
		pub const SELECTION_BORDER: Color = Color::from_rgba(0.4, 0.6, 1.0, 0.8);

		pub const TEXT_PRIMARY: Color = Color::from_rgb(0.9, 0.9, 0.9);
		pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.65);

		// ノードタイプ別ヘッダー色
		pub const HEADER_MATH: Color = Color::from_rgb(0.45, 0.55, 0.75);
		pub const HEADER_INPUT: Color = Color::from_rgb(0.45, 0.65, 0.45);
		pub const HEADER_OUTPUT: Color = Color::from_rgb(0.65, 0.45, 0.45);
		pub const HEADER_UTILITY: Color = Color::from_rgb(0.55, 0.45, 0.65);
		pub const HEADER_TEXTURE: Color = Color::from_rgb(0.65, 0.55, 0.35);
	}
}

use consts::*;

// =============================================================================
// データタイプ
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
	Float,
	Vector2,
	Vector3,
	Color,
	Bool,
	Any,
}

impl DataType {
	fn color(&self) -> Color {
		match self {
			DataType::Float => colors::PORT_FLOAT,
			DataType::Vector2 | DataType::Vector3 => colors::PORT_INPUT,
			DataType::Color => colors::PORT_COLOR,
			DataType::Bool => colors::PORT_BOOL,
			DataType::Any => colors::PORT_ANY,
		}
	}

	fn can_connect(&self, other: &DataType) -> bool {
		if *self == DataType::Any || *other == DataType::Any {
			return true;
		}
		self == other
	}
}

// =============================================================================
// ポート
// =============================================================================

#[derive(Debug, Clone)]
pub struct Port {
	pub id: PortId,
	pub name: String,
	pub data_type: DataType,
	pub is_input: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortId {
	pub node_id: usize,
	pub port_index: usize,
	pub is_input: bool,
}

impl PortId {
	fn new(node_id: usize, port_index: usize, is_input: bool) -> Self {
		Self {
			node_id,
			port_index,
			is_input,
		}
	}
}

// =============================================================================
// ノードタイプ
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
	// 入力
	FloatInput,
	ColorInput,
	TextureInput,
	// 数学
	Add,
	Subtract,
	Multiply,
	Divide,
	Power,
	Sine,
	Cosine,
	// ユーティリティ
	Mix,
	Clamp,
	Remap,
	SplitVector,
	CombineVector,
	// 出力
	Output,
	Preview,
}

impl NodeType {
	fn header_color(&self) -> Color {
		match self {
			NodeType::FloatInput | NodeType::ColorInput | NodeType::TextureInput => {
				colors::HEADER_INPUT
			}
			NodeType::Add
			| NodeType::Subtract
			| NodeType::Multiply
			| NodeType::Divide
			| NodeType::Power
			| NodeType::Sine
			| NodeType::Cosine => colors::HEADER_MATH,
			NodeType::Mix
			| NodeType::Clamp
			| NodeType::Remap
			| NodeType::SplitVector
			| NodeType::CombineVector => colors::HEADER_UTILITY,
			NodeType::Output | NodeType::Preview => colors::HEADER_OUTPUT,
		}
	}

	fn name(&self) -> &'static str {
		match self {
			NodeType::FloatInput => "Float",
			NodeType::ColorInput => "Color",
			NodeType::TextureInput => "Texture",
			NodeType::Add => "Add",
			NodeType::Subtract => "Subtract",
			NodeType::Multiply => "Multiply",
			NodeType::Divide => "Divide",
			NodeType::Power => "Power",
			NodeType::Sine => "Sine",
			NodeType::Cosine => "Cosine",
			NodeType::Mix => "Mix",
			NodeType::Clamp => "Clamp",
			NodeType::Remap => "Remap",
			NodeType::SplitVector => "Split Vector",
			NodeType::CombineVector => "Combine Vector",
			NodeType::Output => "Output",
			NodeType::Preview => "Preview",
		}
	}

	fn create_ports(&self, node_id: usize) -> (Vec<Port>, Vec<Port>) {
		let mut inputs = Vec::new();
		let mut outputs = Vec::new();

		match self {
			NodeType::FloatInput => {
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Value".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
			}
			NodeType::ColorInput => {
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Color".to_string(),
					data_type: DataType::Color,
					is_input: false,
				});
			}
			NodeType::TextureInput => {
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Texture".to_string(),
					data_type: DataType::Color,
					is_input: false,
				});
			}
			NodeType::Add | NodeType::Subtract | NodeType::Multiply | NodeType::Divide => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "A".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 1, true),
					name: "B".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Result".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
			}
			NodeType::Power => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "Base".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 1, true),
					name: "Exponent".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Result".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
			}
			NodeType::Sine | NodeType::Cosine => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "Angle".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Result".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
			}
			NodeType::Mix => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "A".to_string(),
					data_type: DataType::Any,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 1, true),
					name: "B".to_string(),
					data_type: DataType::Any,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 2, true),
					name: "Factor".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Result".to_string(),
					data_type: DataType::Any,
					is_input: false,
				});
			}
			NodeType::Clamp => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "Value".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 1, true),
					name: "Min".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 2, true),
					name: "Max".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Result".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
			}
			NodeType::Remap => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "Value".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 1, true),
					name: "In Min".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 2, true),
					name: "In Max".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 3, true),
					name: "Out Min".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 4, true),
					name: "Out Max".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Result".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
			}
			NodeType::SplitVector => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "Vector".to_string(),
					data_type: DataType::Vector3,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "X".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 1, false),
					name: "Y".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 2, false),
					name: "Z".to_string(),
					data_type: DataType::Float,
					is_input: false,
				});
			}
			NodeType::CombineVector => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "X".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 1, true),
					name: "Y".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 2, true),
					name: "Z".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
				outputs.push(Port {
					id: PortId::new(node_id, 0, false),
					name: "Vector".to_string(),
					data_type: DataType::Vector3,
					is_input: false,
				});
			}
			NodeType::Output => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "Color".to_string(),
					data_type: DataType::Color,
					is_input: true,
				});
				inputs.push(Port {
					id: PortId::new(node_id, 1, true),
					name: "Alpha".to_string(),
					data_type: DataType::Float,
					is_input: true,
				});
			}
			NodeType::Preview => {
				inputs.push(Port {
					id: PortId::new(node_id, 0, true),
					name: "Input".to_string(),
					data_type: DataType::Any,
					is_input: true,
				});
			}
		}

		(inputs, outputs)
	}
}

// =============================================================================
// ノード
// =============================================================================

#[derive(Debug, Clone)]
pub struct Node {
	pub id: usize,
	pub node_type: NodeType,
	pub position: Point,
	pub inputs: Vec<Port>,
	pub outputs: Vec<Port>,
	pub selected: bool,
}

impl Node {
	fn new(id: usize, node_type: NodeType, position: Point) -> Self {
		let (inputs, outputs) = node_type.create_ports(id);
		Self {
			id,
			node_type,
			position,
			inputs,
			outputs,
			selected: false,
		}
	}

	fn height(&self) -> f32 {
		let port_count = self.inputs.len().max(self.outputs.len());
		NODE_HEADER_HEIGHT + (port_count as f32 * NODE_PORT_HEIGHT) + NODE_PORT_PADDING
	}

	fn bounds(&self) -> Rectangle {
		Rectangle {
			x: self.position.x,
			y: self.position.y,
			width: NODE_WIDTH,
			height: self.height(),
		}
	}

	fn port_position(&self, port_index: usize, is_input: bool) -> Point {
		let x = if is_input {
			self.position.x
		} else {
			self.position.x + NODE_WIDTH
		};
		let y = self.position.y
			+ NODE_HEADER_HEIGHT
			+ (port_index as f32 * NODE_PORT_HEIGHT)
			+ NODE_PORT_HEIGHT / 2.0;
		Point::new(x, y)
	}

	fn get_port_at(&self, pos: Point) -> Option<(usize, bool)> {
		// 入力ポートをチェック
		for (i, _) in self.inputs.iter().enumerate() {
			let port_pos = self.port_position(i, true);
			let dist = ((pos.x - port_pos.x).powi(2) + (pos.y - port_pos.y).powi(2)).sqrt();
			if dist <= NODE_PORT_RADIUS * 2.0 {
				return Some((i, true));
			}
		}
		// 出力ポートをチェック
		for (i, _) in self.outputs.iter().enumerate() {
			let port_pos = self.port_position(i, false);
			let dist = ((pos.x - port_pos.x).powi(2) + (pos.y - port_pos.y).powi(2)).sqrt();
			if dist <= NODE_PORT_RADIUS * 2.0 {
				return Some((i, false));
			}
		}
		None
	}
}

// =============================================================================
// 接続
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Connection {
	pub from_node: usize,
	pub from_port: usize,
	pub to_node: usize,
	pub to_port: usize,
}

impl Connection {
	fn new(from_node: usize, from_port: usize, to_node: usize, to_port: usize) -> Self {
		Self {
			from_node,
			from_port,
			to_node,
			to_port,
		}
	}
}

// =============================================================================
// インタラクション状態
// =============================================================================

#[derive(Debug, Clone)]
enum DragState {
	None,
	Node {
		node_id: usize,
		offset: Vector,
	},
	Selection {
		start: Point,
		end: Point,
	},
	Pan {
		last_pos: Point,
	},
	Connection {
		from_port: PortId,
		current_pos: Point,
	},
}

// =============================================================================
// ノードグラフ
// =============================================================================

pub struct NodeGraph {
	nodes: HashMap<usize, Node>,
	connections: Vec<Connection>,
	next_node_id: usize,
	drag_state: DragState,
	camera_offset: Vector,
	zoom: f32,
	size: Size,
	hovered_port: Option<PortId>,
	cache: Cache,
	show_context_menu: bool,
	context_menu_pos: Point,
	selected_nodes: Vec<usize>,
}

impl std::fmt::Debug for NodeGraph {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("NodeGraph")
			.field("nodes", &self.nodes)
			.field("connections", &self.connections)
			.field("zoom", &self.zoom)
			.finish()
	}
}

#[derive(Debug, Clone)]
pub enum NodeGraphMessage {
	AddNode(NodeType, Point),
	RemoveNode(usize),
	NodeDragStart(usize, Point),
	NodeDragMove(Point),
	NodeDragEnd,
	ConnectionStart(PortId, Point),
	ConnectionMove(Point),
	ConnectionEnd(Option<PortId>),
	RemoveConnection(Connection),
	SelectNode(usize, bool), // (node_id, add_to_selection)
	ClearSelection,
	SelectionBoxStart(Point),
	SelectionBoxMove(Point),
	SelectionBoxEnd,
	PanStart(Point),
	PanMove(Point),
	PanEnd,
	Zoom(f32, Point),
	PortHover(Option<PortId>),
	ContextMenu(Point),
	CloseContextMenu,
	DeleteSelected,
	DuplicateSelected,
	ResetView,
}

impl NodeGraph {
	pub fn new() -> Self {
		let mut graph = Self {
			nodes: HashMap::new(),
			connections: Vec::new(),
			next_node_id: 0,
			drag_state: DragState::None,
			camera_offset: Vector::new(0.0, 0.0),
			zoom: 1.0,
			size: Size::new(1400.0, 900.0),
			hovered_port: None,
			cache: Cache::new(),
			show_context_menu: false,
			context_menu_pos: Point::ORIGIN,
			selected_nodes: Vec::new(),
		};

		// サンプルノードを追加
		graph.add_sample_nodes();

		graph
	}

	fn add_sample_nodes(&mut self) {
		// Float入力ノード
		let float1 = self.create_node(NodeType::FloatInput, Point::new(100.0, 150.0));
		let float2 = self.create_node(NodeType::FloatInput, Point::new(100.0, 300.0));

		// 演算ノード
		let add = self.create_node(NodeType::Add, Point::new(350.0, 200.0));
		let multiply = self.create_node(NodeType::Multiply, Point::new(550.0, 250.0));

		// Mixノード
		let mix = self.create_node(NodeType::Mix, Point::new(750.0, 200.0));

		// カラー入力
		let color = self.create_node(NodeType::ColorInput, Point::new(550.0, 100.0));

		// サンプル接続
		self.connections.push(Connection::new(float1, 0, add, 0));
		self.connections.push(Connection::new(float2, 0, add, 1));
		self.connections.push(Connection::new(add, 0, multiply, 0));
		self.connections
			.push(Connection::new(float1, 0, multiply, 1));
		self.connections.push(Connection::new(color, 0, mix, 0));
		self.connections.push(Connection::new(multiply, 0, mix, 2));
	}

	fn create_node(&mut self, node_type: NodeType, position: Point) -> usize {
		let id = self.next_node_id;
		self.next_node_id += 1;
		self.nodes.insert(id, Node::new(id, node_type, position));
		self.cache.clear();
		id
	}

	pub fn set_size(&mut self, size: Size) {
		self.size = size;
		self.cache.clear();
	}

	pub fn update(&mut self, message: NodeGraphMessage) {
		match message {
			NodeGraphMessage::AddNode(node_type, pos) => {
				let world_pos = self.screen_to_world(pos);
				self.create_node(node_type, world_pos);
				self.show_context_menu = false;
			}
			NodeGraphMessage::RemoveNode(id) => {
				self.nodes.remove(&id);
				self.connections
					.retain(|c| c.from_node != id && c.to_node != id);
				self.selected_nodes.retain(|&n| n != id);
				self.cache.clear();
			}
			NodeGraphMessage::NodeDragStart(node_id, pos) => {
				if let Some(node) = self.nodes.get(&node_id) {
					let world_pos = self.screen_to_world(pos);
					let offset =
						Vector::new(world_pos.x - node.position.x, world_pos.y - node.position.y);
					self.drag_state = DragState::Node { node_id, offset };

					// ノードが選択されていない場合、選択をクリアしてこのノードを選択
					if !self.selected_nodes.contains(&node_id) {
						self.selected_nodes.clear();
						self.selected_nodes.push(node_id);
						for node in self.nodes.values_mut() {
							node.selected = node.id == node_id;
						}
					}
				}
			}
			NodeGraphMessage::NodeDragMove(pos) => {
				if let DragState::Node { node_id, offset } = &self.drag_state {
					let world_pos = self.screen_to_world(pos);
					let new_pos = Point::new(world_pos.x - offset.x, world_pos.y - offset.y);

					// グリッドスナップ
					let snapped_pos = Point::new(
						(new_pos.x / GRID_SIZE).round() * GRID_SIZE,
						(new_pos.y / GRID_SIZE).round() * GRID_SIZE,
					);

					if let Some(node) = self.nodes.get_mut(node_id) {
						let delta = Vector::new(
							snapped_pos.x - node.position.x,
							snapped_pos.y - node.position.y,
						);
						node.position = snapped_pos;

						// 選択されている他のノードも移動
						let selected: Vec<usize> = self
							.selected_nodes
							.iter()
							.copied()
							.filter(|&n| n != *node_id)
							.collect();
						for id in selected {
							if let Some(other_node) = self.nodes.get_mut(&id) {
								other_node.position.x += delta.x;
								other_node.position.y += delta.y;
							}
						}
					}
					self.cache.clear();
				}
			}
			NodeGraphMessage::NodeDragEnd => {
				self.drag_state = DragState::None;
			}
			NodeGraphMessage::ConnectionStart(port_id, pos) => {
				let world_pos = self.screen_to_world(pos);
				self.drag_state = DragState::Connection {
					from_port: port_id,
					current_pos: world_pos,
				};
			}
			NodeGraphMessage::ConnectionMove(pos) => {
				let world_pos = self.screen_to_world(pos);
				if let DragState::Connection { current_pos, .. } = &mut self.drag_state {
					*current_pos = world_pos;
				}
				self.cache.clear();
			}
			NodeGraphMessage::ConnectionEnd(maybe_port) => {
				if let DragState::Connection { from_port, .. } = &self.drag_state {
					if let Some(to_port) = maybe_port {
						// 接続を作成
						if from_port.is_input != to_port.is_input
							&& from_port.node_id != to_port.node_id
						{
							let (from, to) = if from_port.is_input {
								(to_port, *from_port)
							} else {
								(*from_port, to_port)
							};

							// 既存の入力接続を削除
							self.connections.retain(|c| {
								!(c.to_node == to.node_id && c.to_port == to.port_index)
							});

							// 新しい接続を追加
							self.connections.push(Connection::new(
								from.node_id,
								from.port_index,
								to.node_id,
								to.port_index,
							));
						}
					}
				}
				self.drag_state = DragState::None;
				self.cache.clear();
			}
			NodeGraphMessage::RemoveConnection(connection) => {
				self.connections.retain(|c| c != &connection);
				self.cache.clear();
			}
			NodeGraphMessage::SelectNode(node_id, add_to_selection) => {
				if add_to_selection {
					if self.selected_nodes.contains(&node_id) {
						self.selected_nodes.retain(|&n| n != node_id);
						if let Some(node) = self.nodes.get_mut(&node_id) {
							node.selected = false;
						}
					} else {
						self.selected_nodes.push(node_id);
						if let Some(node) = self.nodes.get_mut(&node_id) {
							node.selected = true;
						}
					}
				} else {
					self.selected_nodes.clear();
					self.selected_nodes.push(node_id);
					for node in self.nodes.values_mut() {
						node.selected = node.id == node_id;
					}
				}
				self.cache.clear();
			}
			NodeGraphMessage::ClearSelection => {
				self.selected_nodes.clear();
				for node in self.nodes.values_mut() {
					node.selected = false;
				}
				self.cache.clear();
			}
			NodeGraphMessage::SelectionBoxStart(pos) => {
				let world_pos = self.screen_to_world(pos);
				self.drag_state = DragState::Selection {
					start: world_pos,
					end: world_pos,
				};
			}
			NodeGraphMessage::SelectionBoxMove(pos) => {
				let world_pos = self.screen_to_world(pos);
				if let DragState::Selection { end, .. } = &mut self.drag_state {
					*end = world_pos;
				}
				self.cache.clear();
			}
			NodeGraphMessage::SelectionBoxEnd => {
				if let DragState::Selection { start, end } = &self.drag_state {
					let selection_rect = Rectangle {
						x: start.x.min(end.x),
						y: start.y.min(end.y),
						width: (start.x - end.x).abs(),
						height: (start.y - end.y).abs(),
					};

					self.selected_nodes.clear();
					for node in self.nodes.values_mut() {
						node.selected = selection_rect.intersects(&node.bounds());
						if node.selected {
							self.selected_nodes.push(node.id);
						}
					}
				}
				self.drag_state = DragState::None;
				self.cache.clear();
			}
			NodeGraphMessage::PanStart(pos) => {
				self.drag_state = DragState::Pan { last_pos: pos };
			}
			NodeGraphMessage::PanMove(pos) => {
				if let DragState::Pan { last_pos } = &mut self.drag_state {
					let delta = Vector::new(pos.x - last_pos.x, pos.y - last_pos.y);
					self.camera_offset.x += delta.x / self.zoom;
					self.camera_offset.y += delta.y / self.zoom;
					*last_pos = pos;
					self.cache.clear();
				}
			}
			NodeGraphMessage::PanEnd => {
				self.drag_state = DragState::None;
			}
			NodeGraphMessage::Zoom(delta, pos) => {
				let old_zoom = self.zoom;
				self.zoom = (self.zoom + delta * ZOOM_STEP).clamp(MIN_ZOOM, MAX_ZOOM);

				if old_zoom != self.zoom {
					// ズーム中心を維持
					let world_before = self.screen_to_world(pos);
					let world_after = Point::new(
						pos.x / self.zoom - self.camera_offset.x,
						pos.y / self.zoom - self.camera_offset.y,
					);
					self.camera_offset.x += world_after.x - world_before.x;
					self.camera_offset.y += world_after.y - world_before.y;
				}
				self.cache.clear();
			}
			NodeGraphMessage::PortHover(port_id) => {
				self.hovered_port = port_id;
				self.cache.clear();
			}
			NodeGraphMessage::ContextMenu(pos) => {
				self.show_context_menu = true;
				self.context_menu_pos = pos;
			}
			NodeGraphMessage::CloseContextMenu => {
				self.show_context_menu = false;
			}
			NodeGraphMessage::DeleteSelected => {
				for node_id in self.selected_nodes.drain(..) {
					self.nodes.remove(&node_id);
					self.connections
						.retain(|c| c.from_node != node_id && c.to_node != node_id);
				}
				self.cache.clear();
			}
			NodeGraphMessage::DuplicateSelected => {
				let selected: Vec<usize> = self.selected_nodes.clone();
				let mut id_map: HashMap<usize, usize> = HashMap::new();

				// ノードを複製
				for old_id in &selected {
					if let Some(old_node) = self.nodes.get(old_id) {
						let new_id = self.next_node_id;
						self.next_node_id += 1;
						let mut new_node = Node::new(
							new_id,
							old_node.node_type,
							Point::new(old_node.position.x + 50.0, old_node.position.y + 50.0),
						);
						new_node.selected = true;
						self.nodes.insert(new_id, new_node);
						id_map.insert(*old_id, new_id);
					}
				}

				// 接続を複製
				let mut new_connections = Vec::new();
				for conn in &self.connections {
					if let (Some(&new_from), Some(&new_to)) =
						(id_map.get(&conn.from_node), id_map.get(&conn.to_node))
					{
						new_connections.push(Connection::new(
							new_from,
							conn.from_port,
							new_to,
							conn.to_port,
						));
					}
				}
				self.connections.extend(new_connections);

				// 選択を更新
				for node in self.nodes.values_mut() {
					node.selected = id_map.values().any(|&id| id == node.id);
				}
				self.selected_nodes = id_map.values().copied().collect();
				self.cache.clear();
			}
			NodeGraphMessage::ResetView => {
				self.camera_offset = Vector::new(0.0, 0.0);
				self.zoom = 1.0;
				self.cache.clear();
			}
		}
	}

	fn screen_to_world(&self, screen_pos: Point) -> Point {
		Point::new(
			screen_pos.x / self.zoom - self.camera_offset.x,
			screen_pos.y / self.zoom - self.camera_offset.y,
		)
	}

	fn world_to_screen(&self, world_pos: Point) -> Point {
		Point::new(
			(world_pos.x + self.camera_offset.x) * self.zoom,
			(world_pos.y + self.camera_offset.y) * self.zoom,
		)
	}

	fn find_node_at(&self, world_pos: Point) -> Option<usize> {
		for node in self.nodes.values() {
			if node.bounds().contains(world_pos) {
				return Some(node.id);
			}
		}
		None
	}

	fn find_port_at(&self, world_pos: Point) -> Option<PortId> {
		for node in self.nodes.values() {
			if let Some((port_index, is_input)) = node.get_port_at(world_pos) {
				return Some(PortId::new(node.id, port_index, is_input));
			}
		}
		None
	}

	pub fn view(&self) -> Element<'_, NodeGraphMessage> {
		let canvas = Canvas::new(NodeGraphCanvas { graph: self })
			.width(Length::Fill)
			.height(Length::Fill);

		let content: Element<'_, NodeGraphMessage> = if self.show_context_menu {
			let menu = container(
				column![
					text("Add Node").size(12).color(colors::TEXT_SECONDARY),
					context_menu_item("Float Input", NodeType::FloatInput),
					context_menu_item("Color Input", NodeType::ColorInput),
					text("─────────").size(10).color(colors::TEXT_SECONDARY),
					context_menu_item("Add", NodeType::Add),
					context_menu_item("Subtract", NodeType::Subtract),
					context_menu_item("Multiply", NodeType::Multiply),
					context_menu_item("Divide", NodeType::Divide),
					context_menu_item("Sine", NodeType::Sine),
					context_menu_item("Cosine", NodeType::Cosine),
					text("─────────").size(10).color(colors::TEXT_SECONDARY),
					context_menu_item("Mix", NodeType::Mix),
					context_menu_item("Clamp", NodeType::Clamp),
					context_menu_item("Remap", NodeType::Remap),
					text("─────────").size(10).color(colors::TEXT_SECONDARY),
					context_menu_item("Output", NodeType::Output),
					context_menu_item("Preview", NodeType::Preview),
				]
				.spacing(2)
				.padding(8),
			)
			.style(|_| container::Style {
				background: Some(colors::NODE_BG.into()),
				border: iced::Border {
					color: colors::NODE_BORDER,
					width: 1.0,
					radius: 6.0.into(),
				},
				shadow: iced::Shadow {
					color: colors::NODE_SHADOW,
					offset: Vector::new(2.0, 4.0),
					blur_radius: 8.0,
				},
				..Default::default()
			});

			let overlay = iced::widget::stack![
				canvas,
				container(menu).padding(
					iced::Padding::new(self.context_menu_pos.y).left(self.context_menu_pos.x)
				)
			];

			overlay.into()
		} else {
			canvas.into()
		};

		// ツールバー
		let toolbar = row![
			button("Reset View")
				.on_press(NodeGraphMessage::ResetView)
				.padding([4, 12]),
			button("Delete")
				.on_press(NodeGraphMessage::DeleteSelected)
				.padding([4, 12]),
			button("Duplicate")
				.on_press(NodeGraphMessage::DuplicateSelected)
				.padding([4, 12]),
			text(format!("Zoom: {:.0}%", self.zoom * 100.0))
				.size(14)
				.color(colors::TEXT_SECONDARY),
			text(format!("Nodes: {}", self.nodes.len()))
				.size(14)
				.color(colors::TEXT_SECONDARY),
		]
		.spacing(8)
		.padding(8);

		column![
			container(toolbar)
				.style(|_| container::Style {
					background: Some(Color::from_rgb(0.15, 0.15, 0.17).into()),
					..Default::default()
				})
				.width(Length::Fill),
			content,
		]
		.into()
	}
}

fn context_menu_item(
	label: &'static str,
	node_type: NodeType,
) -> Element<'static, NodeGraphMessage> {
	button(text(label).size(13))
		.on_press(NodeGraphMessage::AddNode(node_type, Point::ORIGIN))
		.padding([4, 12])
		.width(Length::Fill)
		.style(|_theme, status| {
			let bg = match status {
				button::Status::Hovered | button::Status::Pressed => colors::NODE_HEADER_BG,
				_ => Color::TRANSPARENT,
			};
			button::Style {
				background: Some(bg.into()),
				text_color: colors::TEXT_PRIMARY,
				border: iced::Border::default(),
				..Default::default()
			}
		})
		.into()
}

// =============================================================================
// キャンバス描画
// =============================================================================

struct NodeGraphCanvas<'a> {
	graph: &'a NodeGraph,
}

impl<'a> canvas::Program<NodeGraphMessage> for NodeGraphCanvas<'a> {
	type State = ();

	fn update(
		&self,
		_state: &mut Self::State,
		event: Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> (canvas::event::Status, Option<NodeGraphMessage>) {
		let Some(cursor_pos) = cursor.position_in(bounds) else {
			return (canvas::event::Status::Ignored, None);
		};

		let world_pos = self.graph.screen_to_world(cursor_pos);

		match event {
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				// ポートのクリックをチェック
				if let Some(port_id) = self.graph.find_port_at(world_pos) {
					return (
						canvas::event::Status::Captured,
						Some(NodeGraphMessage::ConnectionStart(port_id, cursor_pos)),
					);
				}

				// ノードのクリックをチェック
				if let Some(node_id) = self.graph.find_node_at(world_pos) {
					return (
						canvas::event::Status::Captured,
						Some(NodeGraphMessage::NodeDragStart(node_id, cursor_pos)),
					);
				}

				// 空白領域のクリック - 選択ボックス開始
				return (
					canvas::event::Status::Captured,
					Some(NodeGraphMessage::SelectionBoxStart(cursor_pos)),
				);
			}
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
				return (
					canvas::event::Status::Captured,
					Some(NodeGraphMessage::ContextMenu(cursor_pos)),
				);
			}
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
				return (
					canvas::event::Status::Captured,
					Some(NodeGraphMessage::PanStart(cursor_pos)),
				);
			}
			Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				match &self.graph.drag_state {
					DragState::Node { .. } => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::NodeDragMove(cursor_pos)),
						);
					}
					DragState::Connection { .. } => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::ConnectionMove(cursor_pos)),
						);
					}
					DragState::Selection { .. } => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::SelectionBoxMove(cursor_pos)),
						);
					}
					DragState::Pan { .. } => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::PanMove(cursor_pos)),
						);
					}
					DragState::None => {
						// ポートホバーをチェック
						let hovered = self.graph.find_port_at(world_pos);
						if hovered != self.graph.hovered_port {
							return (
								canvas::event::Status::Captured,
								Some(NodeGraphMessage::PortHover(hovered)),
							);
						}
					}
				}
			}
			Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
				match &self.graph.drag_state {
					DragState::Node { .. } => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::NodeDragEnd),
						);
					}
					DragState::Connection { .. } => {
						let target_port = self.graph.find_port_at(world_pos);
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::ConnectionEnd(target_port)),
						);
					}
					DragState::Selection { .. } => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::SelectionBoxEnd),
						);
					}
					_ => {}
				}
			}
			Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
				if matches!(self.graph.drag_state, DragState::Pan { .. }) {
					return (
						canvas::event::Status::Captured,
						Some(NodeGraphMessage::PanEnd),
					);
				}
			}
			Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
				let scroll = match delta {
					mouse::ScrollDelta::Lines { y, .. } => y,
					mouse::ScrollDelta::Pixels { y, .. } => y / 50.0,
				};
				return (
					canvas::event::Status::Captured,
					Some(NodeGraphMessage::Zoom(scroll, cursor_pos)),
				);
			}
			Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
				match key.as_ref() {
					Key::Named(keyboard::key::Named::Delete)
					| Key::Named(keyboard::key::Named::Backspace) => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::DeleteSelected),
						);
					}
					Key::Character("d") if modifiers.command() => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::DuplicateSelected),
						);
					}
					Key::Named(keyboard::key::Named::Escape) => {
						return (
							canvas::event::Status::Captured,
							Some(NodeGraphMessage::CloseContextMenu),
						);
					}
					_ => {}
				}
			}
			_ => {}
		}

		(canvas::event::Status::Ignored, None)
	}

	fn draw(
		&self,
		_state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = Frame::new(renderer, bounds.size());

		// 背景
		frame.fill_rectangle(Point::ORIGIN, bounds.size(), colors::BACKGROUND);

		// グリッド
		self.draw_grid(&mut frame, bounds);

		// 接続線
		self.draw_connections(&mut frame);

		// ドラッグ中の接続線
		self.draw_dragging_connection(&mut frame);

		// ノード
		self.draw_nodes(&mut frame);

		// 選択ボックス
		self.draw_selection_box(&mut frame);

		vec![frame.into_geometry()]
	}
}

impl<'a> NodeGraphCanvas<'a> {
	fn draw_grid(&self, frame: &mut Frame, bounds: Rectangle) {
		let zoom = self.graph.zoom;
		let offset = self.graph.camera_offset;

		// 小さいグリッド
		let grid_size = GRID_SIZE * zoom;
		let start_x = (offset.x * zoom) % grid_size;
		let start_y = (offset.y * zoom) % grid_size;

		for i in 0..((bounds.width / grid_size) as i32 + 2) {
			let x = start_x + i as f32 * grid_size;
			frame.stroke(
				&Path::line(Point::new(x, 0.0), Point::new(x, bounds.height)),
				Stroke::default()
					.with_color(colors::GRID_LINE)
					.with_width(1.0),
			);
		}
		for i in 0..((bounds.height / grid_size) as i32 + 2) {
			let y = start_y + i as f32 * grid_size;
			frame.stroke(
				&Path::line(Point::new(0.0, y), Point::new(bounds.width, y)),
				Stroke::default()
					.with_color(colors::GRID_LINE)
					.with_width(1.0),
			);
		}

		// 大きいグリッド
		let grid_size_large = GRID_SIZE_LARGE * zoom;
		let start_x_large = (offset.x * zoom) % grid_size_large;
		let start_y_large = (offset.y * zoom) % grid_size_large;

		for i in 0..((bounds.width / grid_size_large) as i32 + 2) {
			let x = start_x_large + i as f32 * grid_size_large;
			frame.stroke(
				&Path::line(Point::new(x, 0.0), Point::new(x, bounds.height)),
				Stroke::default()
					.with_color(colors::GRID_LINE_LARGE)
					.with_width(1.0),
			);
		}
		for i in 0..((bounds.height / grid_size_large) as i32 + 2) {
			let y = start_y_large + i as f32 * grid_size_large;
			frame.stroke(
				&Path::line(Point::new(0.0, y), Point::new(bounds.width, y)),
				Stroke::default()
					.with_color(colors::GRID_LINE_LARGE)
					.with_width(1.0),
			);
		}
	}

	fn draw_connections(&self, frame: &mut Frame) {
		for connection in &self.graph.connections {
			if let (Some(from_node), Some(to_node)) = (
				self.graph.nodes.get(&connection.from_node),
				self.graph.nodes.get(&connection.to_node),
			) {
				let from_pos = self
					.graph
					.world_to_screen(from_node.port_position(connection.from_port, false));
				let to_pos = self
					.graph
					.world_to_screen(to_node.port_position(connection.to_port, true));

				self.draw_bezier_connection(frame, from_pos, to_pos, colors::CONNECTION);
			}
		}
	}

	fn draw_dragging_connection(&self, frame: &mut Frame) {
		if let DragState::Connection {
			from_port,
			current_pos,
		} = &self.graph.drag_state
		{
			if let Some(node) = self.graph.nodes.get(&from_port.node_id) {
				let port_pos = self
					.graph
					.world_to_screen(node.port_position(from_port.port_index, from_port.is_input));
				let target_pos = self.graph.world_to_screen(*current_pos);

				let (from, to) = if from_port.is_input {
					(target_pos, port_pos)
				} else {
					(port_pos, target_pos)
				};

				self.draw_bezier_connection(frame, from, to, colors::CONNECTION_ACTIVE);
			}
		}
	}

	fn draw_bezier_connection(&self, frame: &mut Frame, from: Point, to: Point, color: Color) {
		let control_offset = ((to.x - from.x).abs() / 2.0).max(50.0);
		let cp1 = Point::new(from.x + control_offset, from.y);
		let cp2 = Point::new(to.x - control_offset, to.y);

		let path = Path::new(|builder| {
			builder.move_to(from);
			builder.bezier_curve_to(cp1, cp2, to);
		});

		frame.stroke(
			&path,
			Stroke::default()
				.with_color(color)
				.with_width(CONNECTION_THICKNESS),
		);
	}

	fn draw_nodes(&self, frame: &mut Frame) {
		let zoom = self.graph.zoom;

		for node in self.graph.nodes.values() {
			let screen_pos = self.graph.world_to_screen(node.position);
			let width = NODE_WIDTH * zoom;
			let height = node.height() * zoom;

			// ノード影
			frame.fill_rectangle(
				Point::new(
					screen_pos.x + NODE_SHADOW_OFFSET * zoom,
					screen_pos.y + NODE_SHADOW_OFFSET * zoom,
				),
				Size::new(width, height),
				colors::NODE_SHADOW,
			);

			// ノード本体
			let node_path = Path::rectangle(screen_pos, Size::new(width, height));
			frame.fill(&node_path, colors::NODE_BG);

			// 選択枠
			if node.selected {
				frame.stroke(
					&node_path,
					Stroke::default()
						.with_color(colors::NODE_SELECTED_BORDER)
						.with_width(2.0),
				);
			} else {
				frame.stroke(
					&node_path,
					Stroke::default()
						.with_color(colors::NODE_BORDER)
						.with_width(1.0),
				);
			}

			// ヘッダー
			let header_path =
				Path::rectangle(screen_pos, Size::new(width, NODE_HEADER_HEIGHT * zoom));
			frame.fill(&header_path, node.node_type.header_color());

			// ヘッダーテキスト
			let header_text = Text {
				content: node.node_type.name().to_string(),
				position: Point::new(screen_pos.x + 10.0 * zoom, screen_pos.y + 6.0 * zoom),
				color: colors::NODE_HEADER_TEXT,
				size: iced::Pixels(14.0 * zoom),
				..Default::default()
			};
			frame.fill_text(header_text);

			// 入力ポート
			for (i, port) in node.inputs.iter().enumerate() {
				let port_screen_pos = self.graph.world_to_screen(node.port_position(i, true));
				let radius = NODE_PORT_RADIUS * zoom;

				// ポート円
				let is_hovered = self.graph.hovered_port == Some(port.id);
				let port_path = Path::circle(port_screen_pos, radius);
				frame.fill(&port_path, port.data_type.color());

				if is_hovered {
					frame.stroke(
						&port_path,
						Stroke::default()
							.with_color(colors::PORT_HOVER)
							.with_width(2.0),
					);
				}

				// ポート名
				let port_text = Text {
					content: port.name.clone(),
					position: Point::new(
						port_screen_pos.x + radius + 6.0 * zoom,
						port_screen_pos.y - 5.0 * zoom,
					),
					color: colors::TEXT_PRIMARY,
					size: iced::Pixels(11.0 * zoom),
					..Default::default()
				};
				frame.fill_text(port_text);
			}

			// 出力ポート
			for (i, port) in node.outputs.iter().enumerate() {
				let port_screen_pos = self.graph.world_to_screen(node.port_position(i, false));
				let radius = NODE_PORT_RADIUS * zoom;

				// ポート円
				let is_hovered = self.graph.hovered_port == Some(port.id);
				let port_path = Path::circle(port_screen_pos, radius);
				frame.fill(&port_path, port.data_type.color());

				if is_hovered {
					frame.stroke(
						&port_path,
						Stroke::default()
							.with_color(colors::PORT_HOVER)
							.with_width(2.0),
					);
				}

				// ポート名（右寄せ）
				let port_text = Text {
					content: port.name.clone(),
					position: Point::new(
						port_screen_pos.x - radius - 6.0 * zoom,
						port_screen_pos.y - 5.0 * zoom,
					),
					color: colors::TEXT_PRIMARY,
					size: iced::Pixels(11.0 * zoom),
					horizontal_alignment: iced::alignment::Horizontal::Right,
					..Default::default()
				};
				frame.fill_text(port_text);
			}
		}
	}

	fn draw_selection_box(&self, frame: &mut Frame) {
		if let DragState::Selection { start, end } = &self.graph.drag_state {
			let screen_start = self.graph.world_to_screen(*start);
			let screen_end = self.graph.world_to_screen(*end);

			let rect = Rectangle {
				x: screen_start.x.min(screen_end.x),
				y: screen_start.y.min(screen_end.y),
				width: (screen_start.x - screen_end.x).abs(),
				height: (screen_start.y - screen_end.y).abs(),
			};

			let path = Path::rectangle(
				Point::new(rect.x, rect.y),
				Size::new(rect.width, rect.height),
			);
			frame.fill(&path, colors::SELECTION_BOX);
			frame.stroke(
				&path,
				Stroke::default()
					.with_color(colors::SELECTION_BORDER)
					.with_width(1.0),
			);
		}
	}
}
