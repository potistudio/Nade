use super::TimelineClip;

/// タイムライントラック（レイヤー）
#[derive(Clone, Debug)]
pub struct TimelineTrack {
	pub name: String,
	pub clips: Vec<TimelineClip>,
	pub muted: bool,
}

impl TimelineTrack {
	pub fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
			clips: Vec::new(),
			muted: false,
		}
	}

	pub fn add_clip(&mut self, clip: TimelineClip) {
		self.clips.push(clip);
	}

	/// 指定区間と重なるクリップがあるか（`exclude_id` は除外）
	pub fn overlaps(&self, start_time: f32, duration: f32, exclude_id: Option<usize>) -> bool {
		let end = start_time + duration;
		self.clips.iter().any(|clip| {
			if exclude_id == Some(clip.id) {
				return false;
			}
			clip.start_time < end && clip.end_time() > start_time
		})
	}
}
