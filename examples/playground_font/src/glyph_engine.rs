use swash::{
	Attributes, CacheKey, Charmap, FontRef, Setting,
	scale::{Render, ScaleContext, Source},
	shape::ShapeContext,
	text::Script,
	zeno::{Command, Format, PathData},
};

/// フォントがサポートするOpenTypeフィーチャー情報
#[derive(Debug, Clone)]
pub struct FeatureInfo {
	/// 4文字のタグ (例: "liga", "dlig", "smcp")
	pub tag: String,
	/// フィーチャー名（利用可能な場合）
	pub name: Option<String>,
	/// フィーチャーの動作種別
	pub action: String,
}

/// Variable Fontのバリエーション軸情報
#[derive(Debug, Clone)]
pub struct VariationInfo {
	/// 4文字のタグ (例: "wght", "wdth", "slnt")
	pub tag: String,
	/// 軸の名前
	pub name: String,
	/// 最小値
	pub min_value: f32,
	/// デフォルト値
	pub default_value: f32,
	/// 最大値
	pub max_value: f32,
}

/// シェイプ結果のグリフ情報
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
	/// グリフID
	pub id: u16,
	/// X方向オフセット
	pub x: f32,
	/// Y方向オフセット
	pub y: f32,
	/// 送り幅
	pub advance: f32,
}

/// ラスタライズされたグリフ画像
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
	/// 画像の幅
	pub width: u32,
	/// 画像の高さ
	pub height: u32,
	/// ベースラインからの上方向オフセット
	pub top: i32,
	/// 原点からの左方向オフセット
	pub left: i32,
	/// アルファチャンネルデータ（グレースケール）
	pub data: Vec<u8>,
}

pub struct Font {
	// Full content of the font file
	data: Vec<u8>,

	// Offset to the table directory
	offset: u32,

	// Cache key
	key: CacheKey,
}

impl Font {
	pub fn from_file(path: &str, index: usize) -> Option<Self> {
		let data = std::fs::read(path).ok()?;

		// Create a temporary font reference for the first font in the file.
		// This will do some basic validation, compute the necessary offset
		// and generate a fresh cache key for us.
		let font = FontRef::from_index(&data, index)?;
		let (offset, key) = (font.offset, font.key);

		// Return our struct with the original file data and copies of the
		// offset and key from the font reference
		Some(Self { data, offset, key })
	}

	// As a convenience, you may want to forward some methods.
	pub fn attributes(&self) -> Attributes {
		self.as_ref().attributes()
	}

	pub fn charmap(&self) -> Charmap {
		self.as_ref().charmap()
	}

	// Create the transient font reference for accessing this crate's
	// functionality.
	pub fn as_ref(&self) -> FontRef {
		// Note that you'll want to initialize the struct directly here as
		// using any of the FontRef constructors will generate a new key which,
		// while completely safe, will nullify the performance optimizations of
		// the caching mechanisms used in this crate.
		FontRef {
			data: &self.data,
			offset: self.offset,
			key: self.key,
		}
	}
}

pub struct GlyphEngine {
	font_data: Font,
	shape_context: ShapeContext,
	scale_context: ScaleContext,
}

impl GlyphEngine {
	pub fn new(font_path: &str) -> Option<Self> {
		let font_data = Font::from_file(font_path, 0)?;

		Some(Self {
			font_data,
			shape_context: ShapeContext::new(),
			scale_context: ScaleContext::new(),
		})
	}

	/// フォントへの参照を取得
	pub fn font_ref(&self) -> FontRef {
		self.font_data.as_ref()
	}

	/// フォントがサポートするOpenTypeフィーチャー一覧を取得
	pub fn get_features(&self) -> Vec<FeatureInfo> {
		self.font_data
			.as_ref()
			.features()
			.map(|feature| {
				let tag_bytes = feature.tag().to_be_bytes();
				let tag = String::from_utf8_lossy(&tag_bytes).to_string();
				FeatureInfo {
					tag,
					name: feature.name().map(|s| s.to_string()),
					action: format!("{:?}", feature.action()),
				}
			})
			.collect()
	}

	/// Variable Fontのバリエーション軸情報を取得
	pub fn get_variations(&self) -> Vec<VariationInfo> {
		self.font_data
			.as_ref()
			.variations()
			.map(|var| {
				let tag_bytes = var.tag().to_be_bytes();
				let tag = String::from_utf8_lossy(&tag_bytes).to_string();
				VariationInfo {
					tag,
					name: var
						.name(None)
						.map(|s| s.to_string())
						.unwrap_or_else(|| "Unknown".to_string()),
					min_value: var.min_value(),
					default_value: var.default_value(),
					max_value: var.max_value(),
				}
			})
			.collect()
	}

	/// テキストをシェイピングしてグリフ情報のリストを返す
	pub fn shape_text(
		&mut self,
		text: &str,
		size: f32,
		features: &[(&str, u16)],
		variations: &[(&str, f32)],
	) -> Vec<ShapedGlyph> {
		let font = self.font_data.as_ref();

		// フィーチャー設定を変換
		let feature_settings: Vec<Setting<u16>> = features
			.iter()
			.map(|(tag, value)| {
				let tag_u32 = swash::tag_from_str_lossy(tag);
				Setting {
					tag: tag_u32,
					value: *value,
				}
			})
			.collect();

		// バリエーション設定を変換
		let variation_settings: Vec<Setting<f32>> = variations
			.iter()
			.map(|(tag, value)| {
				let tag_u32 = swash::tag_from_str_lossy(tag);
				Setting {
					tag: tag_u32,
					value: *value,
				}
			})
			.collect();

		// Shaperを構築
		let mut shaper = self
			.shape_context
			.builder(font)
			.script(Script::Latin)
			.size(size)
			.features(feature_settings.iter().copied())
			.variations(variation_settings.iter().copied())
			.build();

		// テキストを追加
		shaper.add_str(text);

		// シェイプ結果を収集
		let mut glyphs = Vec::new();
		let mut x_pos = 0.0_f32;

		shaper.shape_with(|cluster| {
			for glyph in cluster.glyphs {
				glyphs.push(ShapedGlyph {
					id: glyph.id,
					x: x_pos + glyph.x,
					y: glyph.y,
					advance: glyph.advance,
				});
				x_pos += glyph.advance;
			}
		});

		glyphs
	}

	/// グリフIDからパスコマンドを取得
	/// グリフIDからパスコマンドを取得
	pub fn get_glyph_path(
		&mut self,
		glyph_id: u16,
		size: f32,
		variations: &[(&str, f32)],
	) -> Option<Vec<Command>> {
		// バリエーション設定を変換
		let variation_settings: Vec<Setting<f32>> = variations
			.iter()
			.map(|(tag, value)| {
				let tag_u32 = swash::tag_from_str_lossy(tag);
				Setting {
					tag: tag_u32,
					value: *value,
				}
			})
			.collect();

		let mut scaler = self
			.scale_context
			.builder(self.font_data.as_ref())
			.size(size)
			.hint(true)
			.variations(variation_settings.iter().copied())
			.build();

		let outline = scaler.scale_outline(glyph_id)?;
		let commands: Vec<Command> = outline.path().commands().collect();

		Some(commands)
	}

	/// グリフをラスタライズして塗りつぶし画像を取得
	pub fn rasterize_glyph(
		&mut self,
		glyph_id: u16,
		size: f32,
		variations: &[(&str, f32)],
	) -> Option<RasterizedGlyph> {
		// バリエーション設定を変換
		let variation_settings: Vec<Setting<f32>> = variations
			.iter()
			.map(|(tag, value)| {
				let tag_u32 = swash::tag_from_str_lossy(tag);
				Setting {
					tag: tag_u32,
					value: *value,
				}
			})
			.collect();

		let mut scaler = self
			.scale_context
			.builder(self.font_data.as_ref())
			.size(size)
			.hint(true)
			.variations(variation_settings.iter().copied())
			.build();

		// アウトラインを取得
		let _outline = scaler.scale_outline(glyph_id)?;

		// レンダラーを構築してラスタライズ
		let mut render = Render::new(&[Source::Outline]);
		render.format(Format::Alpha);

		let mut image_data = Vec::new();
		let image = render.render(&mut scaler, glyph_id)?;

		image_data.extend_from_slice(image.data.as_slice());

		Some(RasterizedGlyph {
			width: image.placement.width,
			height: image.placement.height,
			top: image.placement.top,
			left: image.placement.left,
			data: image_data,
		})
	}

	pub fn get_glyph_points(&mut self, character: char) -> Option<Vec<(f32, f32)>> {
		let mut scaler = self
			.scale_context
			.builder(self.font_data.as_ref())
			.size(128.)
			.hint(true)
			.build();

		// Map the character to a glyph ID
		let charmap = self.font_data.charmap();
		let glyph_id = charmap.map(character as u32);

		// Scale the outline for the glyph
		let outline = scaler.scale_outline(glyph_id).unwrap();
		let points = outline.points();

		// Collect the points into a vector of (x, y) tuples
		let point_vec: Vec<(f32, f32)> = points.iter().map(|p| (p.x, p.y)).collect();

		Some(point_vec)
	}

	pub fn get_bounds(&mut self, character: char) -> Option<swash::zeno::Bounds> {
		let mut scaler = self
			.scale_context
			.builder(self.font_data.as_ref())
			.size(128.)
			.hint(true)
			.build();

		// Map the character to a glyph ID
		let charmap = self.font_data.charmap();
		let glyph_id = charmap.map(character as u32);

		// Get the bounds for the glyph
		let outline = scaler.scale_outline(glyph_id)?;
		let bounds = outline.bounds();

		Some(bounds)
	}

	pub fn get_path(&mut self, character: char) -> Option<Vec<Command>> {
		let mut scaler = self
			.scale_context
			.builder(self.font_data.as_ref())
			.size(128.)
			.hint(true)
			.build();

		// Map the character to a glyph ID
		let charmap = self.font_data.charmap();
		let glyph_id = charmap.map(character as u32);

		// Get the outline for the glyph and collect path commands into an owned Vec
		let outline = scaler.scale_outline(glyph_id)?;
		let commands: Vec<Command> = outline.path().commands().collect();

		Some(commands)
	}
}

pub fn print_localized_strings(font_path: &str) -> Option<()> {
	use swash::FontRef;

	// Read the full font file
	let font_data = std::fs::read(font_path).ok()?;

	// Create a font reference for the first font in the file
	let font = FontRef::from_index(&font_data, 0)?;

	// Print the font attributes (stretch, weight and style)
	log::debug!("{}", font.attributes());

	// Iterate through the localized strings
	for string in font.localized_strings() {
		// Print the string identifier and the actual value
		println!("[{:?}] {}", string.id(), string);
	}

	for feature in font.features() {
		log::info!("Feature: {:?}", feature.action());
	}

	Some(())
}
