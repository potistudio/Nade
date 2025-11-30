mod gizmos;

#[derive(Default)]
struct GizmosDemoApp {
	gizmo_widget: gizmos::GizmoWidget,
}

impl eframe::App for GizmosDemoApp {
	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		// ==== [BEGIN] Custom font ============================================
		let mut fonts = egui::FontDefinitions::default();
		let font_data = egui::FontData::from_static(include_bytes!(
			"../../../assets/fonts/Rubik/Rubik_regular.ttf"
		));

		fonts
			.font_data
			.insert("rubik".to_string(), font_data.into());

		fonts
			.families
			.entry(egui::FontFamily::Proportional)
			.or_default()
			.insert(0, "rubik".to_string());

		ctx.set_fonts(fonts);
		// ==== [END] Custom font ==============================================

		eframe::egui::CentralPanel::default()
			.frame(egui::Frame::default().inner_margin(0))
			.show(ctx, |ui| {
				self.gizmo_widget.ui(ui);
				// let delta_time = ctx.input(|i| i.unstable_dt);
				// let fps = 1.0 / delta_time.max(0.00001);

				// let painter = ctx.layer_painter(egui::LayerId::new(
				// 	egui::Order::Foreground,
				// 	egui::Id::new("fps_overlay"),
				// ));

				// let text = format!("{fps:.1} fps");
				// let margin = 8.0;
				// let galley = painter.layout_no_wrap(
				// 	text,
				// 	egui::FontId::proportional(14.0),
				// 	egui::Color32::WHITE,
				// );
				// let pos = egui::pos2(
				// 	ui.max_rect().right() - galley.size().x - margin,
				// 	ui.max_rect().top() + margin,
				// );

				// painter.rect_filled(
				// 	egui::Rect::from_two_pos(
				// 		pos - egui::vec2(4.0, 2.0),
				// 		pos + galley.size() + egui::vec2(4.0, 2.0),
				// 	),
				// 	4.0,
				// 	egui::Color32::from_black_alpha(180),
				// );

				// painter.galley(pos, galley, egui::Color32::WHITE);
			});
	}
}

fn main() -> eframe::Result<()> {
	unsafe {
		std::env::set_var("RUST_LOG", "debug");
	}
	env_logger::init();

	let options = eframe::NativeOptions {
		..Default::default()
	};

	eframe::run_native(
		"Demo - Gizmos",
		options,
		Box::new(|_cc| Ok(Box::new(GizmosDemoApp::default()))),
	)
}
