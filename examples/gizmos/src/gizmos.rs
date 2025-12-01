use egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

// =============================================================================
// CanvasWidget - 汎用的なパン・ズーム対応キャンバスウィジェット
// =============================================================================

const SCROLL_MULTIPLIER: f32 = 1.0;

/// グリッド描画設定
#[derive(Clone, Debug)]
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
}

/// 汎用的なキャンバスウィジェット
/// パン・ズーム機能とグリッド描画を提供
#[allow(dead_code)]
pub struct CanvasWidget {
	pub scale: f32,
	pub offset: Vec2,
	pub background_color: Color32,
	pub grid_config: Option<GridConfig>,
}

impl Default for CanvasWidget {
	fn default() -> Self {
		Self {
			scale: 1.0,
			offset: Vec2::ZERO,
			background_color: Color32::from_gray(48),
			grid_config: Some(GridConfig::default()),
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

	/// キャンバスを描画し、描画コールバックを呼び出す
	pub fn show<F>(&mut self, ui: &mut Ui, draw_content: F) -> Response
	where
		F: FnOnce(&CanvasContext, &Response),
	{
		let desired_size = ui.available_size();
		let (response, painter) = ui.allocate_painter(desired_size, Sense::click_and_drag());

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

// =============================================================================
// GizmoWidget - ベジェハンドル編集用の特化ウィジェット
// =============================================================================

enum HandlePart {
	Center,
	LeftControl,
	RightControl,
}

fn cubic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
	let it = 1.0 - t;
	let x = it.powi(3) * p0.x
		+ 3.0 * it.powi(2) * t * p1.x
		+ 3.0 * it * t.powi(2) * p2.x
		+ t.powi(3) * p3.x;
	let y = it.powi(3) * p0.y
		+ 3.0 * it.powi(2) * t * p1.y
		+ 3.0 * it * t.powi(2) * p2.y
		+ t.powi(3) * p3.y;
	Pos2::new(x, y)
}

#[derive(Clone, Copy, Debug)]
pub struct BezierHandle {
	pub id: usize,
	pos: Pos2,
	left_point: Pos2,
	right_point: Pos2,
}

impl BezierHandle {
	pub fn new(id: usize, pos: Pos2) -> Self {
		Self {
			id,
			pos,
			left_point: pos + Vec2::new(-40.0, 0.0),
			right_point: pos + Vec2::new(40.0, 0.0),
		}
	}

	pub fn set_position(&mut self, pos: Pos2) {
		let delta = pos - self.pos;
		self.pos = pos;
		self.left_point += delta;
		self.right_point += delta;
	}

	pub fn get_position(&self) -> Pos2 {
		self.pos
	}

	pub fn set_left_point(&mut self, point: Pos2) {
		self.left_point = point;
		self.right_point = (self.left_point - self.pos.to_vec2()) * -1.0 + self.pos.to_vec2();
	}

	pub fn set_right_point(&mut self, point: Pos2) {
		self.right_point = point;
		self.left_point = (self.right_point - self.pos.to_vec2()) * -1.0 + self.pos.to_vec2();
	}

	pub fn get_left_point(&self) -> Pos2 {
		self.left_point
	}

	pub fn get_right_point(&self) -> Pos2 {
		self.right_point
	}
}

pub struct GizmoWidget {
	canvas: CanvasWidget,
	pub handles: Vec<BezierHandle>,
	active_handle: Option<usize>,
	drag_offset: Vec2,
	dragging_part: Option<HandlePart>,
}

impl Default for GizmoWidget {
	fn default() -> Self {
		Self {
			canvas: CanvasWidget::default(),
			handles: vec![
				BezierHandle::new(0, Pos2::new(20.0, 80.0)),
				BezierHandle::new(1, Pos2::new(120.0, 20.0)),
			],
			active_handle: None,
			drag_offset: Vec2::ZERO,
			dragging_part: None,
		}
	}
}

impl GizmoWidget {
	pub fn ui(&mut self, ui: &mut Ui) -> Response {
		// コンテンツ描画に必要な状態をキャプチャ
		let handles = self.handles.clone();
		let active_handle = self.active_handle;

		let response = self.canvas.show(ui, |ctx, _response| {
			Self::draw_bezier_content(ctx, &handles, active_handle);
		});

		// インタラクション処理
		self.handle_bezier_interaction(&response);

		response
	}

	fn draw_bezier_content(
		ctx: &CanvasContext,
		handles: &[BezierHandle],
		active_handle: Option<usize>,
	) {
		// ハンドルの制御点ラインを描画
		for h in handles {
			let p_handle = ctx.local_to_screen(h.pos);
			let p_left = ctx.local_to_screen(h.get_left_point());
			let p_right = ctx.local_to_screen(h.get_right_point());

			ctx.painter.line_segment(
				[p_handle, p_left],
				Stroke::new(1.0, Color32::from_gray(150)),
			);
			ctx.painter.line_segment(
				[p_handle, p_right],
				Stroke::new(1.0, Color32::from_gray(150)),
			);
		}

		// ベジェ曲線を描画
		if handles.len() >= 2 {
			for i in 0..handles.len() - 1 {
				let p0 = handles[i].get_position();
				let p1 = handles[i].get_right_point();
				let p2 = handles[i + 1].get_left_point();
				let p3 = handles[i + 1].get_position();

				let segments = 64;
				for j in 0..segments {
					let t0 = j as f32 / segments as f32;
					let t1 = (j + 1) as f32 / segments as f32;
					let bp0 = cubic_bezier(p0, p1, p2, p3, t0);
					let bp1 = cubic_bezier(p0, p1, p2, p3, t1);
					let sp0 = ctx.local_to_screen(bp0);
					let sp1 = ctx.local_to_screen(bp1);
					ctx.painter
						.line_segment([sp0, sp1], Stroke::new(2.0, Color32::from_rgb(235, 72, 83)));
				}
			}
		}

		// ハンドル点を描画
		for h in handles {
			let p_handle = ctx.local_to_screen(h.pos);
			let p_left = ctx.local_to_screen(h.get_left_point());
			let p_right = ctx.local_to_screen(h.get_right_point());

			ctx.painter
				.circle_filled(p_handle, 3.0, Color32::from_gray(230));
			ctx.painter
				.circle_stroke(p_left, 2.0, Stroke::new(2.0, Color32::from_gray(230)));
			ctx.painter
				.circle_stroke(p_right, 2.0, Stroke::new(2.0, Color32::from_gray(230)));
		}

		// アクティブハンドルの座標表示
		if let Some(h) = active_handle.and_then(|id| handles.iter().find(|h| h.id == id)) {
			let text_pos = h.get_position() + Vec2::new(0.0, -8.0);
			let text = format!("({:.1}, {:.1})", h.get_position().x, h.get_position().y);
			ctx.painter.text(
				ctx.local_to_screen_unscaled(text_pos),
				egui::Align2::LEFT_BOTTOM,
				text,
				egui::FontId::monospace(10.0),
				Color32::WHITE,
			);
		}
	}

	fn handle_bezier_interaction(&mut self, response: &Response) {
		let origin = response.rect.left_top();

		// ドラッグ開始時のハンドル選択
		if response.drag_started()
			&& let Some(pos) = response.interact_pointer_pos()
		{
			let pointer_pos = self.screen_to_local(pos, origin);
			let threshold_radius = 16.0 / self.canvas.scale;

			let Some(best) = self.find_nearest_handle(pointer_pos, threshold_radius) else {
				self.active_handle = None;
				return;
			};

			self.active_handle = Some(best.0);
			if let Some(h) = self.handles.iter().find(|h| h.id == best.0) {
				let handle_pos = match best.1 {
					HandlePart::Center => h.pos,
					HandlePart::LeftControl => h.get_left_point(),
					HandlePart::RightControl => h.get_right_point(),
				};
				self.drag_offset = handle_pos - pointer_pos;
				self.dragging_part = Some(best.1);
			}
		}

		// ドラッグ中のハンドル移動
		if response.dragged()
			&& let (Some(pos), Some(active_id)) =
				(response.interact_pointer_pos(), self.active_handle)
		{
			let pointer_pos = self.screen_to_local(pos, origin);

			if let Some(h) = self.handles.iter_mut().find(|h| h.id == active_id) {
				match self.dragging_part {
					Some(HandlePart::Center) => {
						h.set_position(pointer_pos + self.drag_offset);
					}
					Some(HandlePart::LeftControl) => {
						h.set_left_point(pointer_pos + self.drag_offset);
					}
					Some(HandlePart::RightControl) => {
						h.set_right_point(pointer_pos + self.drag_offset);
					}
					None => {}
				}
			}
		}

		// ドラッグ終了
		if response.drag_stopped() {
			self.active_handle = None;
			self.dragging_part = None;
		}
	}

	fn screen_to_local(&self, screen: Pos2, origin: Pos2) -> Pos2 {
		let local = screen - origin;
		Pos2::new(
			(local.x - self.canvas.offset.x) / self.canvas.scale,
			(local.y - self.canvas.offset.y) / self.canvas.scale,
		)
	}

	fn find_nearest_handle(&self, from: Pos2, radius: f32) -> Option<(usize, HandlePart)> {
		let mut best: Option<(usize, f32)> = None;
		let r2 = radius * radius;
		let mut part = HandlePart::Center;

		for h in &self.handles {
			// center
			let d2_center = (h.pos - from).length_sq();
			if d2_center < r2 && best.is_none_or(|(_, bd2)| d2_center < bd2) {
				best = Some((h.id, d2_center));
				part = HandlePart::Center;
			}

			// left handle
			let d2_left = (h.get_left_point() - from).length_sq();
			if d2_left < r2 && best.is_none_or(|(_, bd2)| d2_left < bd2) {
				best = Some((h.id, d2_left));
				part = HandlePart::LeftControl;
			}

			// right handle
			let d2_right = (h.get_right_point() - from).length_sq();
			if d2_right < r2 && best.is_none_or(|(_, bd2)| d2_right < bd2) {
				best = Some((h.id, d2_right));
				part = HandlePart::RightControl;
			}
		}

		best.map(|(id, _)| (id, part))
	}
}
