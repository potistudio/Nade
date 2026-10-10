//! Gizmos - Bezier handle editor widget for Iced
//!
//! Provides interactive editing of bezier curve control points with:
//! - Draggable center points
//! - Draggable left/right control handles (mirrored)
//! - Cubic bezier curve rendering
//! - Pan support via canvas

use iced::keyboard;
use iced::mouse;
use iced::widget::Action;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Event, Length, Point, Rectangle, Renderer, Theme, Vector};

// =============================================================================
// Constants
// =============================================================================

const GRID_BASE_SPACING: f32 = 20.0;
const GRID_LEVELS: [f32; 4] = [1.0, 5.0, 10.0, 50.0]; // Multipliers for grid spacing levels
const GRID_BASE_COLOR: (f32, f32, f32) = (0.18, 0.18, 0.18);
const GRID_MAX_ALPHA: f32 = 0.8;
const GRID_MIN_SCREEN_SPACING: f32 = 8.0; // Minimum screen pixels between grid lines before fading
const GRID_FADE_RANGE: f32 = 16.0; // Screen pixels range for fading
const ORIGIN_AXIS_COLOR: Color = Color::from_rgb8(64, 140, 196);
const BACKGROUND_COLOR: Color = Color::from_rgb8(28, 34, 42);

const HANDLE_RADIUS: f32 = 5.0;
const CONTROL_POINT_RADIUS: f32 = 3.5;
const BEZIER_SEGMENTS: usize = 64;
const HIT_THRESHOLD: f32 = 12.0;

const BEZIER_COLOR: Color = Color::from_rgb8(72, 196, 220);
const HANDLE_COLOR: Color = Color::from_rgb8(220, 228, 236);
const CONTROL_LINE_COLOR: Color = Color::from_rgb8(140, 152, 168);

const ZOOM_SPEED: f32 = 0.01;
const MIN_SCALE: f32 = 0.01;
const MAX_SCALE: f32 = 100.0;

// =============================================================================
// Bezier Math
// =============================================================================

fn cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
	let it = 1.0 - t;
	let x = it.powi(3) * p0.x + 3.0 * it.powi(2) * t * p1.x + 3.0 * it * t.powi(2) * p2.x + t.powi(3) * p3.x;
	let y = it.powi(3) * p0.y + 3.0 * it.powi(2) * t * p1.y + 3.0 * it * t.powi(2) * p2.y + t.powi(3) * p3.y;
	Point::new(x, y)
}

// =============================================================================
// Handle Part (which part of handle is being dragged)
// =============================================================================

#[derive(Clone, Copy, Debug)]
enum HandlePart {
	Center,
	LeftControl,
	RightControl,
}

// =============================================================================
// Bezier Handle
// =============================================================================

#[derive(Clone, Copy, Debug)]
pub struct BezierHandle {
	pub id: usize,
	pos: Point,
	left_point: Point,
	right_point: Point,
}

impl BezierHandle {
	pub fn new(id: usize, pos: Point) -> Self {
		Self {
			id,
			pos,
			left_point: Point::new(pos.x - 40.0, pos.y),
			right_point: Point::new(pos.x + 40.0, pos.y),
		}
	}

	pub fn set_position(&mut self, pos: Point) {
		let delta = Vector::new(pos.x - self.pos.x, pos.y - self.pos.y);
		self.pos = pos;
		self.left_point = Point::new(self.left_point.x + delta.x, self.left_point.y + delta.y);
		self.right_point = Point::new(self.right_point.x + delta.x, self.right_point.y + delta.y);
	}

	pub fn position(&self) -> Point {
		self.pos
	}

	pub fn set_left_point(&mut self, point: Point) {
		self.left_point = point;
		// Mirror to right
		let dx = self.left_point.x - self.pos.x;
		let dy = self.left_point.y - self.pos.y;
		self.right_point = Point::new(self.pos.x - dx, self.pos.y - dy);
	}

	pub fn set_right_point(&mut self, point: Point) {
		self.right_point = point;
		// Mirror to left
		let dx = self.right_point.x - self.pos.x;
		let dy = self.right_point.y - self.pos.y;
		self.left_point = Point::new(self.pos.x - dx, self.pos.y - dy);
	}

	pub fn left_point(&self) -> Point {
		self.left_point
	}

	pub fn right_point(&self) -> Point {
		self.right_point
	}
}

// =============================================================================
// Drag State
// =============================================================================

#[derive(Clone, Debug)]
struct DragState {
	handle_id: usize,
	part: HandlePart,
	offset: Vector,
}

// =============================================================================
// Canvas State (for pan/zoom)
// =============================================================================

#[derive(Debug)]
struct CanvasTransform {
	scale: f32,
	offset: Vector,
}

impl Default for CanvasTransform {
	fn default() -> Self {
		Self {
			scale: 1.0,
			offset: Vector::ZERO,
		}
	}
}

impl CanvasTransform {
	#[allow(dead_code)]
	fn local_to_screen(&self, local: Point) -> Point {
		Point::new(
			local.x * self.scale + self.offset.x,
			local.y * self.scale + self.offset.y,
		)
	}

	fn screen_to_local(&self, screen: Point) -> Point {
		Point::new(
			(screen.x - self.offset.x) / self.scale,
			(screen.y - self.offset.y) / self.scale,
		)
	}

	/// Handle zoom with scroll delta, zooming around the cursor position
	fn handle_zoom(&mut self, delta: f32, cursor_position: Point) {
		let old_scale = self.scale;
		let zoom_factor = 1.0 + delta * ZOOM_SPEED;
		self.scale = (self.scale * zoom_factor).clamp(MIN_SCALE, MAX_SCALE);

		// Zoom around cursor position
		let cursor_local = self.screen_to_local(cursor_position);
		let local_before = Vector::new(
			cursor_local.x * old_scale + self.offset.x,
			cursor_local.y * old_scale + self.offset.y,
		);
		let local_after = Vector::new(
			cursor_local.x * self.scale + self.offset.x,
			cursor_local.y * self.scale + self.offset.y,
		);

		self.offset.x += local_before.x - local_after.x;
		self.offset.y += local_before.y - local_after.y;
	}
}

// =============================================================================
// Gizmos Application State
// =============================================================================

#[derive(Debug)]
pub struct GizmosApp {
	handles: Vec<BezierHandle>,
	active_handle: Option<usize>,
	drag_state: Option<DragState>,
	transform: CanvasTransform,
	cache: canvas::Cache,
}

impl Default for GizmosApp {
	fn default() -> Self {
		Self {
			handles: vec![
				BezierHandle::new(0, Point::new(100.0, 300.0)),
				BezierHandle::new(1, Point::new(300.0, 150.0)),
				BezierHandle::new(2, Point::new(500.0, 350.0)),
				BezierHandle::new(3, Point::new(700.0, 200.0)),
			],
			active_handle: None,
			drag_state: None,
			transform: CanvasTransform::default(),
			cache: canvas::Cache::default(),
		}
	}
}

// =============================================================================
// Messages
// =============================================================================

#[derive(Debug, Clone)]
pub enum Message {
	DragStarted(Point),
	Dragging(Point),
	DragEnded,
	Scroll(Vector),
	Zoom { delta: f32, cursor_pos: Point },
}

// =============================================================================
// Application Implementation
// =============================================================================

impl GizmosApp {
	pub fn default() -> Self {
		Default::default()
	}

	pub fn update(&mut self, message: Message) {
		match message {
			Message::DragStarted(pos) => {
				let local_pos = self.transform.screen_to_local(pos);
				let threshold = HIT_THRESHOLD / self.transform.scale;

				if let Some((handle_id, part, offset)) = self.find_nearest_handle(local_pos, threshold) {
					self.active_handle = Some(handle_id);
					self.drag_state = Some(DragState {
						handle_id,
						part,
						offset,
					});
					self.cache.clear();
				}
			}
			Message::Dragging(pos) => {
				if let Some(ref drag) = self.drag_state {
					let local_pos = self.transform.screen_to_local(pos);
					let target_pos = Point::new(local_pos.x + drag.offset.x, local_pos.y + drag.offset.y);

					if let Some(handle) = self.handles.iter_mut().find(|h| h.id == drag.handle_id) {
						match drag.part {
							HandlePart::Center => handle.set_position(target_pos),
							HandlePart::LeftControl => handle.set_left_point(target_pos),
							HandlePart::RightControl => handle.set_right_point(target_pos),
						}
						self.cache.clear();
					}
				}
			}
			Message::DragEnded => {
				self.active_handle = None;
				self.drag_state = None;
			}
			Message::Scroll(delta) => {
				self.transform.offset.x += delta.x;
				self.transform.offset.y += delta.y;
				self.cache.clear();
			}
			Message::Zoom { delta, cursor_pos } => {
				self.transform.handle_zoom(delta, cursor_pos);
				self.cache.clear();
			}
		}
	}

	fn find_nearest_handle(&self, from: Point, radius: f32) -> Option<(usize, HandlePart, Vector)> {
		let r2 = radius * radius;
		let mut best: Option<(usize, HandlePart, Vector, f32)> = None;

		for h in &self.handles {
			// Center
			let d2_center = distance_squared(h.position(), from);
			if d2_center < r2 && best.as_ref().is_none_or(|b| d2_center < b.3) {
				let offset = Vector::new(h.position().x - from.x, h.position().y - from.y);
				best = Some((h.id, HandlePart::Center, offset, d2_center));
			}

			// Left control
			let d2_left = distance_squared(h.left_point(), from);
			if d2_left < r2 && best.as_ref().is_none_or(|b| d2_left < b.3) {
				let offset = Vector::new(h.left_point().x - from.x, h.left_point().y - from.y);
				best = Some((h.id, HandlePart::LeftControl, offset, d2_left));
			}

			// Right control
			let d2_right = distance_squared(h.right_point(), from);
			if d2_right < r2 && best.as_ref().is_none_or(|b| d2_right < b.3) {
				let offset = Vector::new(h.right_point().x - from.x, h.right_point().y - from.y);
				best = Some((h.id, HandlePart::RightControl, offset, d2_right));
			}
		}

		best.map(|(id, part, offset, _)| (id, part, offset))
	}

	pub fn view(&self) -> Element<'_, Message> {
		Canvas::new(GizmosCanvasWithState {
			handles: &self.handles,
			active_handle: self.active_handle,
			transform: &self.transform,
			cache: &self.cache,
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
	}
}

fn distance_squared(a: Point, b: Point) -> f32 {
	let dx = a.x - b.x;
	let dy = a.y - b.y;
	dx * dx + dy * dy
}

/// Extended canvas wrapper that tracks modifier state
struct GizmosCanvasWithState<'a> {
	handles: &'a [BezierHandle],
	active_handle: Option<usize>,
	transform: &'a CanvasTransform,
	cache: &'a canvas::Cache,
}

/// Canvas state that tracks keyboard modifiers
#[derive(Default)]
pub struct GizmosCanvasState {
	modifiers: keyboard::Modifiers,
}

impl<'a> canvas::Program<Message> for GizmosCanvasWithState<'a> {
	type State = GizmosCanvasState;

	fn update(
		&self,
		state: &mut Self::State,
		event: &Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Option<Action<Message>> {
		match event {
			Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
				state.modifiers = *modifiers;
				None
			}
			Event::Mouse(mouse_event) => {
				let cursor_pos = cursor.position_in(bounds)?;

				match mouse_event {
					mouse::Event::ButtonPressed(mouse::Button::Left) => {
						Some(Action::publish(Message::DragStarted(cursor_pos)))
					}
					mouse::Event::CursorMoved { .. } => Some(Action::publish(Message::Dragging(cursor_pos))),
					mouse::Event::ButtonReleased(mouse::Button::Left) => Some(Action::publish(Message::DragEnded)),
					mouse::Event::WheelScrolled { delta } => {
						let (dx, dy) = match delta {
							mouse::ScrollDelta::Lines { x, y } => (*x * 20.0, *y * 20.0),
							mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
						};

						if state.modifiers.control() {
							// Ctrl + scroll = zoom
							Some(Action::publish(Message::Zoom { delta: dy, cursor_pos }))
						} else if state.modifiers.shift() {
							// Shift + scroll = horizontal scroll
							Some(Action::publish(Message::Scroll(Vector::new(dy, 0.0))))
						} else {
							// Normal scroll = pan
							Some(Action::publish(Message::Scroll(Vector::new(dx, dy))))
						}
					}
					_ => None,
				}
			}
			_ => None,
		}
	}

	fn draw(
		&self,
		_state: &Self::State,
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
			// Background
			frame.fill_rectangle(Point::ORIGIN, bounds.size(), BACKGROUND_COLOR);

			// Grid
			self.draw_grid(frame, bounds);

			// Bezier curves
			self.draw_bezier_curves(frame);

			// Control lines and handles
			self.draw_handles(frame);
		});

		vec![geometry]
	}
}

impl GizmosCanvasWithState<'_> {
	fn draw_grid(&self, frame: &mut Frame, bounds: Rectangle) {
		let (min_x, max_x, min_y, max_y) = self.visible_bounds(bounds);
		let scale = self.transform.scale;

		// Draw grid levels from largest spacing to smallest (so smaller grids draw on top)
		for &level_multiplier in GRID_LEVELS.iter().rev() {
			let spacing = GRID_BASE_SPACING * level_multiplier;
			let screen_spacing = spacing * scale;

			// Calculate alpha based on screen spacing
			// Grid fades in when screen spacing goes from GRID_MIN_SCREEN_SPACING to GRID_MIN_SCREEN_SPACING + GRID_FADE_RANGE
			let alpha = if screen_spacing < GRID_MIN_SCREEN_SPACING {
				0.0 // Too dense, don't draw
			} else if screen_spacing < GRID_MIN_SCREEN_SPACING + GRID_FADE_RANGE {
				// Fade in
				((screen_spacing - GRID_MIN_SCREEN_SPACING) / GRID_FADE_RANGE) * GRID_MAX_ALPHA
			} else {
				GRID_MAX_ALPHA
			};

			if alpha <= 0.001 {
				continue; // Skip this level entirely
			}

			// Higher level grids get slightly brighter
			let brightness_boost = (level_multiplier.log10() * 0.1).min(0.15);
			let grid_color = Color::from_rgba(
				(GRID_BASE_COLOR.0 + brightness_boost).min(1.0),
				(GRID_BASE_COLOR.1 + brightness_boost).min(1.0),
				(GRID_BASE_COLOR.2 + brightness_boost).min(1.0),
				alpha,
			);

			let line_width = if level_multiplier >= 10.0 { 1.0 } else { 0.5 };
			let grid_stroke = Stroke::default().with_width(line_width).with_color(grid_color);

			// Draw vertical lines
			let start_x = (min_x / spacing).floor() * spacing;
			let mut x = start_x;
			while x <= max_x {
				let p1 = self.local_to_frame(Point::new(x, min_y));
				let p2 = self.local_to_frame(Point::new(x, max_y));
				frame.stroke(&Path::line(p1, p2), grid_stroke);
				x += spacing;
			}

			// Draw horizontal lines
			let start_y = (min_y / spacing).floor() * spacing;
			let mut y = start_y;
			while y <= max_y {
				let p1 = self.local_to_frame(Point::new(min_x, y));
				let p2 = self.local_to_frame(Point::new(max_x, y));
				frame.stroke(&Path::line(p1, p2), grid_stroke);
				y += spacing;
			}
		}

		// Origin axes (always visible)
		let axis_stroke = Stroke::default().with_width(1.5).with_color(ORIGIN_AXIS_COLOR);

		if 0.0 >= min_y && 0.0 <= max_y {
			let p1 = self.local_to_frame(Point::new(min_x, 0.0));
			let p2 = self.local_to_frame(Point::new(max_x, 0.0));
			frame.stroke(&Path::line(p1, p2), axis_stroke);
		}

		if 0.0 >= min_x && 0.0 <= max_x {
			let p1 = self.local_to_frame(Point::new(0.0, min_y));
			let p2 = self.local_to_frame(Point::new(0.0, max_y));
			frame.stroke(&Path::line(p1, p2), axis_stroke);
		}
	}

	fn draw_bezier_curves(&self, frame: &mut Frame) {
		if self.handles.len() < 2 {
			return;
		}

		let bezier_stroke = Stroke::default().with_width(2.5).with_color(BEZIER_COLOR);

		for i in 0..self.handles.len() - 1 {
			let p0 = self.handles[i].position();
			let p1 = self.handles[i].right_point();
			let p2 = self.handles[i + 1].left_point();
			let p3 = self.handles[i + 1].position();

			let path = Path::new(|builder| {
				let start = self.local_to_frame(p0);
				builder.move_to(start);

				for j in 1..=BEZIER_SEGMENTS {
					let t = j as f32 / BEZIER_SEGMENTS as f32;
					let point = cubic_bezier(p0, p1, p2, p3, t);
					let screen_point = self.local_to_frame(point);
					builder.line_to(screen_point);
				}
			});

			frame.stroke(&path, bezier_stroke);
		}
	}

	fn draw_handles(&self, frame: &mut Frame) {
		let control_stroke = Stroke::default().with_width(1.0).with_color(CONTROL_LINE_COLOR);

		for h in self.handles {
			let center = self.local_to_frame(h.position());
			let left = self.local_to_frame(h.left_point());
			let right = self.local_to_frame(h.right_point());

			// Control lines
			frame.stroke(&Path::line(center, left), control_stroke);
			frame.stroke(&Path::line(center, right), control_stroke);

			// Control points (hollow circles)
			let control_stroke_thick = Stroke::default().with_width(2.0).with_color(HANDLE_COLOR);
			frame.stroke(&Path::circle(left, CONTROL_POINT_RADIUS), control_stroke_thick);
			frame.stroke(&Path::circle(right, CONTROL_POINT_RADIUS), control_stroke_thick);

			// Center point (filled)
			frame.fill(&Path::circle(center, HANDLE_RADIUS), HANDLE_COLOR);
		}

		// Highlight active handle
		if let Some(active_id) = self.active_handle
			&& let Some(h) = self.handles.iter().find(|h| h.id == active_id)
		{
			let center = self.local_to_frame(h.position());
			let highlight_stroke = Stroke::default()
				.with_width(2.0)
				.with_color(Color::from_rgb8(72, 196, 220));
			frame.stroke(&Path::circle(center, HANDLE_RADIUS + 3.0), highlight_stroke);
		}
	}

	fn local_to_frame(&self, local: Point) -> Point {
		Point::new(
			local.x * self.transform.scale + self.transform.offset.x,
			local.y * self.transform.scale + self.transform.offset.y,
		)
	}

	fn visible_bounds(&self, bounds: Rectangle) -> (f32, f32, f32, f32) {
		let top_left = self.transform.screen_to_local(Point::ORIGIN);
		let bottom_right = self.transform.screen_to_local(Point::new(bounds.width, bounds.height));

		let min_x = top_left.x.min(bottom_right.x).floor();
		let max_x = top_left.x.max(bottom_right.x).ceil();
		let min_y = top_left.y.min(bottom_right.y).floor();
		let max_y = top_left.y.max(bottom_right.y).ceil();

		(min_x, max_x, min_y, max_y)
	}
}
