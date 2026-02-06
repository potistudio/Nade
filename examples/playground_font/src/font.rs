use egui::{Color32, Pos2, Response, Sense, Stroke, Ui, Vec2, pos2};

use crate::glyph_engine;

const GLYPH_SCALE: f32 = 1.2;
const CURVE_SEGMENTS: usize = 24;
const BACKGROUND_COLOR: Color32 = Color32::from_rgb(20, 20, 20);
const GLYPH_COLOR: Color32 = Color32::from_rgb(80, 230, 120);

pub struct FontPlaygroundWidget {
	glyph_engine: Option<glyph_engine::GlyphEngine>,
}

impl Default for FontPlaygroundWidget {
	fn default() -> Self {
		Self {
			glyph_engine: glyph_engine::GlyphEngine::new("./assets/fonts/Rubik/Rubik_regular.ttf"),
		}
	}
}

impl FontPlaygroundWidget {
	pub fn ui(&mut self, ui: &mut Ui) -> Response {
		let available = ui.available_size_before_wrap();
		let desired = Vec2::new(available.x.max(240.0), available.y.max(240.0));
		let (response, painter) = ui.allocate_painter(desired, Sense::hover());

		painter.rect_filled(response.rect, 0.0, BACKGROUND_COLOR);

		let Some(glyph_engine) = &self.glyph_engine else {
			painter.text(
				response.rect.center(),
				egui::Align2::CENTER_CENTER,
				"Font load failed",
				egui::FontId::proportional(16.0),
				Color32::LIGHT_RED,
			);
			return response;
		};

		let Some(path) = glyph_engine.get_path('a') else {
			painter.text(
				response.rect.center(),
				egui::Align2::CENTER_CENTER,
				"Glyph 'a' not found",
				egui::FontId::proportional(16.0),
				Color32::LIGHT_RED,
			);
			return response;
		};

		let origin = response.rect.center();
		let stroke = Stroke::new(1.5, GLYPH_COLOR);
		let to_screen = |point: Pos2| {
			pos2(
				origin.x + point.x * GLYPH_SCALE,
				origin.y + point.y * GLYPH_SCALE,
			)
		};

		let mut current_pos = Pos2::ZERO;
		let mut contour_start = None;

		for command in path {
			match command {
				swash::zeno::Command::MoveTo(v) => {
					current_pos = pos2(v.x, -v.y);
					contour_start = Some(current_pos);
				}
				swash::zeno::Command::LineTo(v) => {
					let next_pos = pos2(v.x, -v.y);
					painter.line_segment([to_screen(current_pos), to_screen(next_pos)], stroke);
					current_pos = next_pos;
				}
				swash::zeno::Command::QuadTo(v1, v2) => {
					let ctrl = pos2(v1.x, -v1.y);
					let end = pos2(v2.x, -v2.y);
					draw_quad(&painter, current_pos, ctrl, end, &to_screen, stroke);
					current_pos = end;
				}
				swash::zeno::Command::CurveTo(v1, v2, v3) => {
					let ctrl1 = pos2(v1.x, -v1.y);
					let ctrl2 = pos2(v2.x, -v2.y);
					let end = pos2(v3.x, -v3.y);
					draw_cubic(&painter, current_pos, ctrl1, ctrl2, end, &to_screen, stroke);
					current_pos = end;
				}
				swash::zeno::Command::Close => {
					if let Some(start_pos) = contour_start {
						painter
							.line_segment([to_screen(current_pos), to_screen(start_pos)], stroke);
						current_pos = start_pos;
					}
				}
			}
		}

		response
	}
}

fn draw_quad(
	painter: &egui::Painter,
	start: Pos2,
	ctrl: Pos2,
	end: Pos2,
	to_screen: &impl Fn(Pos2) -> Pos2,
	stroke: Stroke,
) {
	let mut previous = start;
	for i in 1..=CURVE_SEGMENTS {
		let t = i as f32 / CURVE_SEGMENTS as f32;
		let next = sample_quad(start, ctrl, end, t);
		painter.line_segment([to_screen(previous), to_screen(next)], stroke);
		previous = next;
	}
}

fn draw_cubic(
	painter: &egui::Painter,
	start: Pos2,
	ctrl1: Pos2,
	ctrl2: Pos2,
	end: Pos2,
	to_screen: &impl Fn(Pos2) -> Pos2,
	stroke: Stroke,
) {
	let mut previous = start;
	for i in 1..=CURVE_SEGMENTS {
		let t = i as f32 / CURVE_SEGMENTS as f32;
		let next = sample_cubic(start, ctrl1, ctrl2, end, t);
		painter.line_segment([to_screen(previous), to_screen(next)], stroke);
		previous = next;
	}
}

fn sample_quad(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
	let u = 1.0 - t;
	pos2(
		u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x,
		u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y,
	)
}

fn sample_cubic(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
	let u = 1.0 - t;
	let uu = u * u;
	let tt = t * t;
	pos2(
		uu * u * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + tt * t * p3.x,
		uu * u * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + tt * t * p3.y,
	)
}
