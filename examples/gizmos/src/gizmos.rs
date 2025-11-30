use egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

const SCROLL_MULTIPLIER: f32 = 1.0;

const GRID_SPACING: f32 = 20.0;
const GRID_STROKE: Stroke = Stroke {
	width: 1.0,
	color: Color32::from_gray(26),
};

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
	pos: Pos2, // gizmo座標系
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
	// ギズモ内でのスケールやオフセット（パン）とか
	pub scale: f32,
	pub offset: Vec2,
	pub handles: Vec<BezierHandle>,
	active_handle: Option<usize>,
	drag_offset: Vec2,
	dragging_part: Option<HandlePart>,
}

impl Default for GizmoWidget {
	fn default() -> Self {
		Self {
			scale: 1.0,
			offset: Vec2::ZERO,
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
		// サイズは適当に。ここは好みでレイアウトに合わせて変えてOK
		let desired_size = ui.available_size();
		let (response, painter) = ui.allocate_painter(desired_size, Sense::click_and_drag());

		let rect = response.rect;

		// ローカル座標系の原点を左上に置く例
		// screen_pos = rect.left_top() + local_pos
		// local_pos = screen_pos - rect.left_top()

		// 背景
		painter.rect_filled(rect, 0.0, Color32::from_gray(48));

		// ここで好きなギズモ（線やハンドル）を描く
		self.draw_gizmo(&painter, rect);

		// ここでインタラクション処理
		self.handle_interaction(ui, &response, rect);

		response
	}

	fn draw_gizmo(&self, painter: &egui::Painter, rect: Rect) {
		let origin = rect.left_top();

		// ==== [BEGIN] Draw background grid ===================================
		let (min_x, max_x, min_y, max_y) = self.visible_gizmo_bounds(rect, origin);

		// 縦線
		let start_x = (min_x / GRID_SPACING).floor() * GRID_SPACING;
		let end_x = (max_x / GRID_SPACING).ceil() * GRID_SPACING;
		let mut x = start_x;
		while x <= end_x {
			let p1 = self.gizmo_to_screen(Pos2::new(x, min_y), origin);
			let p2 = self.gizmo_to_screen(Pos2::new(x, max_y), origin);
			painter.line_segment([p1, p2], GRID_STROKE);
			x += GRID_SPACING;
		}

		// 横線
		let start_y = (min_y / GRID_SPACING).floor() * GRID_SPACING;
		let end_y = (max_y / GRID_SPACING).ceil() * GRID_SPACING;
		let mut y = start_y;
		while y <= end_y {
			let p1 = self.gizmo_to_screen(Pos2::new(min_x, y), origin);
			let p2 = self.gizmo_to_screen(Pos2::new(max_x, y), origin);
			painter.line_segment([p1, p2], GRID_STROKE);
			y += GRID_SPACING;
		}

		// === 原点強調ライン ===
		let axis_color = Color32::from_rgb(120, 200, 255); // 少し明るめの青
		let axis_stroke = Stroke::new(1.0, axis_color);

		// X軸 (y = 0)
		if 0.0 >= min_y && 0.0 <= max_y {
			let p1 = self.gizmo_to_screen(Pos2::new(min_x, 0.0), origin);
			let p2 = self.gizmo_to_screen(Pos2::new(max_x, 0.0), origin);
			painter.line_segment([p1, p2], axis_stroke);
		}

		// Y軸 (x = 0)
		if 0.0 >= min_x && 0.0 <= max_x {
			let p1 = self.gizmo_to_screen(Pos2::new(0.0, min_y), origin);
			let p2 = self.gizmo_to_screen(Pos2::new(0.0, max_y), origin);
			painter.line_segment([p1, p2], axis_stroke);
		}
		// ==== [END] Draw background grid =====================================

		// ハンドル間を線で結ぶ
		// for w in self.handles.windows(2) {
		// 	let p1 = self.gizmo_to_screen(w[0].pos, origin);
		// 	let p2 = self.gizmo_to_screen(w[1].pos, origin);
		// 	painter.line_segment([p1, p2], Stroke::new(2.0, Color32::from_rgb(235, 72, 83)));
		// }
		for h in &self.handles {
			let p_handle = self.gizmo_to_screen(h.pos, origin);
			let p_left = self.gizmo_to_screen(h.get_left_point(), origin);
			let p_right = self.gizmo_to_screen(h.get_right_point(), origin);

			// ハンドルから制御点への線
			painter.line_segment(
				[p_handle, p_left],
				Stroke::new(1.0, Color32::from_gray(150)),
			);
			painter.line_segment(
				[p_handle, p_right],
				Stroke::new(1.0, Color32::from_gray(150)),
			);

			if self.handles.len() >= 2 {
				for i in 0..=self.handles.len() - 2 {
					let p0 = self.handles[i].get_position();
					let p1 = self.handles[i].get_right_point();
					let p2 = self.handles[i + 1].get_left_point();
					let p3 = self.handles[i + 1].get_position();

					let segments = 64;
					for j in 0..segments {
						let t0 = j as f32 / segments as f32;
						let t1 = (j + 1) as f32 / segments as f32;
						let bp0 = cubic_bezier(p0, p1, p2, p3, t0);
						let bp1 = cubic_bezier(p0, p1, p2, p3, t1);
						let sp0 = self.gizmo_to_screen(bp0, origin);
						let sp1 = self.gizmo_to_screen(bp1, origin);
						painter.line_segment(
							[sp0, sp1],
							Stroke::new(2.0, Color32::from_rgb(235, 72, 83)),
						);
					}
				}
			}

			painter.circle_filled(p_handle, 3.0, Color32::from_gray(230));
			painter.circle_stroke(p_left, 2.0, Stroke::new(2.0, Color32::from_gray(230)));
			painter.circle_stroke(p_right, 2.0, Stroke::new(2.0, Color32::from_gray(230)));
		}
	}

	fn handle_interaction(&mut self, ui: &mut Ui, response: &Response, rect: Rect) {
		let origin = rect.left_top();

		// マウス押下タイミングでどのハンドルを選ぶか
		if response.drag_started()
			&& let Some(pos) = response.interact_pointer_pos()
		{
			let pointer_pos = self.screen_to_gizmo(pos, origin);

			let threshold_radius = 16.0 / self.scale; // 見た目のヒット半径をscaleに合わせる

			// 何もヒットしなかった場合は early return してドラッグ処理自体をスキップ
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

		// ドラッグ中にハンドルを動かす
		if response.dragged()
			&& let (Some(pos), Some(active_id)) =
				(response.interact_pointer_pos(), self.active_handle)
		{
			let pointer_pos = self.screen_to_gizmo(pos, origin);

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

		// ドラッグ終了でリセット
		if response.drag_stopped() {
			self.active_handle = None;
		}

		if response.hovered() {
			// Zooming with pinch or Ctrl + scroll
			if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
				let pointer_gizmo_before = self.screen_to_gizmo(pointer, origin);

				let zoom = ui.input(|i| i.zoom_delta());
				if zoom != 1.0 {
					let old_scale = self.scale;
					self.scale *= zoom;

					// Adjust offset so that the point under the cursor stays in
					// the same place
					let gizmo_before = pointer_gizmo_before.to_vec2() * old_scale + self.offset;
					let gizmo_after = pointer_gizmo_before.to_vec2() * self.scale + self.offset;

					self.offset += gizmo_before - gizmo_after;
				}
			}

			// Panning with scroll
			let scroll = ui.input(|i| i.smooth_scroll_delta);
			if scroll != Vec2::ZERO {
				self.offset += scroll * SCROLL_MULTIPLIER;
			}
		}
	}

	/// Convert gizmo local transformation to screen position
	fn gizmo_to_screen(&self, gizmo: Pos2, origin: Pos2) -> Pos2 {
		let local = Pos2::new(
			gizmo.x * self.scale + self.offset.x,
			gizmo.y * self.scale + self.offset.y,
		);
		origin + local.to_vec2()
	}

	/// Convert screen position to gizmo local transformation
	fn screen_to_gizmo(&self, screen: Pos2, origin: Pos2) -> Pos2 {
		let local = screen - origin;
		Pos2::new(
			(local.x - self.offset.x) / self.scale,
			(local.y - self.offset.y) / self.scale,
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

	fn visible_gizmo_bounds(&self, rect: Rect, origin: Pos2) -> (f32, f32, f32, f32) {
		let top_left = self.screen_to_gizmo(rect.left_top(), origin);
		let bottom_right = self.screen_to_gizmo(rect.right_bottom(), origin);

		let min_x = top_left.x.min(bottom_right.x).floor();
		let max_x = top_left.x.max(bottom_right.x).ceil();
		let min_y = top_left.y.min(bottom_right.y).floor();
		let max_y = top_left.y.max(bottom_right.y).ceil();

		(min_x, max_x, min_y, max_y)
	}
}
