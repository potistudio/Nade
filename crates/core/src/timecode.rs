//! Frame-accurate time formatting shared by the viewer, timeline and status bar.

/// Clamp a frame rate to something usable for frame math.
fn sanitize_fps(fps: f32) -> f32 {
	if fps.is_finite() && fps >= 1.0 { fps } else { 30.0 }
}

/// Frame index at `seconds` (floored, never negative).
pub fn frame_at(seconds: f32, fps: f32) -> u32 {
	let fps = sanitize_fps(fps);
	// 浮動小数の誤差で 1 フレーム手前に落ちないよう僅かに持ち上げる
	(seconds.max(0.0) * fps + 1e-3).floor() as u32
}

/// Time in seconds at the start of `frame`.
pub fn time_of_frame(frame: u32, fps: f32) -> f32 {
	frame as f32 / sanitize_fps(fps)
}

/// Snap `seconds` to the start of its frame.
pub fn snap_to_frame(seconds: f32, fps: f32) -> f32 {
	time_of_frame(frame_at(seconds, fps), fps)
}

/// Non-drop timecode `HH:MM:SS:FF`.
pub fn format(seconds: f32, fps: f32) -> String {
	let fps = sanitize_fps(fps);
	let whole_fps = fps.round().max(1.0) as u32;
	let frame = frame_at(seconds, fps);
	let total_seconds = frame / whole_fps;
	let frames = frame % whole_fps;
	let hours = total_seconds / 3600;
	let minutes = (total_seconds / 60) % 60;
	let secs = total_seconds % 60;
	format!("{hours:02}:{minutes:02}:{secs:02}:{frames:02}")
}

/// Compact ruler label: `S:FF` under a minute, `M:SS` beyond.
pub fn format_short(seconds: f32, fps: f32) -> String {
	let fps = sanitize_fps(fps);
	let whole_fps = fps.round().max(1.0) as u32;
	let frame = frame_at(seconds, fps);
	let total_seconds = frame / whole_fps;
	if total_seconds >= 60 {
		format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
	} else {
		format!("{}:{:02}", total_seconds, frame % whole_fps)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn formats_hours_minutes_seconds_frames() {
		assert_eq!(format(0.0, 60.0), "00:00:00:00");
		assert_eq!(format(1.5, 60.0), "00:00:01:30");
		assert_eq!(format(3661.0 + 2.0 / 30.0, 30.0), "01:01:01:02");
	}

	#[test]
	fn frame_math_round_trips() {
		for frame in [0, 1, 59, 60, 61, 599] {
			assert_eq!(frame_at(time_of_frame(frame, 60.0), 60.0), frame);
		}
		assert_eq!(frame_at(-1.0, 60.0), 0);
	}

	#[test]
	fn short_labels_switch_to_minutes() {
		assert_eq!(format_short(2.5, 30.0), "2:15");
		assert_eq!(format_short(75.0, 30.0), "1:15");
	}

	#[test]
	fn invalid_fps_falls_back() {
		assert_eq!(format(1.0, 0.0), "00:00:01:00");
	}
}
