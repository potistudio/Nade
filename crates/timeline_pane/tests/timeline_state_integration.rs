use iced::Color;
use nade_core::{Composition, RectangleObject};
use timeline_pane::{TimelineClip, TimelineState, TimelineTrack};

fn approx_eq_f32(a: f32, b: f32) {
	assert!((a - b).abs() < 1e-5, "left: {a}, right: {b}");
}

#[test]
fn sync_with_composition_builds_per_object_tracks() {
	let mut composition = Composition::new();
	let id_a = composition.add_rectangle(
		RectangleObject::new("Title")
			.with_start_time(0.5)
			.with_duration(1.5)
			.with_fill_color(0.2, 0.4, 0.8, 0.9),
	);
	let id_b = composition.add_rectangle(
		RectangleObject::new("Subtitle")
			.with_start_time(3.0)
			.with_duration(4.0),
	);

	let mut timeline = TimelineState::new();
	timeline.sync_with_composition(&composition);

	assert_eq!(timeline.tracks.len(), 2);
	assert_eq!(timeline.tracks[0].clips.len(), 1);
	assert_eq!(timeline.tracks[1].clips.len(), 1);

	let clip_a = timeline
		.tracks
		.iter()
		.flat_map(|track| track.clips.iter())
		.find(|clip| clip.scene_object_id == Some(id_a))
		.expect("clip for Title should exist");
	assert_eq!(clip_a.name, "Title");
	approx_eq_f32(clip_a.start_time, 0.5);
	approx_eq_f32(clip_a.duration, 1.5);

	let clip_b = timeline
		.tracks
		.iter()
		.flat_map(|track| track.clips.iter())
		.find(|clip| clip.scene_object_id == Some(id_b))
		.expect("clip for Subtitle should exist");
	assert_eq!(clip_b.name, "Subtitle");
	approx_eq_f32(clip_b.start_time, 3.0);
	approx_eq_f32(clip_b.duration, 4.0);
}

#[test]
fn apply_clip_changes_to_composition_updates_matching_objects() {
	let mut composition = Composition::new();
	let id_a = composition.add_rectangle(
		RectangleObject::new("A")
			.with_start_time(1.0)
			.with_duration(2.0),
	);
	let id_b = composition.add_rectangle(
		RectangleObject::new("B")
			.with_start_time(4.0)
			.with_duration(1.0),
	);

	let mut timeline = TimelineState::new();
	timeline.sync_with_composition(&composition);

	let clip_a = timeline
		.tracks
		.iter_mut()
		.flat_map(|track| track.clips.iter_mut())
		.find(|clip| clip.scene_object_id == Some(id_a))
		.expect("clip for A should exist");
	clip_a.start_time = 8.0;
	clip_a.duration = 3.25;

	let clip_b = timeline
		.tracks
		.iter_mut()
		.flat_map(|track| track.clips.iter_mut())
		.find(|clip| clip.scene_object_id == Some(id_b))
		.expect("clip for B should exist");
	clip_b.start_time = 0.0;
	clip_b.duration = 10.0;

	timeline.apply_clip_changes_to_composition(&mut composition);

	let obj_a = composition.get(id_a).expect("object A should still exist");
	let obj_b = composition.get(id_b).expect("object B should still exist");

	approx_eq_f32(obj_a.start_time(), 8.0);
	approx_eq_f32(obj_a.duration(), 3.25);
	approx_eq_f32(obj_b.start_time(), 0.0);
	approx_eq_f32(obj_b.duration(), 10.0);
}

#[test]
fn scroll_and_zoom_public_api_behaves_consistently() {
	let mut timeline = TimelineState::new();
	timeline.update_viewport_width(1000.0);

	timeline.set_scroll_position(100.0, 80.0);
	assert_eq!(timeline.scroll_offset.x, 0.0);
	assert_eq!(timeline.scroll_offset.y, 0.0);

	timeline.apply_scroll_delta(-40.0, -20.0);
	assert_eq!(timeline.scroll_offset.x, -40.0);
	assert_eq!(timeline.scroll_offset.y, -20.0);

	timeline.zoom_in(2.0);
	assert!(timeline.time_scale > 1.0);
	assert!(timeline.scroll_offset.x <= 0.0);

	timeline.zoom_out(2.0);
	assert!(timeline.time_scale <= 1.0);
	assert!(timeline.scroll_offset.x <= 0.0);

	timeline.reset_zoom();
	approx_eq_f32(timeline.time_scale, 1.0);
}

#[test]
fn manual_tracks_and_ids_allow_round_trip_data_modeling() {
	let mut timeline = TimelineState::new();

	let clip_id = timeline.next_clip_id();
	let clip = TimelineClip::new(clip_id, "Manual", 2.0, 5.0, Color::from_rgb(1.0, 0.0, 0.0));

	let mut track = TimelineTrack::new("Track 1");
	track.add_clip(clip);
	timeline.add_track(track);

	assert_eq!(timeline.tracks.len(), 1);
	assert_eq!(timeline.tracks[0].clips.len(), 1);
	assert_eq!(timeline.tracks[0].clips[0].id, clip_id);
	approx_eq_f32(timeline.tracks[0].clips[0].end_time(), 7.0);
}
