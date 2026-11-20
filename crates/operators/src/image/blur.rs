use nade_core::Image;

pub(crate) fn blur_1d(image: &Image, radius: f32) -> Image {
	let width = image.width as usize;
	let height = image.height as usize;
	if width == 0 || height == 0 {
		return image.clone();
	}

	#[allow(clippy::cast_possible_truncation)]
	let radius = radius.max(0.0).round() as i32;
	if radius <= 0 {
		return image.clone();
	}

	let mut rgba = vec![0.0; width * height * 4];
	let max_x = width as i32 - 1;
	for y in 0..height {
		for x in 0..width {
			let mut acc = [0.0; 4];
			let mut count = 0.0f32;
			for dx in -radius..=radius {
				let nx = (x as i32 + dx).clamp(0, max_x) as usize;
				let idx = (y * width + nx) * 4;
				acc[0] += image.rgba[idx];
				acc[1] += image.rgba[idx + 1];
				acc[2] += image.rgba[idx + 2];
				acc[3] += image.rgba[idx + 3];
				count += 1.0;
			}
			let out_idx = (y * width + x) * 4;
			rgba[out_idx] = acc[0] / count;
			rgba[out_idx + 1] = acc[1] / count;
			rgba[out_idx + 2] = acc[2] / count;
			rgba[out_idx + 3] = acc[3] / count;
		}
	}

	Image {
		width: image.width,
		height: image.height,
		rgba,
	}
}
