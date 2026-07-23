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

	/// 次のクリップ ID を払い出す
	pub fn alloc_clip_id(&mut self) -> usize {
		let id = self.next_clip_id;
		self.next_clip_id = self.next_clip_id.saturating_add(1);
		id
	}

	/// 指定数の Video トラックを確保する
	pub fn ensure_track_count(&mut self, count: usize) {
		while self.tracks.len() < count {
			let index = self.tracks.len() + 1;
			self.tracks.push(TimelineTrack::new(&format!("Video {index}")));
		}
	}

	/// トラックのミュート状態を切り替える
	pub fn toggle_mute(&mut self, track_index: usize) -> bool {
		let Some(track) = self.tracks.get_mut(track_index) else {
			return false;
		};
		track.muted = !track.muted;
		true
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::TimelineClip;

	#[test]
	fn alloc_clip_id_increments() {
		let mut model = TimelineModel::new();
		assert_eq!(model.alloc_clip_id(), 0);
		assert_eq!(model.alloc_clip_id(), 1);
	}

	#[test]
	fn ensure_track_count_creates_named_tracks() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(2);
		assert_eq!(model.tracks.len(), 2);
		assert_eq!(model.tracks[0].name, "Video 1");
		assert_eq!(model.tracks[1].name, "Video 2");
	}

	#[test]
	fn track_overlaps_detects_interval() {
		let mut track = TimelineTrack::new("Video 1");
		track.add_clip(TimelineClip::new(0, "A", 1.0, 2.0));
		assert!(track.overlaps(2.0, 1.0, None));
		assert!(!track.overlaps(3.0, 1.0, None));
		assert!(!track.overlaps(2.0, 1.0, Some(0)));
	}
}
