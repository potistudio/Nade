use anyhow::Result;

mod core;
mod encoder;
mod renderer;

fn main() -> Result<()> {
	unsafe { std::env::set_var("RUST_LOG", "debug") };
	env_logger::init();

	let encoder = encoder::ffmpeg_encoder::FfmpegEncoder::default();
	let mut renderer = renderer::Renderer { encoder };

	renderer.render()?;

	Ok(())
}
