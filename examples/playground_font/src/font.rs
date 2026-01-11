use egui::{Color32, ColorImage, Pos2, Response, Stroke, TextureOptions, Ui, pos2};

use crate::glyph_engine::{self, FeatureInfo, GlyphEngine, ShapedGlyph, VariationInfo};

/// フォントフィーチャーの有効/無効状態
#[derive(Debug, Clone)]
struct FeatureState {
	info: FeatureInfo,
	enabled: bool,
}

/// バリエーション軸の現在値
#[derive(Debug, Clone)]
struct VariationState {
	info: VariationInfo,
	value: f32,
}

pub struct FontPlaygroundWidget {
	glyph_engine: Option<GlyphEngine>,

	// フォント情報
	font_path: String,
	font_name: String,

	// UI状態
	input_text: String,
	font_size: f32,
	fill_mode: bool, // true: 塗りつぶし, false: アウトライン

	// フィーチャーとバリエーション
	features: Vec<FeatureState>,
	variations: Vec<VariationState>,

	// シェイプ結果キャッシュ
	shaped_glyphs: Vec<ShapedGlyph>,
	needs_reshape: bool,

	// エラーメッセージ
	error_message: Option<String>,
}

impl Default for FontPlaygroundWidget {
	fn default() -> Self {
		let default_path = "./assets/fonts/Rubik/Rubik_regular.ttf";
		let mut widget = Self {
			glyph_engine: None,
			font_path: default_path.to_string(),
			font_name: "Rubik Regular".to_string(),
			input_text: "The quick brown fox jumps over the lazy dog. fi fl ff ffi".to_string(),
			font_size: 64.0,
			fill_mode: true,
			features: Vec::new(),
			variations: Vec::new(),
			shaped_glyphs: Vec::new(),
			needs_reshape: true,
			error_message: None,
		};

		widget.load_font(default_path);
		widget
	}
}

impl FontPlaygroundWidget {
	/// フォントを読み込む
	fn load_font(&mut self, path: &str) {
		match glyph_engine::GlyphEngine::new(path) {
			Some(engine) => {
				// フォントのフィーチャーを取得
				let features: Vec<FeatureState> = engine
					.get_features()
					.into_iter()
					.map(|info| FeatureState {
						info,
						enabled: false,
					})
					.collect();

				// フォントのバリエーション軸を取得
				let variations: Vec<VariationState> = engine
					.get_variations()
					.into_iter()
					.map(|info| {
						let value = info.default_value;
						VariationState { info, value }
					})
					.collect();

				// ファイル名からフォント名を抽出
				let font_name = std::path::Path::new(path)
					.file_stem()
					.and_then(|s| s.to_str())
					.unwrap_or("Unknown")
					.to_string();

				self.glyph_engine = Some(engine);
				self.font_path = path.to_string();
				self.font_name = font_name;
				self.features = features;
				self.variations = variations;
				self.shaped_glyphs.clear();
				self.needs_reshape = true;
				self.error_message = None;
			}
			None => {
				self.error_message = Some(format!("フォントの読み込みに失敗しました: {}", path));
			}
		}
	}

	/// ファイルピッカーを開いてフォントを選択
	fn open_font_picker(&mut self) {
		if let Some(path) = rfd::FileDialog::new()
			.add_filter("Font Files", &["ttf", "otf", "ttc", "otc"])
			.add_filter("All Files", &["*"])
			.pick_file()
			&& let Some(path_str) = path.to_str()
		{
			self.load_font(path_str);
		}
	}

	fn reshape_if_needed(&mut self) {
		if !self.needs_reshape {
			return;
		}

		let Some(ref mut engine) = self.glyph_engine else {
			return;
		};

		// 有効なフィーチャーを収集
		let active_features: Vec<(&str, u16)> = self
			.features
			.iter()
			.filter(|f| f.enabled)
			.map(|f| (f.info.tag.as_str(), 1u16))
			.collect();

		// 現在のバリエーション値を収集
		let active_variations: Vec<(&str, f32)> = self
			.variations
			.iter()
			.map(|v| (v.info.tag.as_str(), v.value))
			.collect();

		self.shaped_glyphs = engine.shape_text(
			&self.input_text,
			self.font_size,
			&active_features,
			&active_variations,
		);

		self.needs_reshape = false;
	}

	pub fn ui(&mut self, ui: &mut Ui) -> Response {
		// シェイピングを実行
		self.reshape_if_needed();

		egui::SidePanel::left("feature_panel")
			.resizable(true)
			.default_width(280.0)
			.show_inside(ui, |ui| {
				ui.heading("🔤 Font Feature Playground");
				ui.separator();

				// フォント選択
				ui.label("Font:");
				ui.horizontal(|ui| {
					ui.label(&self.font_name);
					if ui.button("📂 Open...").clicked() {
						self.open_font_picker();
					}
				});

				// エラーメッセージ表示
				if let Some(ref error) = self.error_message {
					ui.colored_label(Color32::RED, error);
				}
				ui.add_space(8.0);

				// テキスト入力
				ui.label("Sample Text:");
				if ui.text_edit_singleline(&mut self.input_text).changed() {
					self.needs_reshape = true;
				}
				ui.add_space(8.0);

				// フォントサイズ
				ui.label("Font Size:");
				if ui
					.add(egui::Slider::new(&mut self.font_size, 12.0..=200.0).suffix("px"))
					.changed()
				{
					self.needs_reshape = true;
				}
				ui.add_space(8.0);

				// 描画モード
				ui.checkbox(&mut self.fill_mode, "🎨 Fill Mode (vs Outline)");
				ui.add_space(16.0);

				// バリエーション軸（Variable Font用）
				if !self.variations.is_empty() {
					ui.heading("📐 Variation Axes");
					ui.separator();

					for var in &mut self.variations {
						ui.horizontal(|ui| {
							ui.label(&var.info.tag);
							ui.label(&var.info.name);
						});
						if ui
							.add(
								egui::Slider::new(
									&mut var.value,
									var.info.min_value..=var.info.max_value,
								)
								.show_value(true),
							)
							.changed()
						{
							self.needs_reshape = true;
						}
						ui.add_space(4.0);
					}
					ui.add_space(16.0);
				}

				// OpenTypeフィーチャー
				ui.heading("✨ OpenType Features");
				ui.separator();

				if self.features.is_empty() {
					ui.label("No features available in this font.");
				} else {
					egui::ScrollArea::vertical()
						.max_height(400.0)
						.show(ui, |ui| {
							for feature in &mut self.features {
								ui.horizontal(|ui| {
									if ui.checkbox(&mut feature.enabled, "").changed() {
										self.needs_reshape = true;
									}
									ui.monospace(&feature.info.tag);
									if let Some(name) = &feature.info.name {
										ui.label(name);
									} else {
										ui.label(&feature.info.action);
									}
								});
							}
						});
				}
			});

		// メインプレビュー領域
		egui::CentralPanel::default()
			.show_inside(ui, |ui| {
				ui.heading("Preview");
				ui.separator();

				// グリフ情報を表示
				ui.horizontal(|ui| {
					ui.label(format!("Glyphs: {}", self.shaped_glyphs.len()));
				});

				ui.add_space(16.0);

				// グリフを描画
				let (response, painter) = ui.allocate_painter(
					egui::vec2(ui.available_width(), 400.0),
					egui::Sense::hover(),
				);

				let rect = response.rect;
				painter.rect_filled(rect, 0.0, Color32::from_gray(32));

				// ベースライン位置
				let baseline_y = rect.top() + 150.0;
				let start_x = rect.left() + 20.0;

				// ベースラインを描画
				painter.line_segment(
					[
						pos2(rect.left(), baseline_y),
						pos2(rect.right(), baseline_y),
					],
					Stroke::new(1.0, Color32::from_gray(80)),
				);

				// 各グリフを描画
				// アクティブなバリエーションを収集
				let active_variations: Vec<(&str, f32)> = self
					.variations
					.iter()
					.map(|v| (v.info.tag.as_str(), v.value))
					.collect();

				for shaped in &self.shaped_glyphs {
					let glyph_x = start_x + shaped.x;
					let glyph_y = baseline_y - shaped.y;
					let stroke = Stroke::new(1.5, Color32::from_rgb(100, 200, 255));

					if self.fill_mode {
						// 塗りつぶしモード（ラスタライズ）
						let rasterized = self.glyph_engine.as_mut().and_then(|engine| {
							engine.rasterize_glyph(shaped.id, self.font_size, &active_variations)
						});

						if let Some(glyph) = rasterized
							&& glyph.width > 0 && glyph.height > 0
						{
							let size = [glyph.width as usize, glyph.height as usize];
							let mut pixels = Vec::with_capacity(size[0] * size[1] * 4);
							for alpha in glyph.data {
								let color = Color32::from_rgba_premultiplied(100, 200, 255, alpha);
								pixels.extend_from_slice(&color.to_array());
							}

							let image = ColorImage::from_rgba_unmultiplied(size, &pixels);
							let texture = ui.ctx().load_texture(
								format!("glyph_{}", shaped.id),
								image,
								TextureOptions::LINEAR,
							);

							let pos =
								pos2(glyph_x + glyph.left as f32, baseline_y - glyph.top as f32);
							let rect = egui::Rect::from_min_size(
								pos,
								egui::vec2(glyph.width as f32, glyph.height as f32),
							);

							painter.image(
								texture.id(),
								rect,
								egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
								Color32::WHITE,
							);
						}
					} else {
						// アウトラインモード（パス描画）
						// グリフのパスを取得して描画（font_sizeで直接取得するのでスケール不要）
						let path = self.glyph_engine.as_mut().and_then(|engine| {
							engine.get_glyph_path(shaped.id, self.font_size, &active_variations)
						});

						if let Some(path) = path {
							let mut current_pos = Pos2::new(glyph_x, glyph_y);
							let mut path_start = current_pos; // パスの開始点を記録

							for cmd in &path {
								match cmd {
									swash::zeno::Command::MoveTo(v) => {
										current_pos = pos2(glyph_x + v.x, glyph_y - v.y);
										path_start = current_pos; // 新しいサブパスの開始点
									}
									swash::zeno::Command::LineTo(v) => {
										let next_pos = pos2(glyph_x + v.x, glyph_y - v.y);
										painter.line_segment([current_pos, next_pos], stroke);
										current_pos = next_pos;
									}
									swash::zeno::Command::QuadTo(v1, v2) => {
										// 2次ベジェ曲線を複数セグメントで描画
										let p0 = current_pos;
										let p1 = pos2(glyph_x + v1.x, glyph_y - v1.y);
										let p2 = pos2(glyph_x + v2.x, glyph_y - v2.y);

										const SEGMENTS: usize = 8;
										let mut prev = p0;
										for i in 1..=SEGMENTS {
											let t = i as f32 / SEGMENTS as f32;
											let t1 = 1.0 - t;
											// B(t) = (1-t)²P0 + 2(1-t)tP1 + t²P2
											let x =
												t1 * t1 * p0.x + 2.0 * t1 * t * p1.x + t * t * p2.x;
											let y =
												t1 * t1 * p0.y + 2.0 * t1 * t * p1.y + t * t * p2.y;
											let next = pos2(x, y);
											painter.line_segment([prev, next], stroke);
											prev = next;
										}
										current_pos = p2;
									}
									swash::zeno::Command::CurveTo(v1, v2, v3) => {
										// 3次ベジェ曲線を複数セグメントで描画
										let p0 = current_pos;
										let p1 = pos2(glyph_x + v1.x, glyph_y - v1.y);
										let p2 = pos2(glyph_x + v2.x, glyph_y - v2.y);
										let p3 = pos2(glyph_x + v3.x, glyph_y - v3.y);

										const SEGMENTS: usize = 12;
										let mut prev = p0;
										for i in 1..=SEGMENTS {
											let t = i as f32 / SEGMENTS as f32;
											let t1 = 1.0 - t;
											// B(t) = (1-t)³P0 + 3(1-t)²tP1 + 3(1-t)t²P2 + t³P3
											let x = t1 * t1 * t1 * p0.x
												+ 3.0 * t1 * t1 * t * p1.x + 3.0
												* t1 * t * t * p2
												.x + t * t * t * p3.x;
											let y = t1 * t1 * t1 * p0.y
												+ 3.0 * t1 * t1 * t * p1.y + 3.0
												* t1 * t * t * p2
												.y + t * t * t * p3.y;
											let next = pos2(x, y);
											painter.line_segment([prev, next], stroke);
											prev = next;
										}
										current_pos = p3;
									}
									swash::zeno::Command::Close => {
										// パスを閉じる - 開始点に戻る
										if current_pos != path_start {
											painter.line_segment([current_pos, path_start], stroke);
										}
										current_pos = path_start;
									}
								}
							}
						}
					}
				}

				// 有効なフィーチャーを表示
				ui.add_space(16.0);
				let enabled_features: Vec<&str> = self
					.features
					.iter()
					.filter(|f| f.enabled)
					.map(|f| f.info.tag.as_str())
					.collect();

				if !enabled_features.is_empty() {
					ui.horizontal(|ui| {
						ui.label("Active features:");
						for tag in enabled_features {
							ui.monospace(format!("{} ", tag));
						}
					});
				}

				response
			})
			.response
	}
}
