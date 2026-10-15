use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestPanel;

impl AreaKind for TestPanel {
	fn all() -> Vec<Self> {
		vec![Self]
	}

	fn icon(&self) -> svg::Handle {
		svg::Handle::from_memory(Vec::<u8>::new())
	}

	fn label(&self) -> &'static str {
		"Test"
	}
}

fn leaf(id: usize) -> DockNode<TestPanel> {
	DockNode::Leaf(Area::new(id, TestPanel))
}

fn split(
	direction: SplitDirection,
	first_size: f32,
	first: DockNode<TestPanel>,
	second: DockNode<TestPanel>,
) -> DockNode<TestPanel> {
	DockNode::Split {
		direction,
		first_size,
		first: Box::new(first),
		second: Box::new(second),
	}
}

fn system(root: DockNode<TestPanel>) -> PanelSystem<TestPanel> {
	let mut system = PanelSystem::new().with_layout(root);
	system.window_size = Size::new(1000.0, 1000.0);
	system
}

fn axes() -> [SplitDirection; 2] {
	[SplitDirection::Horizontal, SplitDirection::Vertical]
}

fn extent(bounds: Rectangle, axis: SplitDirection) -> f32 {
	match axis {
		SplitDirection::Horizontal => bounds.width,
		SplitDirection::Vertical => bounds.height,
	}
}

fn bounds(system: &PanelSystem<TestPanel>, id: usize) -> Rectangle {
	system.bounds_at_path(&system.area_path(id).unwrap())
}

fn start_drag(system: &mut PanelSystem<TestPanel>, path: &[usize], axis: SplitDirection) {
	system.update::<()>(PanelSystemMessage::MouseMove(Point::ORIGIN));
	system.update::<()>(PanelSystemMessage::ResizeStart(path.to_vec(), axis));
}

fn drag_to(system: &mut PanelSystem<TestPanel>, axis: SplitDirection, delta: f32) {
	let position = match axis {
		SplitDirection::Horizontal => Point::new(delta, 0.0),
		SplitDirection::Vertical => Point::new(0.0, delta),
	};
	system.update::<()>(PanelSystemMessage::MouseMove(position));
}

fn assert_close(actual: f32, expected: f32) {
	assert!((actual - expected).abs() < 0.001, "expected {expected}, got {actual}");
}

fn assert_bounds(actual: Rectangle, expected: Rectangle) {
	assert_close(actual.x, expected.x);
	assert_close(actual.y, expected.y);
	assert_close(actual.width, expected.width);
	assert_close(actual.height, expected.height);
}

#[test]
fn resizing_preserves_dividers_in_the_second_branch() {
	for axis in axes() {
		let mut system = system(split(axis, 250.0, leaf(0), split(axis, 300.0, leaf(1), leaf(2))));
		let far = bounds(&system, 2);
		start_drag(&mut system, &[], axis);
		drag_to(&mut system, axis, 50.0);
		assert_close(extent(bounds(&system, 0), axis), 300.0);
		assert_close(extent(bounds(&system, 1), axis), 250.0);
		assert_bounds(bounds(&system, 2), far);
		drag_to(&mut system, axis, -50.0);
		assert_close(extent(bounds(&system, 0), axis), 200.0);
		assert_close(extent(bounds(&system, 1), axis), 350.0);
		assert_bounds(bounds(&system, 2), far);
	}
}

#[test]
fn resizing_preserves_dividers_in_the_first_branch() {
	for axis in axes() {
		let mut system = system(split(axis, 600.0, split(axis, 200.0, leaf(0), leaf(1)), leaf(2)));
		let far = bounds(&system, 0);
		let adjacent_size = extent(bounds(&system, 1), axis);
		start_drag(&mut system, &[], axis);
		drag_to(&mut system, axis, -50.0);
		assert_bounds(bounds(&system, 0), far);
		assert_close(extent(bounds(&system, 1), axis), adjacent_size - 50.0);
	}
}

#[test]
fn perpendicular_branches_preserve_their_dividers_and_share_the_tightest_limit() {
	for axis in axes() {
		let cross_axis = if axis == SplitDirection::Horizontal {
			SplitDirection::Vertical
		} else {
			SplitDirection::Horizontal
		};
		let mut system = system(split(
			axis,
			200.0,
			leaf(0),
			split(
				cross_axis,
				400.0,
				split(axis, 300.0, leaf(1), leaf(2)),
				split(axis, 200.0, leaf(3), leaf(4)),
			),
		));
		let far_first = bounds(&system, 2);
		let far_second = bounds(&system, 4);
		start_drag(&mut system, &[], axis);
		drag_to(&mut system, axis, 10000.0);
		assert_close(extent(bounds(&system, 0), axis), 300.0);
		assert_close(extent(bounds(&system, 1), axis), 200.0);
		assert_close(extent(bounds(&system, 3), axis), MIN_PANEL_SIZE);
		assert_bounds(bounds(&system, 2), far_first);
		assert_bounds(bounds(&system, 4), far_second);
	}
}

#[test]
fn resizing_stops_at_adjacent_panel_minimums_without_pushing_other_dividers() {
	for axis in axes() {
		let mut system = system(split(
			axis,
			400.0,
			split(axis, 200.0, leaf(0), leaf(1)),
			split(axis, 150.0, leaf(2), leaf(3)),
		));
		let original: Vec<_> = (0..4).map(|id| bounds(&system, id)).collect();
		start_drag(&mut system, &[], axis);
		drag_to(&mut system, axis, 10000.0);
		assert_close(extent(bounds(&system, 2), axis), MIN_PANEL_SIZE);
		assert_bounds(bounds(&system, 0), original[0]);
		assert_bounds(bounds(&system, 3), original[3]);
		drag_to(&mut system, axis, -10000.0);
		assert_close(extent(bounds(&system, 1), axis), MIN_PANEL_SIZE);
		assert_bounds(bounds(&system, 0), original[0]);
		assert_bounds(bounds(&system, 3), original[3]);
		drag_to(&mut system, axis, 0.0);
		for (id, expected) in original.into_iter().enumerate() {
			assert_bounds(bounds(&system, id), expected);
		}
	}
}

#[test]
fn nested_resize_leaves_ancestors_and_siblings_unchanged() {
	for axis in axes() {
		let mut system = system(split(axis, 250.0, leaf(0), split(axis, 300.0, leaf(1), leaf(2))));
		let sibling = bounds(&system, 0);
		let parent = system.bounds_at_path(&[1]);
		start_drag(&mut system, &[1], axis);
		drag_to(&mut system, axis, 50.0);
		assert_bounds(bounds(&system, 0), sibling);
		assert_bounds(system.bounds_at_path(&[1]), parent);
		assert_close(extent(bounds(&system, 1), axis), 350.0);
	}
}

#[test]
fn resize_materializes_ratio_sized_descendants() {
	for axis in axes() {
		let mut system = system(split(axis, 0.25, leaf(0), split(axis, 0.5, leaf(1), leaf(2))));
		let far = bounds(&system, 2);
		let adjacent = extent(bounds(&system, 0), axis);
		start_drag(&mut system, &[], axis);
		drag_to(&mut system, axis, 40.0);
		assert_close(extent(bounds(&system, 0), axis), adjacent + 40.0);
		assert_bounds(bounds(&system, 2), far);
	}
}

#[test]
fn resize_starts_from_the_visible_clamped_size() {
	for axis in axes() {
		let mut system = system(split(axis, 2000.0, leaf(0), leaf(1)));
		let visible_size = extent(bounds(&system, 0), axis);
		start_drag(&mut system, &[], axis);
		drag_to(&mut system, axis, -25.0);
		assert_close(extent(bounds(&system, 0), axis), visible_size - 25.0);
	}
}

#[test]
fn undersized_splits_do_not_force_fixed_dividers_to_move() {
	for axis in axes() {
		let mut system = system(split(axis, 400.0, leaf(0), split(axis, 0.5, leaf(1), leaf(2))));
		system.window_size = Size::new(606.0, 606.0);
		let original: Vec<_> = (0..3).map(|id| bounds(&system, id)).collect();
		start_drag(&mut system, &[], axis);
		for delta in [-50.0, 50.0] {
			drag_to(&mut system, axis, delta);
			for (id, expected) in original.iter().enumerate() {
				assert_bounds(bounds(&system, id), *expected);
			}
		}
	}
}

#[test]
fn deeply_nested_dividers_remain_fixed() {
	for axis in axes() {
		let mut system = system(split(
			axis,
			200.0,
			leaf(0),
			split(axis, 400.0, split(axis, 150.0, leaf(1), leaf(2)), leaf(3)),
		));
		let far_first = bounds(&system, 2);
		let far_second = bounds(&system, 3);
		start_drag(&mut system, &[], axis);
		drag_to(&mut system, axis, 10000.0);
		assert_close(extent(bounds(&system, 0), axis), 250.0);
		assert_close(extent(bounds(&system, 1), axis), MIN_PANEL_SIZE);
		assert_bounds(bounds(&system, 2), far_first);
		assert_bounds(bounds(&system, 3), far_second);
	}
}

#[test]
fn window_resize_still_keeps_pixel_sized_leading_panels() {
	for axis in axes() {
		let mut system = system(split(axis, 250.0, leaf(0), split(axis, 300.0, leaf(1), leaf(2))));
		let far_size = extent(bounds(&system, 2), axis);
		let size = match axis {
			SplitDirection::Horizontal => Size::new(1100.0, 1000.0),
			SplitDirection::Vertical => Size::new(1000.0, 1100.0),
		};
		system.update::<()>(PanelSystemMessage::WindowResized(size));
		assert_close(extent(bounds(&system, 0), axis), 250.0);
		assert_close(extent(bounds(&system, 1), axis), 300.0);
		assert_close(extent(bounds(&system, 2), axis), far_size + 100.0);
	}
}
