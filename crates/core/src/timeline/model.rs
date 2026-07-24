use super::TimelineTrack;

/// トラックの可視 UI 状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackVisibilityDisplay {
	/// 非表示
	Hidden,
	/// 表示中
	Shown,
	/// 可視フラグは ON だがソロ除外で描画されない
	SoloExcluded,
}

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

	/// 可視フラグを切り替える。非表示にする場合はソロも解除する。
	pub fn toggle_visible(&mut self, track_index: usize) -> bool {
		let Some(track) = self.tracks.get_mut(track_index) else {
			return false;
		};
		track.visible = !track.visible;
		if !track.visible {
			track.solo = false;
		}
		true
	}

	/// ソロフラグを切り替える。不可視トラックはソロにできない。
	pub fn toggle_solo(&mut self, track_index: usize) -> bool {
		let Some(track) = self.tracks.get_mut(track_index) else {
			return false;
		};
		if track.solo {
			track.solo = false;
			return true;
		}
		if !track.visible {
			return false;
		}
		track.solo = true;
		true
	}

	/// いずれかのトラックがソロか
	pub fn any_solo(&self) -> bool {
		self.tracks.iter().any(|track| track.solo)
	}

	/// プレビューに描画すべきトラックか
	pub fn is_track_rendered(&self, track_index: usize) -> bool {
		let Some(track) = self.tracks.get(track_index) else {
			return false;
		};
		if !track.visible {
			return false;
		}
		if self.any_solo() {
			return track.solo;
		}
		true
	}

	/// V ボタン用の表示状態（実フラグを壊さず第3状態を返す）
	pub fn visibility_display(&self, track_index: usize) -> TrackVisibilityDisplay {
		let Some(track) = self.tracks.get(track_index) else {
			return TrackVisibilityDisplay::Hidden;
		};
		if !track.visible {
			return TrackVisibilityDisplay::Hidden;
		}
		if self.any_solo() && !track.solo {
			return TrackVisibilityDisplay::SoloExcluded;
		}
		TrackVisibilityDisplay::Shown
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

	#[test]
	fn solo_excludes_without_changing_visible_flag() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(2);
		assert!(model.toggle_solo(1));

		assert!(model.tracks[0].visible);
		assert!(!model.tracks[0].solo);
		assert!(!model.is_track_rendered(0));
		assert_eq!(model.visibility_display(0), TrackVisibilityDisplay::SoloExcluded);
		assert_eq!(model.visibility_display(1), TrackVisibilityDisplay::Shown);
	}

	#[test]
	fn ending_solo_keeps_visibility_flags() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(2);
		assert!(model.toggle_visible(0));
		assert!(model.toggle_solo(1));
		assert!(model.toggle_solo(1));

		assert!(!model.tracks[0].visible);
		assert!(model.tracks[1].visible);
		assert!(!model.any_solo());
	}

	#[test]
	fn cannot_solo_invisible_track() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(1);
		assert!(model.toggle_visible(0));
		assert!(!model.toggle_solo(0));
		assert!(!model.tracks[0].solo);
	}

	#[test]
	fn hiding_clears_solo() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(1);
		assert!(model.toggle_solo(0));
		assert!(model.toggle_visible(0));
		assert!(!model.tracks[0].visible);
		assert!(!model.tracks[0].solo);
	}
}
