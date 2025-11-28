#[derive(Default)]
pub struct StatusBar {}

impl StatusBar {
	pub fn ui(&mut self, ui: &mut egui::Ui) {
		ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
			ui.label("v0.0.1a");
		});
	}
}
