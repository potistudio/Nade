use super::*;
use anyhow::Result;
use nade_core::FrameBuffer;
use std::{
	io::Write,
	process::{Command, Stdio},
};

pub struct FfmpegEncoder {
	ffmpeg: std::process::Child,
	stdin: Option<std::process::ChildStdin>,
}

impl Default for FfmpegEncoder {
	fn default() -> Self {
		let mut ffmpeg = Command::new("ffmpeg")
			.args([
				"-f",
				"rawvideo",
				"-pixel_format",
				"rgba",
				"-video_size",
				&format!("{}x{}", 1920, 1080),
				"-framerate",
				"60",
				"-i",
				"-",
				"-c:v",
				"libx264",
				"-preset",
				"fast",
				"-crf",
				"20",
				"-loglevel",
				"quiet",
				"render.mp4",
			])
			.stdin(Stdio::piped())
			.spawn()
			.unwrap();

		let stdin = ffmpeg.stdin.take().unwrap();

		Self {
			ffmpeg,
			stdin: Some(stdin),
		}
	}
}

impl Encoder for FfmpegEncoder {
	fn prepare(&mut self) -> Result<()> {
		log::debug!("Preparing FFmpeg encoder...");

		log::debug!("Prepared FFmpeg encoder.");
		Ok(())
	}

	fn encode_frame(&mut self, frame: &FrameBuffer, index: u32) -> Result<()> {
		log::debug!("Encoding frame [{}]...", index);
		if let Some(stdin) = self.stdin.as_mut() {
			stdin.write_all(&frame.data)?;
		}

		Ok(())
	}

	fn finish(&mut self) -> Result<()> {
		log::debug!("Finishing FFmpeg encoder...");

		drop(self.stdin.take());
		let status = self.ffmpeg.wait()?;

		log::debug!("Finished FFmpeg with status: {}", status);
		Ok(())
	}
}
