use swash::{
	CacheKey, Charmap, FontRef,
	scale::ScaleContext,
	zeno::{Command, PathData},
};

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
	pub fn charmap(&self) -> Charmap<'_> {
		self.as_ref().charmap()
	}

	// Create the transient font reference for accessing this crate's
	// functionality.
	pub fn as_ref(&self) -> FontRef<'_> {
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
}

impl GlyphEngine {
	pub fn new(font_path: &str) -> Option<Self> {
		let font_data = Font::from_file(font_path, 0)?;

		Some(Self { font_data })
	}

	#[allow(dead_code)]
	pub fn get_glyph_points(&self, character: char) -> Option<Vec<(f32, f32)>> {
		// Create a scale context and scaler
		let mut context = ScaleContext::new();
		let mut scaler = context.builder(self.font_data.as_ref()).size(128.).hint(true).build();

		// Map the character to a glyph ID
		let charmap = self.font_data.charmap();
		let glyph_id = charmap.map(character as u32);

		// Scale the outline for the glyph
		let outline = scaler.scale_outline(glyph_id)?;
		let points = outline.points();

		// Collect the points into a vector of (x, y) tuples
		let point_vec: Vec<(f32, f32)> = points.iter().map(|p| (p.x, p.y)).collect();

		Some(point_vec)
	}

	#[allow(dead_code)]
	pub fn get_bounds(&self, character: char) -> Option<swash::zeno::Bounds> {
		// Create a scale context and scaler
		let mut context = ScaleContext::new();
		let mut scaler = context.builder(self.font_data.as_ref()).size(128.).hint(true).build();

		// Map the character to a glyph ID
		let charmap = self.font_data.charmap();
		let glyph_id = charmap.map(character as u32);

		// Get the bounds for the glyph
		let outline = scaler.scale_outline(glyph_id)?;
		let bounds = outline.bounds();

		Some(bounds)
	}

	pub fn get_path(&self, character: char) -> Option<Vec<Command>> {
		// Create a scale context and scaler
		let mut context = ScaleContext::new();
		let mut scaler = context.builder(self.font_data.as_ref()).size(128.).hint(true).build();

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
