use iced::{
	Color, Element, Length,
	widget::{container, image},
};
use std::time::Instant;

// =============================================================================
// 定数
// =============================================================================

pub const CANVAS_WIDTH: u32 = 640;
pub const CANVAS_HEIGHT: u32 = 480;

// =============================================================================
// ピクセルバッファ
// =============================================================================

#[derive(Debug, Clone)]
pub struct PixelBuffer {
	pub width: u32,
	pub height: u32,
	pub pixels: Vec<u8>, // RGBA format
}

impl PixelBuffer {
	pub fn new(width: u32, height: u32) -> Self {
		let size = (width * height * 4) as usize;
		Self {
			width,
			height,
			pixels: vec![0; size],
		}
	}

	#[inline]
	pub fn set_pixel_rgba(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
		if x >= self.width || y >= self.height {
			return;
		}
		let idx = ((y * self.width + x) * 4) as usize;
		self.pixels[idx] = r;
		self.pixels[idx + 1] = g;
		self.pixels[idx + 2] = b;
		self.pixels[idx + 3] = a;
	}
}

// =============================================================================
// エフェクトパラメータ
// =============================================================================

#[derive(Debug, Clone)]
pub struct EffectParams {
	pub time_scale: f32,
	pub color1: Color,
	pub color2: Color,
	pub effect_type: EffectType,
	pub param1: f32,
	pub param2: f32,
	pub param3: f32,
}

impl Default for EffectParams {
	fn default() -> Self {
		Self {
			time_scale: 1.0,
			color1: Color::from_rgb(0.2, 0.5, 1.0),
			color2: Color::from_rgb(1.0, 0.3, 0.5),
			effect_type: EffectType::Plasma,
			param1: 4.0,
			param2: 0.5,
			param3: 2.0,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
	Plasma,
	Ripple,
	Fire,
	Noise,
	Gradient,
}

impl std::fmt::Display for EffectType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.name())
	}
}

impl EffectType {
	pub fn name(&self) -> &'static str {
		match self {
			EffectType::Plasma => "Plasma",
			EffectType::Ripple => "Ripple",
			EffectType::Fire => "Fire",
			EffectType::Noise => "Noise",
			EffectType::Gradient => "Gradient",
		}
	}

	pub fn all() -> &'static [EffectType] {
		&[
			EffectType::Plasma,
			EffectType::Ripple,
			EffectType::Fire,
			EffectType::Noise,
			EffectType::Gradient,
		]
	}
}

// =============================================================================
// プレビューキャンバス
// =============================================================================

#[derive(Debug)]
pub struct PreviewCanvas {
	buffer: PixelBuffer,
	params: EffectParams,
	start_time: Instant,
	noise_seed: u32,
	fire_buffer: Vec<u8>,
}

impl PreviewCanvas {
	pub fn new() -> Self {
		let buffer = PixelBuffer::new(CANVAS_WIDTH, CANVAS_HEIGHT);
		let fire_buffer = vec![0u8; (CANVAS_WIDTH * CANVAS_HEIGHT) as usize];

		Self {
			buffer,
			params: EffectParams::default(),
			start_time: Instant::now(),
			noise_seed: 12345,
			fire_buffer,
		}
	}

	pub fn get_params(&self) -> &EffectParams {
		&self.params
	}

	pub fn set_time_scale(&mut self, scale: f32) {
		self.params.time_scale = scale;
	}

	pub fn set_color1(&mut self, color: Color) {
		self.params.color1 = color;
	}

	pub fn set_color2(&mut self, color: Color) {
		self.params.color2 = color;
	}

	pub fn set_effect_type(&mut self, effect_type: EffectType) {
		self.params.effect_type = effect_type;
		if effect_type == EffectType::Fire {
			self.fire_buffer.fill(0);
		}
	}

	pub fn set_param1(&mut self, value: f32) {
		self.params.param1 = value;
	}

	pub fn set_param2(&mut self, value: f32) {
		self.params.param2 = value;
	}

	pub fn set_param3(&mut self, value: f32) {
		self.params.param3 = value;
	}

	pub fn update(&mut self, _message: PreviewCanvasMessage) {
		// 現在は特に処理なし
	}

	/// フレームを更新（リアルタイム描画）
	pub fn tick(&mut self) {
		let time = self.start_time.elapsed().as_secs_f32() * self.params.time_scale;

		match self.params.effect_type {
			EffectType::Plasma => self.render_plasma(time),
			EffectType::Ripple => self.render_ripple(time),
			EffectType::Fire => self.render_fire(),
			EffectType::Noise => self.render_noise(time),
			EffectType::Gradient => self.render_gradient(time),
		}
	}

	fn render_plasma(&mut self, time: f32) {
		let freq = self.params.param1;
		let complexity = self.params.param3;

		for y in 0..self.buffer.height {
			for x in 0..self.buffer.width {
				let fx = x as f32 / self.buffer.width as f32;
				let fy = y as f32 / self.buffer.height as f32;

				let v1 = (fx * freq * std::f32::consts::PI + time).sin();
				let v2 = (fy * freq * std::f32::consts::PI + time * 0.7).sin();
				let v3 = ((fx + fy) * freq * 0.5 * std::f32::consts::PI + time * 1.3).sin();
				let v4 = (((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt() * freq * complexity + time).sin();

				let v = (v1 + v2 + v3 + v4) / 4.0;
				let t = (v + 1.0) / 2.0;

				let r = self.params.color1.r * (1.0 - t) + self.params.color2.r * t;
				let g = self.params.color1.g * (1.0 - t) + self.params.color2.g * t;
				let b = self.params.color1.b * (1.0 - t) + self.params.color2.b * t;

				self.buffer.set_pixel_rgba(x, y, (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255);
			}
		}
	}

	fn render_ripple(&mut self, time: f32) {
		let freq = self.params.param1;
		let speed = self.params.param2 * 10.0;
		let decay = self.params.param3;

		let cx = self.buffer.width as f32 / 2.0;
		let cy = self.buffer.height as f32 / 2.0;

		for y in 0..self.buffer.height {
			for x in 0..self.buffer.width {
				let dx = x as f32 - cx;
				let dy = y as f32 - cy;
				let dist = (dx * dx + dy * dy).sqrt();

				let wave = ((dist * freq * 0.05 - time * speed).sin() + 1.0) / 2.0;
				let fade = (-dist * 0.005 * decay).exp();
				let v = wave * fade;

				let r = self.params.color1.r * (1.0 - v) + self.params.color2.r * v;
				let g = self.params.color1.g * (1.0 - v) + self.params.color2.g * v;
				let b = self.params.color1.b * (1.0 - v) + self.params.color2.b * v;

				self.buffer.set_pixel_rgba(x, y, (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255);
			}
		}
	}

	fn render_fire(&mut self) {
		let width = self.buffer.width as usize;
		let height = self.buffer.height as usize;
		let cooling = (self.params.param1 * 5.0) as u8;
		let spread = self.params.param2.max(0.1);

		// 最下行にランダムな炎の種を生成
		for x in 0..width {
			let idx = (height - 1) * width + x;
			self.fire_buffer[idx] = if self.fast_random() > 0.4 { 255 } else { 0 };
		}

		// 下から上へ炎を伝播（最下行は除く）
		for y in 1..height {
			let current_y = height - 1 - y; // 上から見たインデックス
			let below_y = current_y + 1;    // 一つ下の行

			for x in 0..width {
				let idx = current_y * width + x;
				let below_idx = below_y * width;

				let left = if x > 0 { self.fire_buffer[below_idx + x - 1] as u16 } else { 0 };
				let center = self.fire_buffer[below_idx + x] as u16;
				let right = if x < width - 1 { self.fire_buffer[below_idx + x + 1] as u16 } else { 0 };

				let avg = ((left + center + right) as f32 / 3.0 * spread) as u16;
				let cooled = avg.saturating_sub(cooling as u16);
				self.fire_buffer[idx] = cooled.min(255) as u8;
			}
		}

		// ピクセルバッファに描画
		for y in 0..height {
			for x in 0..width {
				let idx = y * width + x;
				let heat = self.fire_buffer[idx];
				let (r, g, b) = self.heat_to_color(heat);
				self.buffer.set_pixel_rgba(x as u32, y as u32, r, g, b, 255);
			}
		}
	}

	fn heat_to_color(&self, heat: u8) -> (u8, u8, u8) {
		let t = heat as f32 / 255.0;

		if t < 0.33 {
			let s = t / 0.33;
			(
				(self.params.color1.r * s * 255.0) as u8,
				(self.params.color1.g * s * 255.0) as u8,
				(self.params.color1.b * s * 255.0) as u8,
			)
		} else if t < 0.66 {
			let s = (t - 0.33) / 0.33;
			(
				((self.params.color1.r * (1.0 - s) + self.params.color2.r * s) * 255.0) as u8,
				((self.params.color1.g * (1.0 - s) + self.params.color2.g * s) * 255.0) as u8,
				((self.params.color1.b * (1.0 - s) + self.params.color2.b * s) * 255.0) as u8,
			)
		} else {
			let s = (t - 0.66) / 0.34;
			(
				((self.params.color2.r * (1.0 - s) + s) * 255.0) as u8,
				((self.params.color2.g * (1.0 - s) + s) * 255.0) as u8,
				((self.params.color2.b * (1.0 - s) + s) * 255.0) as u8,
			)
		}
	}

	fn render_noise(&mut self, time: f32) {
		let scale = self.params.param1;
		let octaves = self.params.param3 as u32;
		let speed = self.params.param2;

		for y in 0..self.buffer.height {
			for x in 0..self.buffer.width {
				let fx = x as f32 / self.buffer.width as f32 * scale;
				let fy = y as f32 / self.buffer.height as f32 * scale;

				let mut noise_val = 0.0f32;
				let mut amplitude = 1.0f32;
				let mut freq = 1.0f32;

				for _ in 0..octaves {
					noise_val += self.perlin_noise(fx * freq + time * speed, fy * freq) * amplitude;
					amplitude *= 0.5;
					freq *= 2.0;
				}

				let v = ((noise_val + 1.0) / 2.0).clamp(0.0, 1.0);

				let r = self.params.color1.r * (1.0 - v) + self.params.color2.r * v;
				let g = self.params.color1.g * (1.0 - v) + self.params.color2.g * v;
				let b = self.params.color1.b * (1.0 - v) + self.params.color2.b * v;

				self.buffer.set_pixel_rgba(x, y, (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255);
			}
		}
	}

	fn render_gradient(&mut self, time: f32) {
		let angle = time * self.params.param2;
		let freq = self.params.param1;

		let cos_a = angle.cos();
		let sin_a = angle.sin();

		for y in 0..self.buffer.height {
			for x in 0..self.buffer.width {
				let fx = x as f32 / self.buffer.width as f32 - 0.5;
				let fy = y as f32 / self.buffer.height as f32 - 0.5;

				let rotated = fx * cos_a + fy * sin_a;
				let v = ((rotated * freq + 0.5).fract() + 1.0).fract();

				let r = self.params.color1.r * (1.0 - v) + self.params.color2.r * v;
				let g = self.params.color1.g * (1.0 - v) + self.params.color2.g * v;
				let b = self.params.color1.b * (1.0 - v) + self.params.color2.b * v;

				self.buffer.set_pixel_rgba(x, y, (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255);
			}
		}
	}

	fn perlin_noise(&self, x: f32, y: f32) -> f32 {
		let x0 = x.floor() as i32;
		let y0 = y.floor() as i32;
		let x1 = x0 + 1;
		let y1 = y0 + 1;

		let sx = x - x0 as f32;
		let sy = y - y0 as f32;

		let sx = sx * sx * (3.0 - 2.0 * sx);
		let sy = sy * sy * (3.0 - 2.0 * sy);

		let n00 = self.gradient_dot(x0, y0, x, y);
		let n10 = self.gradient_dot(x1, y0, x, y);
		let n01 = self.gradient_dot(x0, y1, x, y);
		let n11 = self.gradient_dot(x1, y1, x, y);

		let nx0 = n00 * (1.0 - sx) + n10 * sx;
		let nx1 = n01 * (1.0 - sx) + n11 * sx;

		nx0 * (1.0 - sy) + nx1 * sy
	}

	fn gradient_dot(&self, ix: i32, iy: i32, x: f32, y: f32) -> f32 {
		let hash = self.hash(ix, iy);
		let angle = hash as f32 * 2.0 * std::f32::consts::PI / 256.0;
		let gx = angle.cos();
		let gy = angle.sin();
		let dx = x - ix as f32;
		let dy = y - iy as f32;
		gx * dx + gy * dy
	}

	fn hash(&self, x: i32, y: i32) -> u8 {
		let mut h = self.noise_seed;
		h ^= x as u32;
		h = h.wrapping_mul(0x85ebca6b);
		h ^= y as u32;
		h = h.wrapping_mul(0xc2b2ae35);
		h ^= h >> 16;
		(h & 0xFF) as u8
	}

	fn fast_random(&mut self) -> f32 {
		self.noise_seed = self.noise_seed.wrapping_mul(1103515245).wrapping_add(12345);
		(self.noise_seed >> 16) as f32 / 65535.0
	}

	pub fn view(&self) -> Element<'_, PreviewCanvasMessage> {
		let handle = image::Handle::from_rgba(
			self.buffer.width,
			self.buffer.height,
			self.buffer.pixels.clone(),
		);

		let img = image::Image::new(handle)
			.width(Length::Fill)
			.height(Length::Fill)
			.content_fit(iced::ContentFit::Contain);

		container(img)
			.width(Length::Fill)
			.height(Length::Fill)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.style(|_| container::Style {
				background: Some(Color::from_rgb(0.05, 0.05, 0.05).into()),
				..Default::default()
			})
			.into()
	}
}

// =============================================================================
// メッセージ
// =============================================================================

#[derive(Debug, Clone)]
pub enum PreviewCanvasMessage {
	Tick,
}
