//! # Renderer Module
//!
//! エフェクトベースのレンダリングシステムを提供します。

use anyhow::Result;
use core::{FrameBuffer, RectangleObject, RenderContext, RgbColor, SceneObjectData, TextObject};
use domain::Composition;
use effects::Effect;
use effects::WaveEffect;
use encoder::Encoder;
use image::{ImageBuffer, Rgba};
use rayon::prelude::*;
use swash::{
	FontRef, GlyphId,
	scale::{Render, ScaleContext, Source},
	zeno::Format,
};

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

// =============================================================================
// シーンオブジェクトレンダリング
// =============================================================================

/// コンポジションからフレームを生成
///
/// 指定時間で可視なシーンオブジェクトをすべて描画します。
///
/// # 引数
///
/// * `composition` - 描画するコンポジション
/// * `time` - 現在時間（秒）
/// * `width` - 出力幅
/// * `height` - 出力高さ
///
/// # 戻り値
///
/// RGBA フォーマットの `ImageBuffer`
pub fn render_frame_with_composition(
	_composition: &Composition,
	_time: f32,
	width: u32,
	height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
	// Composition は現状 Instance のみ保持するため、シーンオブジェクト描画は
	// `render_objects` 経由で行う。
	ImageBuffer::from_pixel(width, height, Rgba([30, 30, 30, 255]))
}

/// シーンオブジェクト群からフレームを生成
///
/// 指定時間で可視なオブジェクトだけを描画します。
pub fn render_objects<'a>(
	objects: impl IntoIterator<Item = &'a dyn SceneObjectData>,
	time: f32,
	width: u32,
	height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
	let mut img = ImageBuffer::from_pixel(width, height, Rgba([30, 30, 30, 255]));
	for obj in objects {
		if !obj.is_visible_at(time) {
			continue;
		}
		if let Some(rect) = obj.as_rectangle() {
			draw_rectangle(&mut img, rect, width, height);
		} else if let Some(text) = obj.as_text() {
			draw_text(&mut img, text, width, height);
		}
	}
	img
}

/// 矩形を描画（トランスフォーム適用）
///
/// # 引数
///
/// * `img` - 描画先の画像バッファ
/// * `rect` - 描画する矩形オブジェクト
/// * `canvas_width` - キャンバスの幅
/// * `canvas_height` - キャンバスの高さ
fn draw_rectangle(
	img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
	rect: &RectangleObject,
	canvas_width: u32,
	canvas_height: u32,
) {
	let transform = rect.transform;

	// キャンバス中央を原点として位置を計算
	let center_x = canvas_width as f32 / 2.0 + transform.position[0];
	let center_y = canvas_height as f32 / 2.0 + transform.position[1];

	// スケールを適用したサイズ
	let half_w = rect.width * transform.scale[0] / 2.0;
	let half_h = rect.height * transform.scale[1] / 2.0;

	// 回転角度（Z軸回転のみ対応）
	let rotation_z = transform.rotation[2].to_radians();
	let cos_r = rotation_z.cos();
	let sin_r = rotation_z.sin();

	// 色を取得（アルファブレンディング用）
	let fill_r = (rect.fill_color[0] * 255.0) as u8;
	let fill_g = (rect.fill_color[1] * 255.0) as u8;
	let fill_b = (rect.fill_color[2] * 255.0) as u8;
	let fill_a = rect.fill_color[3] * transform.opacity;

	// バウンディングボックスを計算（回転を考慮）
	let corners = [
		(-half_w, -half_h),
		(half_w, -half_h),
		(half_w, half_h),
		(-half_w, half_h),
	];

	// 回転後の角の位置を計算してバウンディングボックスを求める
	let rotated_corners: Vec<(f32, f32)> = corners
		.iter()
		.map(|(x, y)| {
			let rx = x * cos_r - y * sin_r + center_x;
			let ry = x * sin_r + y * cos_r + center_y;
			(rx, ry)
		})
		.collect();

	let min_x = rotated_corners.iter().map(|(x, _)| *x).fold(f32::INFINITY, f32::min);
	let max_x = rotated_corners
		.iter()
		.map(|(x, _)| *x)
		.fold(f32::NEG_INFINITY, f32::max);
	let min_y = rotated_corners.iter().map(|(_, y)| *y).fold(f32::INFINITY, f32::min);
	let max_y = rotated_corners
		.iter()
		.map(|(_, y)| *y)
		.fold(f32::NEG_INFINITY, f32::max);

	let x0 = (min_x as i32).max(0) as u32;
	let x1 = (max_x as u32).min(canvas_width);
	let y0 = (min_y as i32).max(0) as u32;
	let y1 = (max_y as u32).min(canvas_height);

	// 各ピクセルを走査して矩形内かどうかをチェック
	for py in y0..y1 {
		for px in x0..x1 {
			// ピクセル位置を矩形のローカル座標に変換
			let dx = px as f32 - center_x;
			let dy = py as f32 - center_y;

			// 逆回転してローカル座標を取得
			let local_x = dx * cos_r + dy * sin_r;
			let local_y = -dx * sin_r + dy * cos_r;

			// 矩形の範囲内かチェック
			if local_x >= -half_w && local_x <= half_w && local_y >= -half_h && local_y <= half_h {
				// アルファブレンディング
				if fill_a >= 1.0 {
					img.put_pixel(px, py, Rgba([fill_r, fill_g, fill_b, 255]));
				} else if fill_a > 0.0 {
					let bg = img.get_pixel(px, py);
					let blend = |fg: u8, bg: u8| (fg as f32 * fill_a + bg as f32 * (1.0 - fill_a)) as u8;
					img.put_pixel(
						px,
						py,
						Rgba([blend(fill_r, bg[0]), blend(fill_g, bg[1]), blend(fill_b, bg[2]), 255]),
					);
				}
			}
		}
	}
}

/// テキストを描画（トランスフォーム適用）
fn draw_text(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, text: &TextObject, canvas_width: u32, canvas_height: u32) {
	if text.text.is_empty() || text.font_size <= 0.0 {
		return;
	}

	let Ok(font_data) = std::fs::read(&text.font_path) else {
		log::warn!("Failed to load font: {}", text.font_path);
		return;
	};
	let Some(font) = FontRef::from_index(&font_data, 0) else {
		log::warn!("Invalid font file: {}", text.font_path);
		return;
	};

	let transform = text.transform;
	let center_x = canvas_width as f32 / 2.0 + transform.position[0];
	let center_y = canvas_height as f32 / 2.0 + transform.position[1];
	let scale_x = transform.scale[0];
	let scale_y = transform.scale[1];
	let rotation_z = transform.rotation[2].to_radians();
	let cos_r = rotation_z.cos();
	let sin_r = rotation_z.sin();

	let fill_r = text.fill_color[0].clamp(0.0, 1.0);
	let fill_g = text.fill_color[1].clamp(0.0, 1.0);
	let fill_b = text.fill_color[2].clamp(0.0, 1.0);
	let fill_a = (text.fill_color[3] * transform.opacity).clamp(0.0, 1.0);
	if fill_a <= 0.0 {
		return;
	}

	let metrics = font.metrics(&[]).scale(text.font_size);
	let glyph_metrics = font.glyph_metrics(&[]).scale(text.font_size);
	let charmap = font.charmap();
	let line_height = metrics.ascent + metrics.descent + metrics.leading;

	struct LaidOutGlyph {
		id: GlyphId,
		x: f32,
		baseline_y: f32,
	}

	let mut glyphs = Vec::new();
	let mut pen_x = 0.0_f32;
	let mut line_y = 0.0_f32;
	let mut line_width = 0.0_f32;
	let mut max_line_width = 0.0_f32;
	let mut line_count = 1_u32;

	for ch in text.text.chars() {
		if ch == '\n' {
			max_line_width = max_line_width.max(line_width);
			pen_x = 0.0;
			line_width = 0.0;
			line_y += line_height;
			line_count += 1;
			continue;
		}

		let glyph_id = charmap.map(ch);
		glyphs.push(LaidOutGlyph {
			id: glyph_id,
			x: pen_x,
			baseline_y: line_y,
		});
		let advance = glyph_metrics.advance_width(glyph_id) + text.spacing;
		pen_x += advance;
		line_width = pen_x;
	}
	max_line_width = max_line_width.max(line_width);

	if glyphs.is_empty() {
		return;
	}

	let total_height = line_height * line_count as f32 - metrics.leading;
	let origin_x = -max_line_width / 2.0;
	let origin_y = -total_height / 2.0 + metrics.ascent;

	let mut context = ScaleContext::new();
	let mut scaler = context.builder(font).size(text.font_size).hint(true).build();

	for glyph in glyphs {
		let Some(image) = Render::new(&[Source::Outline])
			.format(Format::Alpha)
			.render(&mut scaler, glyph.id)
		else {
			continue;
		};

		let placement = image.placement;
		for gy in 0..placement.height {
			for gx in 0..placement.width {
				let alpha = image.data[(gy * placement.width + gx) as usize] as f32 / 255.0;
				if alpha <= 0.0 {
					continue;
				}

				// フォント座標（y上向き）→ ローカル画面座標（y下向き、テキスト中心原点）
				let local_x = (origin_x + glyph.x + placement.left as f32 + gx as f32) * scale_x;
				let local_y = (origin_y + glyph.baseline_y - placement.top as f32 + gy as f32) * scale_y;

				let world_x = local_x * cos_r - local_y * sin_r + center_x;
				let world_y = local_x * sin_r + local_y * cos_r + center_y;

				let px = world_x.round() as i32;
				let py = world_y.round() as i32;
				if px < 0 || py < 0 || px as u32 >= canvas_width || py as u32 >= canvas_height {
					continue;
				}

				let coverage = (alpha * fill_a).clamp(0.0, 1.0);
				let bg = img.get_pixel(px as u32, py as u32);
				let blend = |fg: f32, bg: u8| (fg * 255.0 * coverage + bg as f32 * (1.0 - coverage)) as u8;
				img.put_pixel(
					px as u32,
					py as u32,
					Rgba([blend(fill_r, bg[0]), blend(fill_g, bg[1]), blend(fill_b, bg[2]), 255]),
				);
			}
		}
	}
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

#[cfg(test)]
mod tests {
	use super::*;
	use core::TextObject;

	fn rubik_path() -> String {
		format!(
			"{}/../../assets/fonts/Rubik/Rubik_regular.ttf",
			env!("CARGO_MANIFEST_DIR")
		)
	}

	#[test]
	fn draws_text_object() {
		let text = TextObject::new("Label")
			.with_text("A")
			.with_font_path(rubik_path())
			.with_font_size(64.0)
			.with_fill_color(1.0, 1.0, 1.0, 1.0);

		let img = render_objects([&text as &dyn SceneObjectData], 0.0, 200, 200);
		let lit = img.pixels().any(|p| p[0] > 40 || p[1] > 40 || p[2] > 40);
		assert!(lit, "expected text pixels brighter than background");
	}

	#[test]
	fn hidden_text_object_is_not_drawn() {
		let text = TextObject::new("Hidden")
			.with_text("A")
			.with_font_path(rubik_path())
			.with_font_size(64.0)
			.with_start_time(1.0)
			.with_duration(1.0);

		let img = render_objects([&text as &dyn SceneObjectData], 0.0, 64, 64);
		assert!(img.pixels().all(|p| *p == Rgba([30, 30, 30, 255])));
	}
}
