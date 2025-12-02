use canvas_panel::canvas::*;
use egui::{self, Color32, Pos2, Response, Stroke, Ui, Vec2};

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
			ctx.line_segments(
				&[h.pos, h.get_left_point()],
				Stroke::new(1.0, Color32::from_gray(150)),
			);

			ctx.line_segments(
				&[h.pos, h.get_right_point()],
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
