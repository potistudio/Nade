pub struct PreviewPanel {
	scale: f32,
}

impl Default for PreviewPanel {
	fn default() -> Self {
		Self { scale: 1.0 }
	}
}

impl PreviewPanel {
	pub fn set_scale(&mut self, scale: f32) {
		self.scale = scale;
	}

	pub fn get_scale(&self) -> f32 {
		self.scale
	}
}
