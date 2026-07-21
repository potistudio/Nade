//! Generic pan/zoom canvas widget for Iced
//!
//! Provides a canvas with:
//! - Pan (scroll) and zoom (Ctrl+scroll) functionality
//! - Optional grid drawing with origin axes
//! - Coordinate transformation between local and screen space

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

const SCROLL_MULTIPLIER: f32 = 1.0;
const ZOOM_SPEED: f32 = 0.1;
const MIN_SCALE: f32 = 0.1;
const MAX_SCALE: f32 = 10.0;

/// Grid drawing configuration
#[derive(Clone, Copy, Debug)]
pub struct GridConfig {
	/// Spacing between grid lines
	pub spacing: f32,
	/// Grid line color
	pub line_color: Color,
	/// Grid line width
	pub line_width: f32,
	/// Whether to show origin axes
	pub show_origin_axes: bool,
	/// Origin axis color
	pub origin_axis_color: Color,
}

impl Default for GridConfig {
	fn default() -> Self {
		Self {
			spacing: 20.0,
			line_color: Color::from_rgb8(32, 40, 52),
			line_width: 1.0,
			show_origin_axes: true,
			origin_axis_color: Color::from_rgb8(64, 140, 196),
		}
	}
}

/// Canvas state for pan/zoom operations
#[derive(Debug)]
pub struct CanvasState {
	/// Current zoom scale
	pub scale: f32,
	/// Current pan offset
	pub offset: Vector,
	/// Background color
	pub background_color: Color,
	/// Optional grid configuration
	pub grid_config: Option<GridConfig>,
	/// Cache for the grid geometry
	cache: canvas::Cache,
}

impl Default for CanvasState {
	fn default() -> Self {
		Self {
			scale: 1.0,
			offset: Vector::ZERO,
			background_color: Color::from_rgb8(28, 34, 42),
			grid_config: Some(GridConfig::default()),
			cache: canvas::Cache::default(),
		}
	}
}

impl CanvasState {
	/// Create a new canvas state
	pub fn new() -> Self {
		Self::default()
	}

	/// Set the background color
	pub fn with_background_color(mut self, color: Color) -> Self {
		self.background_color = color;
		self
	}

	/// Set the grid configuration (None to disable grid)
	pub fn with_grid(mut self, config: Option<GridConfig>) -> Self {
		self.grid_config = config;
		self
	}

	/// Convert local coordinates to screen coordinates
	pub fn local_to_screen(&self, local: Point, bounds: Rectangle) -> Point {
		let origin = bounds.position();
		Point::new(
			origin.x + local.x * self.scale + self.offset.x,
			origin.y + local.y * self.scale + self.offset.y,
		)
	}

	/// Convert screen coordinates to local coordinates
	pub fn screen_to_local(&self, screen: Point, bounds: Rectangle) -> Point {
		let origin = bounds.position();
		Point::new(
			(screen.x - origin.x - self.offset.x) / self.scale,
			(screen.y - origin.y - self.offset.y) / self.scale,
		)
	}

	/// Get the visible bounds in local coordinates
	pub fn visible_bounds(&self, bounds: Rectangle) -> (f32, f32, f32, f32) {
		let top_left = self.screen_to_local(bounds.position(), bounds);
		let bottom_right = self.screen_to_local(Point::new(bounds.x + bounds.width, bounds.y + bounds.height), bounds);

		let min_x = top_left.x.min(bottom_right.x).floor();
		let max_x = top_left.x.max(bottom_right.x).ceil();
		let min_y = top_left.y.min(bottom_right.y).floor();
		let max_y = top_left.y.max(bottom_right.y).ceil();

		(min_x, max_x, min_y, max_y)
	}

	/// Handle scroll/zoom events
	pub fn handle_scroll(&mut self, delta: Vector, _cursor_position: Option<Point>, _bounds: Rectangle) {
		// Pan with scroll
		self.offset.x += delta.x * SCROLL_MULTIPLIER;
		self.offset.y += delta.y * SCROLL_MULTIPLIER;
		self.cache.clear();
	}

	/// Handle zoom events
	pub fn handle_zoom(&mut self, zoom_delta: f32, cursor_position: Option<Point>, bounds: Rectangle) {
		if let Some(cursor) = cursor_position {
			let old_scale = self.scale;
			let zoom_factor = 1.0 + zoom_delta * ZOOM_SPEED;
			self.scale = (self.scale * zoom_factor).clamp(MIN_SCALE, MAX_SCALE);

			// Zoom around cursor position
			let cursor_local = self.screen_to_local(cursor, bounds);
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
			self.cache.clear();
		}
	}

	/// Clear the geometry cache (call when content changes)
	pub fn request_redraw(&mut self) {
		self.cache.clear();
	}
}

/// Trait for custom canvas drawing
///
/// Implement this trait to draw custom content on the canvas.
/// The draw method receives a CanvasContext for coordinate-aware drawing.
pub trait CanvasProgram {
	/// The message type for canvas events
	type Message: Clone;

	/// Draw custom content on the canvas
	fn draw_content(&self, frame: &mut Frame, bounds: Rectangle, state: &CanvasState);

	/// Handle canvas events (optional)
	fn update(
		&mut self,
		_event: canvas::Event,
		_bounds: Rectangle,
		_cursor: mouse::Cursor,
		_state: &mut CanvasState,
	) -> Option<Self::Message> {
		None
	}
}

/// Internal canvas program wrapper
struct CanvasWrapper<'a, P: CanvasProgram> {
	program: &'a P,
	state: &'a CanvasState,
}

impl<P: CanvasProgram> canvas::Program<P::Message> for CanvasWrapper<'_, P> {
	type State = ();

	fn draw(
		&self,
		_internal_state: &Self::State,
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let geometry = self.state.cache.draw(renderer, bounds.size(), |frame| {
			// Draw background
			frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.state.background_color);

			// Draw grid if configured
			if let Some(ref grid) = self.state.grid_config {
				self.draw_grid(frame, bounds, grid);
			}

			// Draw custom content
			self.program.draw_content(frame, bounds, self.state);
		});

		vec![geometry]
	}
}

impl<P: CanvasProgram> CanvasWrapper<'_, P> {
	fn draw_grid(&self, frame: &mut Frame, bounds: Rectangle, grid: &GridConfig) {
		let (min_x, max_x, min_y, max_y) = self.state.visible_bounds(bounds);

		let grid_stroke = Stroke::default()
			.with_width(grid.line_width)
			.with_color(grid.line_color);

		// Draw vertical lines
		let start_x = (min_x / grid.spacing).floor() * grid.spacing;
		let mut x = start_x;
		while x <= max_x {
			let p1 = self.local_to_frame(Point::new(x, min_y), bounds);
			let p2 = self.local_to_frame(Point::new(x, max_y), bounds);
			frame.stroke(&Path::line(p1, p2), grid_stroke);
			x += grid.spacing;
		}

		// Draw horizontal lines
		let start_y = (min_y / grid.spacing).floor() * grid.spacing;
		let mut y = start_y;
		while y <= max_y {
			let p1 = self.local_to_frame(Point::new(min_x, y), bounds);
			let p2 = self.local_to_frame(Point::new(max_x, y), bounds);
			frame.stroke(&Path::line(p1, p2), grid_stroke);
			y += grid.spacing;
		}

		// Draw origin axes
		if grid.show_origin_axes {
			let axis_stroke = Stroke::default().with_width(1.5).with_color(grid.origin_axis_color);

			// X axis (y = 0)
			if 0.0 >= min_y && 0.0 <= max_y {
				let p1 = self.local_to_frame(Point::new(min_x, 0.0), bounds);
				let p2 = self.local_to_frame(Point::new(max_x, 0.0), bounds);
				frame.stroke(&Path::line(p1, p2), axis_stroke);
			}

			// Y axis (x = 0)
			if 0.0 >= min_x && 0.0 <= max_x {
				let p1 = self.local_to_frame(Point::new(0.0, min_y), bounds);
				let p2 = self.local_to_frame(Point::new(0.0, max_y), bounds);
				frame.stroke(&Path::line(p1, p2), axis_stroke);
			}
		}
	}

	/// Convert local coordinates to frame coordinates (relative to frame origin)
	fn local_to_frame(&self, local: Point, _bounds: Rectangle) -> Point {
		Point::new(
			local.x * self.state.scale + self.state.offset.x,
			local.y * self.state.scale + self.state.offset.y,
		)
	}
}

/// Create a canvas element from a CanvasProgram
pub fn canvas_view<'a, P: CanvasProgram + 'a>(program: &'a P, state: &'a CanvasState) -> Element<'a, P::Message>
where
	P::Message: 'a,
{
	let wrapper = CanvasWrapper { program, state };
	Canvas::new(wrapper).width(Length::Fill).height(Length::Fill).into()
}

// ============================================================================
// Drawing helper functions for use in CanvasProgram::draw_content
// ============================================================================

/// Draw a line in local coordinates
pub fn draw_line(frame: &mut Frame, state: &CanvasState, _bounds: Rectangle, p1: Point, p2: Point, stroke: Stroke) {
	let sp1 = local_to_frame(state, p1);
	let sp2 = local_to_frame(state, p2);
	frame.stroke(&Path::line(sp1, sp2), stroke);
}

/// Draw a filled circle in local coordinates
pub fn draw_circle_filled(
	frame: &mut Frame,
	state: &CanvasState,
	_bounds: Rectangle,
	center: Point,
	radius: f32,
	color: Color,
) {
	let screen_center = local_to_frame(state, center);
	let screen_radius = radius * state.scale;
	frame.fill(&Path::circle(screen_center, screen_radius), color);
}

/// Draw a circle stroke in local coordinates
pub fn draw_circle_stroke(
	frame: &mut Frame,
	state: &CanvasState,
	_bounds: Rectangle,
	center: Point,
	radius: f32,
	stroke: Stroke,
) {
	let screen_center = local_to_frame(state, center);
	let screen_radius = radius * state.scale;
	frame.stroke(&Path::circle(screen_center, screen_radius), stroke);
}

/// Draw a filled rectangle in local coordinates
pub fn draw_rect_filled(frame: &mut Frame, state: &CanvasState, _bounds: Rectangle, rect: Rectangle, color: Color) {
	let top_left = local_to_frame(state, rect.position());
	let size = Size::new(rect.width * state.scale, rect.height * state.scale);
	frame.fill_rectangle(top_left, size, color);
}

/// Draw a rectangle stroke in local coordinates
pub fn draw_rect_stroke(frame: &mut Frame, state: &CanvasState, _bounds: Rectangle, rect: Rectangle, stroke: Stroke) {
	let top_left = local_to_frame(state, rect.position());
	let size = Size::new(rect.width * state.scale, rect.height * state.scale);
	let path = Path::rectangle(top_left, size);
	frame.stroke(&path, stroke);
}

/// Convert local coordinates to frame coordinates
fn local_to_frame(state: &CanvasState, local: Point) -> Point {
	Point::new(
		local.x * state.scale + state.offset.x,
		local.y * state.scale + state.offset.y,
	)
}
