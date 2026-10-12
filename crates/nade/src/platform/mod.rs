pub fn sync_scroll_phase(state: &mut timeline_panel::TimelineInteraction) {
	#[cfg(target_os = "macos")]
	#[allow(unsafe_code)]
	unsafe {
		unsafe extern "C" {
			fn nade_set_timeline_scroll_bounds(x: f64, y: f64, width: f64, height: f64);
			fn nade_scroll_gesture_active() -> bool;
			fn nade_scroll_input_is_momentum() -> bool;
		}
		let bounds = state.scroll_bounds().unwrap_or_default();
		nade_set_timeline_scroll_bounds(
			bounds.x as f64,
			bounds.y as f64,
			bounds.width as f64,
			bounds.height as f64,
		);
		state.set_scroll_gesture_active(nade_scroll_gesture_active());
		state.set_scroll_input_is_momentum(nade_scroll_input_is_momentum());
	}
	#[cfg(not(target_os = "macos"))]
	let _ = state;
}
