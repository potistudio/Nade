pub mod ffmpeg_encoder;

use crate::core::FrameBuffer;
use anyhow::Result;

pub trait Encoder {
	fn prepare(&mut self) -> Result<()>;
	fn encode_frame(&mut self, frame: &FrameBuffer, index: u32) -> Result<()>;
	fn finish(&mut self) -> Result<()>;
}
