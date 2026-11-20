//! Locating the FFmpeg executables.
//!
//! Apps launched from Finder do not inherit the shell `PATH`, so Homebrew and
//! MacPorts locations are checked as well. `NADE_FFMPEG` / `NADE_FFPROBE`
//! override the lookup.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const FALLBACK_DIRS: [&str; 4] = ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin", "/usr/bin"];

fn locate(name: &str, override_var: &str) -> PathBuf {
	if let Some(path) = std::env::var_os(override_var) {
		return PathBuf::from(path);
	}
	let file = if cfg!(windows) {
		format!("{name}.exe")
	} else {
		name.to_string()
	};
	let on_path = std::env::var_os("PATH")
		.map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
		.unwrap_or_default();
	on_path
		.into_iter()
		.chain(FALLBACK_DIRS.iter().map(PathBuf::from))
		.map(|dir| dir.join(&file))
		.find(|candidate| is_executable(candidate))
		.unwrap_or_else(|| PathBuf::from(file))
}

fn is_executable(path: &Path) -> bool {
	path.is_file()
}

/// Path of the `ffmpeg` executable.
pub fn ffmpeg_path() -> &'static Path {
	static PATH: OnceLock<PathBuf> = OnceLock::new();
	PATH.get_or_init(|| locate("ffmpeg", "NADE_FFMPEG"))
}

/// Path of the `ffprobe` executable.
pub fn ffprobe_path() -> &'static Path {
	static PATH: OnceLock<PathBuf> = OnceLock::new();
	PATH.get_or_init(|| locate("ffprobe", "NADE_FFPROBE"))
}
