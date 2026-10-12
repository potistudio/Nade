use iced::{Rectangle, mouse};
use std::time::{Duration, Instant};
use timeline_panel::{TimelineCanvasEvent, TimelineInteraction, TimelineMessage, TimelineModel};

fn scroll(state: &mut TimelineInteraction, x: f32, y: f32) {
	state.apply_message(
		&mut TimelineModel::new(),
		TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseWheelScrolled {
			delta: mouse::ScrollDelta::Pixels { x, y },
			bounds: Rectangle::new(iced::Point::ORIGIN, iced::Size::new(800.0, 400.0)),
		}),
		0.0,
	);
}

#[test]
fn overscroll_is_animated_and_returns_to_the_edge() {
	for direction in [1.0, -1.0] {
		let mut state = TimelineInteraction::default();
		scroll(&mut state, 0.0, direction * 100.0);
		assert_eq!(state.scroll_offset.y, 0.0);
		assert!(state.is_scroll_animating());
		let start = Instant::now();
		state.tick_scroll_spring(start + Duration::from_millis(16));
		assert!(state.scroll_offset.y * direction > 0.0);
		assert!(state.scroll_offset.y.abs() < 80.0);
		for tick in 2..=240 {
			state.tick_scroll_spring(start + Duration::from_millis(tick * 16));
		}
		assert_eq!(state.scroll_offset.y, 0.0);
		assert!(!state.is_scroll_animating());
	}
}

#[test]
fn momentum_events_do_not_write_the_displayed_position() {
	let mut state = TimelineInteraction::default();
	scroll(&mut state, 0.0, 20.0);
	let start = Instant::now();
	for tick in 1..=100 {
		state.tick_scroll_spring(start + Duration::from_millis(tick * 16));
		let before = state.scroll_offset.y;
		scroll(&mut state, 0.0, 10.0 / tick as f32);
		assert_eq!(state.scroll_offset.y, before);
		assert!(state.scroll_offset.y.abs() < 80.0);
	}
	for tick in 101..=340 {
		state.tick_scroll_spring(start + Duration::from_millis(tick * 16));
	}
	assert_eq!(state.scroll_offset.y, 0.0);
}

#[test]
fn repeated_input_stays_bounded_without_clamping_the_position() {
	let mut state = TimelineInteraction::default();
	for _ in 0..100_000 {
		scroll(&mut state, 0.0, 10.0);
	}
	assert_eq!(state.scroll_offset.y, 0.0);
	let start = Instant::now();
	for tick in 1..=240 {
		state.tick_scroll_spring(start + Duration::from_millis(tick * 16));
		assert!(state.scroll_offset.y.abs() < 80.0);
	}
	assert_eq!(state.scroll_offset.y, 0.0);
}

#[test]
fn direct_gesture_has_resistance_but_no_spring_until_release() {
	let mut state = TimelineInteraction::default();
	state.set_scroll_gesture_active(true);
	scroll(&mut state, 0.0, 100.0);
	let held_position = state.scroll_offset.y;
	assert!(held_position > 0.0 && held_position < 80.0);
	assert!(!state.is_scroll_animating());
	state.tick_scroll_spring(Instant::now() + Duration::from_millis(32));
	assert_eq!(state.scroll_offset.y, held_position);
	state.set_scroll_gesture_active(false);
	assert!(state.is_scroll_animating());
	let released = Instant::now();
	state.tick_scroll_spring(released + Duration::from_millis(16));
	assert!(state.scroll_offset.y < held_position);
	for tick in 2..=240 {
		state.tick_scroll_spring(released + Duration::from_millis(tick * 16));
	}
	assert_eq!(state.scroll_offset.y, 0.0);
}

#[test]
fn momentum_is_ignored_only_after_overscroll_release() {
	let mut state = TimelineInteraction::default();
	state.set_scroll_gesture_active(true);
	scroll(&mut state, 0.0, 40.0);
	state.set_scroll_gesture_active(false);
	state.set_scroll_input_is_momentum(true);
	let before = state.scroll_offset.y;
	scroll(&mut state, -20.0, 20.0);
	assert_eq!(state.scroll_offset.y, before);
	assert_eq!(state.scroll_offset.x, -20.0);
	let released = Instant::now();
	for tick in 1..=240 {
		state.tick_scroll_spring(released + Duration::from_millis(tick * 16));
	}
	assert_eq!(state.scroll_offset.y, 0.0);
	// No latched cancellation: momentum is accepted again once inside bounds.
	scroll(&mut state, -10.0, 0.0);
	assert_eq!(state.scroll_offset.x, -30.0);
	state.set_scroll_input_is_momentum(false);
	state.set_scroll_gesture_active(true);
	scroll(&mut state, 0.0, 20.0);
	assert!(state.scroll_offset.y > 0.0);

	let mut ordinary = TimelineInteraction::default();
	ordinary.set_scroll_input_is_momentum(true);
	scroll(&mut ordinary, -20.0, 0.0);
	assert_eq!(ordinary.scroll_offset.x, -20.0);
}

#[test]
fn scrolling_inside_the_bounds_is_immediate() {
	let mut state = TimelineInteraction::default();
	scroll(&mut state, -100.0, 0.0);
	assert_eq!(state.scroll_offset.x, -100.0);
	assert!(!state.is_scroll_animating());
	state.tick_scroll_spring(Instant::now() + Duration::from_secs(1));
	assert_eq!(state.scroll_offset.x, -100.0);
}
