//! 汎用的なパン・ズーム対応キャンバスウィジェット

use egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

const SCROLL_MULTIPLIER: f32 = 1.0;

/// グリッド描画設定
#[derive(Clone, Copy, Debug)]
pub struct GridConfig {
	pub spacing: f32,
	pub stroke: Stroke,
	pub show_origin_axes: bool,
	pub origin_axis_color: Color32,
}

impl Default for GridConfig {
	fn default() -> Self {
		Self {
			spacing: 20.0,
			stroke: Stroke {
				width: 1.0,
				color: Color32::from_gray(26),
			},
			show_origin_axes: true,
			origin_axis_color: Color32::from_rgb(120, 200, 255),
		}
	}
}

/// キャンバスの描画コンテキスト
/// 描画コールバック内で使用される
#[allow(dead_code)]
pub struct CanvasContext<'a> {
	pub painter: &'a egui::Painter,
	pub rect: Rect,
	pub origin: Pos2,
	pub scale: f32,
	pub offset: Vec2,
}

#[allow(dead_code)]
impl CanvasContext<'_> {
	/// ローカル座標をスクリーン座標に変換
	pub fn local_to_screen(&self, local: Pos2) -> Pos2 {
		let screen_local = Pos2::new(
			local.x * self.scale + self.offset.x,
			local.y * self.scale + self.offset.y,
		);
		self.origin + screen_local.to_vec2()
	}

	/// スケールを適用せずにローカル座標をスクリーン座標に変換（テキスト描画用）
	pub fn local_to_screen_unscaled(&self, local: Pos2) -> Pos2 {
		let screen_local = Pos2::new(local.x + self.offset.x, local.y + self.offset.y);
		self.origin + screen_local.to_vec2()
	}

	/// スクリーン座標をローカル座標に変換
	pub fn screen_to_local(&self, screen: Pos2) -> Pos2 {
		let local = screen - self.origin;
		Pos2::new(
			(local.x - self.offset.x) / self.scale,
			(local.y - self.offset.y) / self.scale,
		)
	}

	/// 可視領域のローカル座標での範囲を取得
	pub fn visible_bounds(&self) -> (f32, f32, f32, f32) {
		let top_left = self.screen_to_local(self.rect.left_top());
		let bottom_right = self.screen_to_local(self.rect.right_bottom());

		let min_x = top_left.x.min(bottom_right.x).floor();
		let max_x = top_left.x.max(bottom_right.x).ceil();
		let min_y = top_left.y.min(bottom_right.y).floor();
		let max_y = top_left.y.max(bottom_right.y).ceil();

		(min_x, max_x, min_y, max_y)
	}

	// =========================================================================
	// 描画メソッド（ローカル座標系で描画）
	// =========================================================================

	/// 線分を描画（ローカル座標）
	pub fn line(&self, p1: Pos2, p2: Pos2, stroke: Stroke) {
		let sp1 = self.local_to_screen(p1);
		let sp2 = self.local_to_screen(p2);
		self.painter.line_segment([sp1, sp2], stroke);
	}

	/// 塗りつぶし円を描画（ローカル座標）
	pub fn circle_filled(&self, center: Pos2, radius: f32, color: Color32) {
		let screen_center = self.local_to_screen(center);
		let screen_radius = radius * self.scale;
		self.painter
			.circle_filled(screen_center, screen_radius, color);
	}

	/// 円のストロークを描画（ローカル座標）
	pub fn circle_stroke(&self, center: Pos2, radius: f32, stroke: Stroke) {
		let screen_center = self.local_to_screen(center);
		let screen_radius = radius * self.scale;
		self.painter
			.circle_stroke(screen_center, screen_radius, stroke);
	}

	/// 円を描画（ローカル座標、塗りつぶし＋ストローク）
	pub fn circle(&self, center: Pos2, radius: f32, fill: Color32, stroke: Stroke) {
		let screen_center = self.local_to_screen(center);
		let screen_radius = radius * self.scale;
		self.painter
			.circle(screen_center, screen_radius, fill, stroke);
	}

	/// 塗りつぶし矩形を描画（ローカル座標）
	pub fn rect_filled(&self, rect: Rect, rounding: impl Into<egui::CornerRadius>, color: Color32) {
		let screen_rect = Rect::from_two_pos(
			self.local_to_screen(rect.left_top()),
			self.local_to_screen(rect.right_bottom()),
		);
		self.painter.rect_filled(screen_rect, rounding, color);
	}

	/// 矩形のストロークを描画（ローカル座標）
	pub fn rect_stroke(
		&self,
		rect: Rect,
		rounding: impl Into<egui::CornerRadius>,
		stroke: impl Into<Stroke>,
	) {
		let screen_rect = Rect::from_two_pos(
			self.local_to_screen(rect.left_top()),
			self.local_to_screen(rect.right_bottom()),
		);
		self.painter
			.rect_stroke(screen_rect, rounding, stroke, StrokeKind::Outside);
	}

	/// 矩形を描画（ローカル座標、塗りつぶし＋ストローク）
	pub fn rect(
		&self,
		rect: Rect,
		rounding: impl Into<egui::CornerRadius>,
		fill: Color32,
		stroke: impl Into<Stroke>,
	) {
		let screen_rect = Rect::from_two_pos(
			self.local_to_screen(rect.left_top()),
			self.local_to_screen(rect.right_bottom()),
		);
		self.painter
			.rect(screen_rect, rounding, fill, stroke, StrokeKind::Outside);
	}

	/// テキストを描画（ローカル座標、スケール適用）
	pub fn text(
		&self,
		pos: Pos2,
		anchor: egui::Align2,
		text: impl ToString,
		font_id: egui::FontId,
		color: Color32,
	) {
		let screen_pos = self.local_to_screen(pos);
		self.painter.text(screen_pos, anchor, text, font_id, color);
	}

	/// テキストを描画（ローカル座標、スケール非適用 - 常に同じサイズ）
	pub fn text_unscaled(
		&self,
		pos: Pos2,
		anchor: egui::Align2,
		text: impl ToString,
		font_id: egui::FontId,
		color: Color32,
	) {
		let screen_pos = self.local_to_screen_unscaled(pos);
		self.painter.text(screen_pos, anchor, text, font_id, color);
	}

	/// 複数の線分を描画（ローカル座標）
	pub fn line_segments(&self, points: &[Pos2], stroke: Stroke) {
		for window in points.windows(2) {
			self.line(window[0], window[1], stroke);
		}
	}

	/// 閉じた多角形を描画（ローカル座標）
	pub fn polygon_stroke(&self, points: &[Pos2], stroke: Stroke) {
		if points.len() < 2 {
			return;
		}
		self.line_segments(points, stroke);
		if points.len() >= 3 {
			self.line(points[points.len() - 1], points[0], stroke);
		}
	}

	// =========================================================================
	// SVG-like path drawing methods
	// =========================================================================

	/// Move to a position (starts a new path segment)
	pub fn move_to(&self, pos: Pos2) -> Pos2 {
		pos
	}

	/// Draw a line from current position to target position
	pub fn line_to(&self, from: Pos2, to: Pos2, stroke: Stroke) {
		let sp1 = self.local_to_screen(from);
		let sp2 = self.local_to_screen(to);
		self.painter.line_segment([sp1, sp2], stroke);
	}

	/// Draw a quadratic Bézier curve
	pub fn quad_to(&self, from: Pos2, control: Pos2, to: Pos2, stroke: Stroke) {
		let points = self.subdivide_quadratic(from, control, to, 20);
		for window in points.windows(2) {
			let sp1 = self.local_to_screen(window[0]);
			let sp2 = self.local_to_screen(window[1]);
			self.painter.line_segment([sp1, sp2], stroke);
		}
	}

	/// Draw a cubic Bézier curve
	pub fn curve_to(&self, from: Pos2, control1: Pos2, control2: Pos2, to: Pos2, stroke: Stroke) {
		let points = self.subdivide_cubic(from, control1, control2, to, 30);
		for window in points.windows(2) {
			let sp1 = self.local_to_screen(window[0]);
			let sp2 = self.local_to_screen(window[1]);
			self.painter.line_segment([sp1, sp2], stroke);
		}
	}

	/// Draw a horizontal line
	pub fn horizontal_line_to(&self, from: Pos2, x: f32, stroke: Stroke) {
		let to = Pos2::new(x, from.y);
		self.line_to(from, to, stroke);
	}

	/// Draw a vertical line
	pub fn vertical_line_to(&self, from: Pos2, y: f32, stroke: Stroke) {
		let to = Pos2::new(from.x, y);
		self.line_to(from, to, stroke);
	}

	/// Draw an arc (approximated with cubic Bézier curves)
	pub fn arc_to(
		&self,
		center: Pos2,
		radius: f32,
		start_angle: f32,
		end_angle: f32,
		stroke: Stroke,
	) {
		let segments =
			((end_angle - start_angle).abs() / std::f32::consts::FRAC_PI_4).ceil() as usize;
		let segments = segments.max(1);
		let angle_step = (end_angle - start_angle) / segments as f32;

		for i in 0..segments {
			let a1 = start_angle + angle_step * i as f32;
			let a2 = start_angle + angle_step * (i + 1) as f32;

			let p1 = Pos2::new(center.x + radius * a1.cos(), center.y + radius * a1.sin());
			let p2 = Pos2::new(center.x + radius * a2.cos(), center.y + radius * a2.sin());

			// Approximate arc segment with line
			self.line_to(p1, p2, stroke);
		}
	}

	/// Close path by drawing line back to start
	pub fn close_path(&self, current: Pos2, start: Pos2, stroke: Stroke) {
		self.line_to(current, start, stroke);
	}

	/// Draw an ellipse
	pub fn ellipse(&self, center: Pos2, rx: f32, ry: f32, stroke: Stroke) {
		let segments = 32;
		let angle_step = std::f32::consts::TAU / segments as f32;

		for i in 0..segments {
			let a1 = angle_step * i as f32;
			let a2 = angle_step * (i + 1) as f32;

			let p1 = Pos2::new(center.x + rx * a1.cos(), center.y + ry * a1.sin());
			let p2 = Pos2::new(center.x + rx * a2.cos(), center.y + ry * a2.sin());

			self.line_to(p1, p2, stroke);
		}
	}

	/// Draw a filled ellipse
	pub fn ellipse_filled(&self, center: Pos2, rx: f32, ry: f32, color: Color32) {
		let screen_center = self.local_to_screen(center);
		let screen_rx = rx * self.scale;
		let screen_ry = ry * self.scale;

		// Approximate with a circle using average radius
		let avg_radius = (screen_rx + screen_ry) / 2.0;
		self.painter.circle_filled(screen_center, avg_radius, color);
	}

	/// Draw a polyline (open path through multiple points)
	pub fn polyline(&self, points: &[Pos2], stroke: Stroke) {
		for window in points.windows(2) {
			self.line_to(window[0], window[1], stroke);
		}
	}

	/// Draw a filled polygon
	pub fn polygon_filled(&self, points: &[Pos2], color: Color32) {
		if points.len() < 3 {
			return;
		}
		let screen_points: Vec<Pos2> = points.iter().map(|p| self.local_to_screen(*p)).collect();
		self.painter.add(egui::Shape::convex_polygon(
			screen_points,
			color,
			Stroke::NONE,
		));
	}

	// Helper: subdivide quadratic Bézier curve
	fn subdivide_quadratic(&self, p0: Pos2, p1: Pos2, p2: Pos2, segments: usize) -> Vec<Pos2> {
		let mut points = Vec::with_capacity(segments + 1);
		for i in 0..=segments {
			let t = i as f32 / segments as f32;
			let mt = 1.0 - t;
			let x = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
			let y = mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y;
			points.push(Pos2::new(x, y));
		}
		points
	}

	// Helper: subdivide cubic Bézier curve
	fn subdivide_cubic(
		&self,
		p0: Pos2,
		p1: Pos2,
		p2: Pos2,
		p3: Pos2,
		segments: usize,
	) -> Vec<Pos2> {
		let mut points = Vec::with_capacity(segments + 1);
		for i in 0..=segments {
			let t = i as f32 / segments as f32;
			let mt = 1.0 - t;
			let mt2 = mt * mt;
			let mt3 = mt2 * mt;
			let t2 = t * t;
			let t3 = t2 * t;
			let x = mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x;
			let y = mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y;
			points.push(Pos2::new(x, y));
		}
		points
	}
}

/// 汎用的なキャンバスウィジェット
/// パン・ズーム機能とグリッド描画を提供
#[allow(dead_code)]
pub struct CanvasWidget {
	pub scale: f32,
	pub offset: Vec2,
	pub background_color: Color32,
	pub grid_config: Option<GridConfig>,
	pub desired_size: Option<Vec2>,
}

impl Default for CanvasWidget {
	fn default() -> Self {
		Self {
			scale: 1.0,
			offset: Vec2::ZERO,
			background_color: Color32::from_gray(48),
			grid_config: Some(GridConfig::default()),
			desired_size: None,
		}
	}
}

#[allow(dead_code)]
impl CanvasWidget {
	/// 新しいキャンバスを作成
	pub fn new() -> Self {
		Self::default()
	}

	/// 背景色を設定
	pub fn with_background_color(mut self, color: Color32) -> Self {
		self.background_color = color;
		self
	}

	/// グリッド設定を設定（Noneでグリッドを無効化）
	pub fn with_grid(mut self, config: Option<GridConfig>) -> Self {
		self.grid_config = config;
		self
	}

	/// キャンバスのサイズを設定（Noneで利用可能なサイズ全体を使用）
	pub fn with_size(mut self, size: Option<Vec2>) -> Self {
		self.desired_size = size;
		self
	}

	/// キャンバスを描画し、描画コールバックを呼び出す
	/// サイズは `desired_size` が設定されていればそれを使用、なければ利用可能なサイズ全体
	pub fn show<F>(&mut self, ui: &mut Ui, draw_content: F) -> Response
	where
		F: FnOnce(&CanvasContext, &Response),
	{
		let size = self.desired_size.unwrap_or_else(|| ui.available_size());
		self.show_with_size(ui, size, draw_content)
	}

	/// 指定されたサイズでキャンバスを描画
	pub fn show_sized<F>(&mut self, ui: &mut Ui, size: Vec2, draw_content: F) -> Response
	where
		F: FnOnce(&CanvasContext, &Response),
	{
		self.show_with_size(ui, size, draw_content)
	}

	/// 内部実装：指定サイズでキャンバスを描画
	fn show_with_size<F>(&mut self, ui: &mut Ui, size: Vec2, draw_content: F) -> Response
	where
		F: FnOnce(&CanvasContext, &Response),
	{
		let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());

		let rect = response.rect;
		let origin = rect.left_top();

		// 背景描画
		painter.rect_filled(rect, 0.0, self.background_color);

		// グリッド描画
		if let Some(ref grid_config) = self.grid_config {
			self.draw_grid(&painter, rect, origin, grid_config);
		}

		// 描画コンテキストを作成
		let ctx = CanvasContext {
			painter: &painter,
			rect,
			origin,
			scale: self.scale,
			offset: self.offset,
		};

		// ユーザーのコンテンツを描画
		draw_content(&ctx, &response);

		// パン・ズーム処理
		self.handle_pan_zoom(ui, &response, origin);

		response
	}

	/// グリッドを描画
	fn draw_grid(&self, painter: &egui::Painter, rect: Rect, origin: Pos2, config: &GridConfig) {
		let (min_x, max_x, min_y, max_y) = self.visible_bounds(rect, origin);

		// 縦線
		let start_x = (min_x / config.spacing).floor() * config.spacing;
		let end_x = (max_x / config.spacing).ceil() * config.spacing;
		let mut x = start_x;
		while x <= end_x {
			let p1 = self.local_to_screen(Pos2::new(x, min_y), origin);
			let p2 = self.local_to_screen(Pos2::new(x, max_y), origin);
			painter.line_segment([p1, p2], config.stroke);
			x += config.spacing;
		}

		// 横線
		let start_y = (min_y / config.spacing).floor() * config.spacing;
		let end_y = (max_y / config.spacing).ceil() * config.spacing;
		let mut y = start_y;
		while y <= end_y {
			let p1 = self.local_to_screen(Pos2::new(min_x, y), origin);
			let p2 = self.local_to_screen(Pos2::new(max_x, y), origin);
			painter.line_segment([p1, p2], config.stroke);
			y += config.spacing;
		}

		// 原点軸
		if config.show_origin_axes {
			let axis_stroke = Stroke::new(1.0, config.origin_axis_color);

			// X軸 (y = 0)
			if 0.0 >= min_y && 0.0 <= max_y {
				let p1 = self.local_to_screen(Pos2::new(min_x, 0.0), origin);
				let p2 = self.local_to_screen(Pos2::new(max_x, 0.0), origin);
				painter.line_segment([p1, p2], axis_stroke);
			}

			// Y軸 (x = 0)
			if 0.0 >= min_x && 0.0 <= max_x {
				let p1 = self.local_to_screen(Pos2::new(0.0, min_y), origin);
				let p2 = self.local_to_screen(Pos2::new(0.0, max_y), origin);
				painter.line_segment([p1, p2], axis_stroke);
			}
		}
	}

	/// パン・ズーム処理
	fn handle_pan_zoom(&mut self, ui: &mut Ui, response: &Response, origin: Pos2) {
		if response.hovered() {
			// ズーム処理（ピンチまたはCtrl+スクロール）
			if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
				let pointer_local_before = self.screen_to_local(pointer, origin);

				let zoom = ui.input(|i| i.zoom_delta());
				if zoom != 1.0 {
					let old_scale = self.scale;
					self.scale *= zoom;

					// ポインタ位置を基準にズーム
					let local_before = pointer_local_before.to_vec2() * old_scale + self.offset;
					let local_after = pointer_local_before.to_vec2() * self.scale + self.offset;

					self.offset += local_before - local_after;
				}
			}

			// パン処理（スクロール）
			let scroll = ui.input(|i| i.smooth_scroll_delta);
			if scroll != Vec2::ZERO {
				self.offset += scroll * SCROLL_MULTIPLIER;
			}
		}
	}

	/// ローカル座標をスクリーン座標に変換
	fn local_to_screen(&self, local: Pos2, origin: Pos2) -> Pos2 {
		let screen_local = Pos2::new(
			local.x * self.scale + self.offset.x,
			local.y * self.scale + self.offset.y,
		);
		origin + screen_local.to_vec2()
	}

	/// スクリーン座標をローカル座標に変換
	fn screen_to_local(&self, screen: Pos2, origin: Pos2) -> Pos2 {
		let local = screen - origin;
		Pos2::new(
			(local.x - self.offset.x) / self.scale,
			(local.y - self.offset.y) / self.scale,
		)
	}

	/// 可視領域のローカル座標での範囲を取得
	fn visible_bounds(&self, rect: Rect, origin: Pos2) -> (f32, f32, f32, f32) {
		let top_left = self.screen_to_local(rect.left_top(), origin);
		let bottom_right = self.screen_to_local(rect.right_bottom(), origin);

		let min_x = top_left.x.min(bottom_right.x).floor();
		let max_x = top_left.x.max(bottom_right.x).ceil();
		let min_y = top_left.y.min(bottom_right.y).floor();
		let max_y = top_left.y.max(bottom_right.y).ceil();

		(min_x, max_x, min_y, max_y)
	}
}
