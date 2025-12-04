mod timeline;

use egui;

#[derive(Default)]
struct TimelineApp {
	timeline_widget: timeline::TimelineWidget,
}

impl eframe::App for TimelineApp {
	fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
		// カスタムフォント設定
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

		eframe::egui::CentralPanel::default()
			.frame(egui::Frame::default().inner_margin(0.0))
			.show(ctx, |ui| {
				self.timeline_widget.ui(ui);
			});
	}
}

fn main() -> eframe::Result<()> {
	unsafe {
		std::env::set_var("RUST_LOG", "debug");
	}
	env_logger::init();

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([1200.0, 400.0])
			.with_title("Timeline Demo"),
		..Default::default()
	};

	log::debug!("Starting Timeline Demo");

	eframe::run_native(
		"Demo - Timeline",
		options,
		Box::new(|_cc| Ok(Box::new(TimelineApp::default()))),
	)
}
