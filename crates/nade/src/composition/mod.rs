pub struct FrameIndex(u64);

impl FrameIndex {
	pub fn new(index: u64) -> Self {
		FrameIndex(index)
	}
}

impl From<FrameIndex> for u64 {
	fn from(frame_index: FrameIndex) -> Self {
		frame_index.0
	}
}

pub struct Composition {
	width: u32,
	height: u32,
	duration: u64,
}
