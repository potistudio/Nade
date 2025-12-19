pub mod ffmpeg_encoder;

use anyhow::Result;
use nade_core::FrameBuffer;

pub trait Encoder {
	fn prepare(&mut self) -> Result<()>;
	fn encode_frame(&mut self, frame: &FrameBuffer, index: u32) -> Result<()>;
	fn finish(&mut self) -> Result<()>;
}
