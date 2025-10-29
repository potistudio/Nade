use image::{ImageBuffer, Rgb};
use std::io::Write;
use std::process::{Command, Stdio};

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

fn main() {
	let width = 1280;
	let height = 720;

	let mut ffmpeg = Command::new("ffmpeg")
		.args(&[
			"-f",
			"rawvideo",
			"-pixel_format",
			"rgb24",
			"-video_size",
			&format!("{}x{}", width, height),
			"-framerate",
			"30",
			"-i",
			"-",
			"-c:v",
			"libx264",
			"-preset",
			"fast",
			"-crf",
			"23",
			"render.mp4",
		])
		.stdin(Stdio::piped())
		.spawn()
		.unwrap();

	let mut stdin = ffmpeg.stdin.take().unwrap();

	for i in 0..300 {
		let frame = render_frame(i, width, height);
		stdin.write_all(&frame.into_raw()).unwrap();
	}

	drop(stdin);
	ffmpeg.wait().unwrap();
}
