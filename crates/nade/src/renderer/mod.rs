use crate::encoder::Encoder;
use anyhow::Result;
use core::FrameBuffer;
use image::{ImageBuffer, Rgb};

/// 指定されたフレーム番号に対応する画像を生成
///
/// # 引数
///
/// * `frame_num` - フレーム番号（アニメーション時間を決定）
/// * `width` - 出力画像の幅（ピクセル）
/// * `height` - 出力画像の高さ（ピクセル）
///
/// # 戻り値
///
/// RGB フォーマットの `ImageBuffer`
pub fn render_frame(frame_num: u32, width: u32, height: u32) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
	let mut img = ImageBuffer::new(width, height);

	// フレーム番号を時間パラメータに変換
	let t = frame_num as f32 / 100.0;

	// 各ピクセルのカラーを計算
	for (x, y, pixel) in img.enumerate_pixels_mut() {
		// x座標に基づく赤チャンネル（sin波でアニメーション）
		let r = ((x as f32 / width as f32 * 255.0) * t.sin().abs()) as u8;
		// y座標に基づく緑チャンネル（cos波でアニメーション）
		let g = ((y as f32 / height as f32 * 255.0) * t.cos().abs()) as u8;
		// 固定の青チャンネル
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
