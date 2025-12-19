//! # Renderer Module
//!
//! エフェクトベースのレンダリングシステムを提供します。

pub mod effects;

use crate::encoder::Encoder;
use anyhow::Result;
use effects::WaveEffect;
use image::{ImageBuffer, Rgba};
use nade_core::{Effect, FrameBuffer, RenderContext, RgbColor};
use rayon::prelude::*;

/// 指定されたフレーム番号に対応する画像を生成
///
/// rayon を使用して行単位で並列処理を行います。
///
/// # 引数
///
/// * `frame_num` - フレーム番号（アニメーション時間を決定）
/// * `width` - 出力画像の幅（ピクセル）
/// * `height` - 出力画像の高さ（ピクセル）
///
/// # 戻り値
///
/// RGBA フォーマットの `ImageBuffer`
pub fn render_frame(frame_num: u32, width: u32, height: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
	// デフォルトのエフェクトを使用
	let effect = WaveEffect::default();
	render_frame_with_effect(&effect, frame_num, width, height)
}

/// エフェクトを指定してフレームを生成
///
/// 任意の `Effect` 実装を使用してレンダリングを行います。
///
/// # 引数
///
/// * `effect` - 適用するエフェクト
/// * `frame_num` - フレーム番号
/// * `width` - 出力幅
/// * `height` - 出力高さ
///
/// # 戻り値
///
/// RGBA フォーマットの `ImageBuffer`
pub fn render_frame_with_effect(
	effect: &dyn Effect,
	frame_num: u32,
	width: u32,
	height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
	// レンダリングコンテキストを作成
	let ctx = RenderContext {
		width,
		height,
		time: frame_num as f32 / 60.0,
		frame: frame_num,
	};

	// 並列処理で各行のピクセルを計算
	let pixels: Vec<u8> = (0..height)
		.into_par_iter()
		.flat_map(|y| {
			(0..width)
				.flat_map(|x| {
					let color = effect.apply(RgbColor::BLACK, x, y, &ctx);
					[color.r, color.g, color.b, 255]
				})
				.collect::<Vec<u8>>()
		})
		.collect();

	ImageBuffer::from_raw(width, height, pixels).expect("Buffer size mismatch")
}

/// レンダラー構造体
///
/// エンコーダーと連携してフレームをレンダリング・エンコードします。
pub struct Renderer<E: Encoder> {
	/// エンコーダー
	pub encoder: E,
}

impl<E: Encoder> Renderer<E> {
	/// 全フレームをレンダリング
	///
	/// 600フレーム（10秒 @ 60fps）をレンダリングしてエンコードします。
	pub fn render(&mut self) -> Result<()> {
		self.encoder.prepare()?;

		let width = 1920;
		let height = 1080;
		let total_frames = 600;

		// デフォルトエフェクトを使用
		let effect = WaveEffect::default();

		for frame_num in 0..total_frames {
			let frame_image = render_frame_with_effect(&effect, frame_num, width, height);

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
