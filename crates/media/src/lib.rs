//! # Media
//!
//! Probing and decoding of footage for the renderer.
//!
//! Stills are decoded with the `image` crate. Moving pictures and sound go
//! through the FFmpeg command line tools (`ffprobe` / `ffmpeg`), which are
//! looked up on `PATH` and in the usual install locations.

mod cache;
mod probe;
mod tools;
mod video;

pub use cache::{MediaCache, mip_level};
pub use probe::{is_still_image, probe};
pub use tools::{ffmpeg_path, ffprobe_path};
pub use video::VideoDecoder;

/// Decoded picture (premultiplied RGBA for stills, opaque RGBA for video).
pub type Picture = image::RgbaImage;
