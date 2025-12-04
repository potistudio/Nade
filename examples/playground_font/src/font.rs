use canvas_panel::canvas::CanvasWidget;
use egui::{Color32, Pos2, Rect, Response, Stroke, Ui, Vec2, pos2};

use crate::glyph_engine;

pub struct FontPlaygroundWidget {
	canvas: CanvasWidget,
	glyph_engine: glyph_engine::GlyphEngine,
}

impl Default for FontPlaygroundWidget {
	fn default() -> Self {
		Self {
			canvas: Default::default(),
			glyph_engine: glyph_engine::GlyphEngine::new("./assets/fonts/Rubik/Rubik_regular.ttf")
				.unwrap(),
		}
	}
}

impl FontPlaygroundWidget {
	pub fn ui(&mut self, ui: &mut Ui) -> Response {
		self.canvas.show(ui, |ctx, _responce| {
			let path = self.glyph_engine.get_path('a').expect("No path");
			let mut current_pos = egui::Pos2::new(0.0, 0.0);
			let stroke = Stroke::new(1.0, Color32::GREEN);

			path.iter().for_each(|cmd| match cmd {
				swash::zeno::Command::MoveTo(v) => {
					current_pos = egui::pos2(v.x, -v.y);
				}
				swash::zeno::Command::LineTo(v) => {
					ctx.line_to(current_pos, Pos2::new(v.x, -v.y), stroke);
					current_pos = egui::pos2(v.x, -v.y);
				}
				swash::zeno::Command::QuadTo(v1, v2) => {
					ctx.quad_to(current_pos, pos2(v1.x, -v1.y), pos2(v2.x, -v2.y), stroke);
					current_pos = egui::pos2(v2.x, -v2.y);
				}
				swash::zeno::Command::CurveTo(v1, v2, v3) => {
					ctx.curve_to(
						current_pos,
						pos2(v1.x, -v1.y),
						pos2(v2.x, -v2.y),
						pos2(v3.x, -v3.y),
						stroke,
					);
					current_pos = egui::pos2(v3.x, -v3.y);
				}
				swash::zeno::Command::Close => {
					println!("ClosePath");
				}
			});
		})
	}
}
