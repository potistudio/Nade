#[derive(Debug, Clone, Copy)]
pub struct EvalContext {
	pub fps: f64,
}

impl Default for EvalContext {
	fn default() -> Self {
		Self { fps: 60.0 }
	}
}
