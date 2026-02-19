use super::TimelineClip;

/// タイムラインレイヤー
#[derive(Clone, Debug)]
pub(super) struct TimelineLayer {
	pub name: String,
	pub clips: Vec<TimelineClip>,
	pub muted: bool,
}

impl TimelineLayer {
	pub(super) fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
			clips: Vec::new(),
			muted: false,
		}
	}

	pub(super) fn add_clip(&mut self, clip: TimelineClip) {
		self.clips.push(clip);
	}
}
