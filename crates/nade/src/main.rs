//! Entry point of Nade

use eframe::{
	egui::{self, Vec2},
	egui_wgpu, wgpu,
};
use egui::{Color32, ColorImage, TextureHandle};

fn main() -> eframe::Result<()> {
	let native_options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
		vsync: false,
		wgpu_options: egui_wgpu::WgpuConfiguration {
			present_mode: wgpu::PresentMode::Immediate,
			..Default::default()
		},
		..Default::default()
	};

	eframe::run_native(
		"Raw Pixel Viewer",
		native_options,
		Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
	)
}

struct MyApp {
	width: usize,
	height: usize,
	pixels: Vec<Color32>,
	texture: Option<TextureHandle>,
	time: f32,
	status_bar: status_bar::StatusBar,
}

impl MyApp {
	fn new(_cc: &eframe::CreationContext<'_>) -> Self {
		let width = 320;
		let height = 240;
		let pixels = vec![Color32::BLACK; width * height];

		Self {
			width,
			height,
			pixels,
			texture: None,
			time: 0.0,
			status_bar: status_bar::StatusBar::default(),
		}
	}

	/// ここで生ピクセルを書き換える（リアルタイム更新のコア）
	fn update_pixels(&mut self, dt: f32) {
		self.time += dt;

		let w = self.width as i32;
		let h = self.height as i32;
		let t = self.time;

		for y in 0..h {
			for x in 0..w {
				let fx = x as f32 / w as f32;
				let fy = y as f32 / h as f32;

				// 適当な動く模様（サイン波＋時間）
				let r = ((fx * 10.0 + t).sin() * 0.5 + 0.5) * 255.0;
				let g = ((fy * 10.0 - t * 1.3).cos() * 0.5 + 0.5) * 255.0;
				let d = ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt();
				let b = (((d * 20.0) - t * 0.7).sin() * 0.5 + 0.5) * 255.0;

				let idx = (y as usize) * (w as usize) + (x as usize);
				self.pixels[idx] = Color32::from_rgb(r as u8, g as u8, b as u8);
			}
		}
	}
}

impl eframe::App for MyApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		let mut fonts = egui::FontDefinitions::default();
		let font_data = egui::FontData::from_static(include_bytes!(
			"../../../assets/fonts/Inter/Inter_regular.otf"
		));

		fonts
			.font_data
			.insert("rubik".to_string(), font_data.into());

		fonts
			.families
			.entry(egui::FontFamily::Proportional)
			.or_default()
			.insert(0, "rubik".to_string());

		ctx.set_fonts(fonts);

		egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
			self.status_bar.ui(ui);
		});

		// Δt をざっくり計算（タイミング厳密でなくてよければこれで十分）
		let dt = ctx.input(|i| i.stable_dt).max(0.001); // 0 に潰れないように
		self.update_pixels(dt);

		// テクスチャの初期化 or 更新
		let texture: &mut TextureHandle = self.texture.get_or_insert_with(|| {
			let image = ColorImage {
				source_size: Vec2::new(self.width as f32, self.height as f32),
				size: [self.width, self.height],
				pixels: self.pixels.clone(),
			};
			ctx.load_texture(
				"raw_pixels_texture",
				image,
				egui::TextureOptions::NEAREST, // ドット絵感出したいなら NEAREST
			)
		});

		// 既存テクスチャに新しいピクセルを流し込む
		texture.set(
			ColorImage {
				source_size: Vec2::new(self.width as f32, self.height as f32),
				size: [self.width, self.height],
				pixels: self.pixels.clone(),
			},
			egui::TextureOptions::NEAREST,
		);

		egui::CentralPanel::default().show(ctx, |ui| {
			ui.heading("Raw Pixel Viewer (eframe + egui)");

			// ウィンドウにフィットさせて描画
			let available = ui.available_size();
			let img_size = {
				// アスペクト比維持して拡大
				let scale_x = available.x / self.width as f32;
				let scale_y = available.y / self.height as f32;
				let scale = scale_x.min(scale_y);
				egui::vec2(self.width as f32 * scale, self.height as f32 * scale)
			};

			ui.add(egui::Slider::new(&mut self.time, 0.0..=10.0).text("Time"));
			let response = ui.add(egui::Image::new(&*texture).fit_to_exact_size(img_size));

			let rect = response.rect.intersect(ui.max_rect());
			let fps = 1.0 / dt;
			let fps_text = format!("{:.3} FPS", fps);
			let painter = ui.painter();

			// Background box
			let bg_rect =
				egui::Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(80.0, 24.0));
			painter.rect_filled(bg_rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180));

			// Text
			painter.text(
				bg_rect.left_top() + egui::vec2(4.0, 3.0),
				egui::Align2::LEFT_TOP,
				fps_text,
				egui::TextStyle::Monospace.resolve(ui.style()),
				Color32::WHITE,
			);
		});

		// 毎フレーム再描画してほしいので
		ctx.request_repaint();
	}
}
