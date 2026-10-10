use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use core::Image;

pub(crate) fn hash_color_size(color: [f32; 4], width: u32, height: u32) -> u64 {
	let mut hasher = DefaultHasher::new();
	1u8.hash(&mut hasher);
	for channel in color {
		channel.to_bits().hash(&mut hasher);
	}
	width.hash(&mut hasher);
	height.hash(&mut hasher);
	hasher.finish()
}

pub(crate) fn hash_image(image: &Image) -> u64 {
	let mut hasher = DefaultHasher::new();
	hash_image_into(image, &mut hasher);
	hasher.finish()
}

pub(crate) fn hash_image_pair(a: &Image, b: &Image) -> u64 {
	let mut hasher = DefaultHasher::new();
	hash_image_into(a, &mut hasher);
	hash_image_into(b, &mut hasher);
	hasher.finish()
}

fn hash_image_into(image: &Image, hasher: &mut DefaultHasher) {
	image.width.hash(hasher);
	image.height.hash(hasher);
	for value in &image.rgba {
		value.to_bits().hash(hasher);
	}
}
