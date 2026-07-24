use super::TimelineTrack;

/// タイムラインのドメインモデル
#[derive(Debug, Default, Clone)]
pub struct TimelineModel {
	pub tracks: Vec<TimelineTrack>,
	next_clip_id: usize,
	/// ソロ開始前の可視状態。全ソロ解除時に復元する。
	solo_visibility_backup: Option<Vec<bool>>,
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

	/// 可視フラグを切り替える。
	///
	/// - 非表示にする場合はソロも解除する
	/// - ソロ中に非ソロの不可視トラックを可視にすると、そのトラックもソロに入る
	pub fn toggle_visible(&mut self, track_index: usize) -> bool {
		if track_index >= self.tracks.len() {
			return false;
		}

		let currently_visible = self.tracks[track_index].visible;
		if currently_visible {
			let ending_solo_session =
				self.tracks[track_index].solo && self.tracks.iter().filter(|track| track.solo).count() == 1;
			self.tracks[track_index].visible = false;
			self.tracks[track_index].solo = false;
			if self.any_solo() {
				self.sync_visibility_to_solo();
			} else if ending_solo_session || self.solo_visibility_backup.is_some() {
				self.restore_solo_visibility();
				// ユーザーが明示的に隠したトラックは復元後も非表示のまま
				self.tracks[track_index].visible = false;
				self.tracks[track_index].solo = false;
			}
		} else if self.any_solo() {
			// ソロ除外中に「見せる」→ ソロ群に参加（状態と表示を一致させる）
			self.tracks[track_index].visible = true;
			self.tracks[track_index].solo = true;
			self.sync_visibility_to_solo();
		} else {
			self.tracks[track_index].visible = true;
		}
		true
	}

	/// ソロフラグを切り替える。不可視トラックはソロにできない。
	///
	/// ソロ中は非ソロ・トラックの `visible` を false にし、
	/// 全ソロ解除時にソロ開始前の可視状態へ戻す。
	pub fn toggle_solo(&mut self, track_index: usize) -> bool {
		if track_index >= self.tracks.len() {
			return false;
		}

		if self.tracks[track_index].solo {
			self.tracks[track_index].solo = false;
			if self.any_solo() {
				self.sync_visibility_to_solo();
			} else {
				self.restore_solo_visibility();
			}
			return true;
		}

		if !self.tracks[track_index].visible {
			return false;
		}

		if self.solo_visibility_backup.is_none() {
			self.solo_visibility_backup = Some(self.tracks.iter().map(|track| track.visible).collect());
		}
		self.tracks[track_index].solo = true;
		self.sync_visibility_to_solo();
		true
	}

	/// いずれかのトラックがソロか
	pub fn any_solo(&self) -> bool {
		self.tracks.iter().any(|track| track.solo)
	}

	/// プレビューに描画すべきトラックか（`visible` と一致）
	pub fn is_track_rendered(&self, track_index: usize) -> bool {
		self.tracks.get(track_index).is_some_and(|track| track.visible)
	}

	fn sync_visibility_to_solo(&mut self) {
		for track in &mut self.tracks {
			track.visible = track.solo;
		}
	}

	fn restore_solo_visibility(&mut self) {
		if let Some(backup) = self.solo_visibility_backup.take() {
			for (track, visible) in self.tracks.iter_mut().zip(backup) {
				track.visible = visible;
				track.solo = false;
			}
		} else {
			for track in &mut self.tracks {
				track.solo = false;
			}
		}
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
	fn solo_turns_off_other_visibility_state() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(2);
		assert!(model.tracks[0].visible);
		assert!(model.tracks[1].visible);

		assert!(model.toggle_solo(1));
		assert!(!model.tracks[0].visible);
		assert!(!model.tracks[0].solo);
		assert!(model.tracks[1].visible);
		assert!(model.tracks[1].solo);
		assert!(!model.is_track_rendered(0));
		assert!(model.is_track_rendered(1));
	}

	#[test]
	fn ending_solo_restores_visibility() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(2);
		assert!(model.toggle_visible(0)); // hide track 0 first
		assert!(!model.tracks[0].visible);

		assert!(model.toggle_solo(1));
		assert!(!model.tracks[0].visible);
		assert!(model.tracks[1].visible);

		assert!(model.toggle_solo(1)); // end solo
		assert!(!model.tracks[0].visible); // restored hidden
		assert!(model.tracks[1].visible);
		assert!(!model.tracks[1].solo);
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
	fn hiding_solo_track_ends_solo_and_restores() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(2);
		assert!(model.toggle_solo(1));
		assert!(!model.tracks[0].visible);

		assert!(model.toggle_visible(1)); // hide the solo track
		assert!(!model.tracks[1].solo);
		assert!(model.tracks[0].visible); // restored
	}

	#[test]
	fn showing_during_solo_joins_solo_group() {
		let mut model = TimelineModel::new();
		model.ensure_track_count(2);
		assert!(model.toggle_solo(1));
		assert!(!model.tracks[0].visible);

		assert!(model.toggle_visible(0));
		assert!(model.tracks[0].visible);
		assert!(model.tracks[0].solo);
		assert!(model.tracks[1].solo);
	}
}
