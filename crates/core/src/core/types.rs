use super::image::Image;
use super::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
	Value,
	Image,
}

#[derive(Debug, Clone)]
pub enum Output {
	Value(Value),
	Image(Image),
}

impl Output {
	pub fn output_type(&self) -> OutputType {
		match self {
			Self::Value(_) => OutputType::Value,
			Self::Image(_) => OutputType::Image,
		}
	}
}
