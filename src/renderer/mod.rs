use crate::core::FrameBuffer;
use crate::encoder::Encoder;
use anyhow::Result;
use image::{ImageBuffer, Rgb};

fn render_frame(frame_num: u32, width: u32, height: u32) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
	let mut img = ImageBuffer::new(width, height);

	let t = frame_num as f32 / 100.0;

	for (x, y, pixel) in img.enumerate_pixels_mut() {
		let r = ((x as f32 / width as f32 * 255.0) * t.sin().abs()) as u8;
		let g = ((y as f32 / height as f32 * 255.0) * t.cos().abs()) as u8;
		let b = 128;
		*pixel = Rgb([r, g, b]);
	}

	img
}

pub struct Renderer<E: Encoder> {
	pub encoder: E,
}

impl<E: Encoder> Renderer<E> {
	pub fn render(&mut self) -> Result<()> {
		self.encoder.prepare()?;

		let width = 1920;
		let height = 1080;
		let total_frames = 600;

		for frame_num in 0..total_frames {
			let frame_image = render_frame(frame_num, width, height); // composition.render_frame();

			let frame_buffer = FrameBuffer {
				width,
				height,
				data: frame_image.into_raw(),
			};

			self.encoder.encode_frame(&frame_buffer, frame_num)?;
		}

		self.encoder.finish()?;
		Ok(())
	}
}
