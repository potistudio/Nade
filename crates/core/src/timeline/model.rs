use super::TimelineTrack;

/// タイムラインのドメインモデル
#[derive(Debug, Default, Clone)]
pub struct TimelineModel {
	pub tracks: Vec<TimelineTrack>,
	next_clip_id: usize,
}

impl TimelineModel {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_track(&mut self, track: TimelineTrack) {
		self.tracks.push(track);
	}
}
