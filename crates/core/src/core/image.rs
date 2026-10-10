use super::error::EvalError;

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
	pub width: u32,
	pub height: u32,
	pub rgba: Vec<f32>,
}

impl Image {
	pub fn new(width: u32, height: u32, rgba: Vec<f32>) -> Result<Self, EvalError> {
		let expected = width as usize * height as usize * 4;
		if rgba.len() != expected {
			return Err(EvalError::InvalidImageBuffer {
				expected,
				actual: rgba.len(),
			});
		}

		Ok(Self { width, height, rgba })
	}

	pub fn solid_color(color: [f32; 4], width: u32, height: u32) -> Self {
		let size = width as usize * height as usize;
		let mut rgba = Vec::with_capacity(size * 4);
		for _ in 0..size {
			rgba.push(color[0]);
			rgba.push(color[1]);
			rgba.push(color[2]);
			rgba.push(color[3]);
		}

		Self { width, height, rgba }
	}
}
