mod font;
mod glyph_engine;

#[derive(Default)]
struct FontPlaygroundApp {
	playground_widget: font::FontPlaygroundWidget,
}

impl eframe::App for FontPlaygroundApp {
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
				self.playground_widget.ui(ui);
			});
	}
}

fn main() -> eframe::Result<()> {
	unsafe {
		std::env::set_var("RUST_LOG", "playground_font=debug");
	}
	env_logger::init();

	let options = eframe::NativeOptions {
		..Default::default()
	};

	log::debug!("Print localized strings from the font:");
	glyph_engine::print_localized_strings("./assets/fonts/Rubik/Rubik_regular.ttf");

	eframe::run_native(
		"Font Feature Playground",
		options,
		Box::new(|_cc| Ok(Box::new(FontPlaygroundApp::default()))),
	)
}
