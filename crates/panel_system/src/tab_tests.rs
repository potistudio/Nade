use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
	A,
	B,
	C,
}

impl AreaKind for Kind {
	fn all() -> Vec<Self> {
		vec![Self::A, Self::B, Self::C]
	}

	fn icon(&self) -> svg::Handle {
		svg::Handle::from_memory(Vec::<u8>::new())
	}

	fn label(&self) -> &'static str {
		"Kind"
	}
}

fn tabs(id: usize, tabs: Vec<Kind>, active: usize) -> DockNode<Kind> {
	DockNode::Leaf(Area::with_tabs(id, tabs, active))
}

/// Area 0 on the left, area 1 on the right, in a 1000×1000 window.
fn system(left: DockNode<Kind>, right: DockNode<Kind>) -> PanelSystem<Kind> {
	let mut system = PanelSystem::new()
		.with_layout(LayoutBuilder::hsplit(left, right, 0.5))
		.with_ids(10);
	system.window_size = Size::new(1000.0, 1000.0);
	system.materialize_split_sizes();
	system
}

fn area(system: &PanelSystem<Kind>, id: usize) -> (Vec<Kind>, usize) {
	let area = system.find_area(id).expect("area exists");
	(area.tabs().to_vec(), area.active_index())
}

type Msg = PanelSystemMessage<Kind, ()>;

#[test]
fn removing_the_active_tab_selects_its_right_neighbor() {
	let mut area = Area::with_tabs(0, vec![Kind::A, Kind::B, Kind::C], 1);
	assert_eq!(area.remove_tab(1), Some(Kind::B));
	assert_eq!(area.content(), &Kind::C);

	area.activate(1);
	area.remove_tab(1);
	assert_eq!(area.content(), &Kind::A);
}

#[test]
fn removing_a_tab_before_the_active_one_keeps_it_active() {
	let mut area = Area::with_tabs(0, vec![Kind::A, Kind::B, Kind::C], 2);
	area.remove_tab(0);
	assert_eq!(area.content(), &Kind::C);
}

#[test]
fn move_tab_uses_slots_counted_before_the_move() {
	let mut area = Area::with_tabs(0, vec![Kind::A, Kind::B, Kind::C], 0);
	area.move_tab(0, 3);
	assert_eq!(area.tabs(), &[Kind::B, Kind::C, Kind::A]);
	assert_eq!(area.content(), &Kind::A);

	area.move_tab(2, 0);
	assert_eq!(area.tabs(), &[Kind::A, Kind::B, Kind::C]);
}

#[test]
fn add_tab_activates_an_existing_tab_instead_of_duplicating() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	system.update(Msg::AddTab(0, Kind::B));
	assert_eq!(area(&system, 0), (vec![Kind::A, Kind::B], 1));

	system.update(Msg::AddTab(0, Kind::C));
	assert_eq!(area(&system, 0), (vec![Kind::A, Kind::B, Kind::C], 2));
}

#[test]
fn change_editor_replaces_only_the_active_tab() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 1), tabs(1, vec![Kind::C], 0));
	system.update(Msg::ChangeEditor(0, Kind::C));
	assert_eq!(area(&system, 0), (vec![Kind::A, Kind::C], 1));
}

#[test]
fn change_editor_activates_an_existing_tab_instead_of_duplicating() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 1), tabs(1, vec![Kind::C], 0));
	system.update(Msg::ToggleEditorMenu(0));
	system.update(Msg::ChangeEditor(0, Kind::A));
	assert_eq!(area(&system, 0), (vec![Kind::A, Kind::B], 0));

	// The closing menu stays under the tab that opened it.
	let menu = system.editor_menu.as_ref().expect("menu is fading out");
	assert_eq!((menu.to, menu.anchor_tab), (0.0, 1));
}

#[test]
fn inserting_a_tab_next_to_the_same_editor_merges_it() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::C], 0), tabs(1, vec![Kind::B, Kind::C], 0));
	system.apply_tab_drop(0, 1, TabDrop::Insert { area_id: 1, index: 0 });
	assert_eq!(area(&system, 0), (vec![Kind::A], 0));
	assert_eq!(area(&system, 1), (vec![Kind::B, Kind::C], 1));
	assert_eq!(system.focused_area, Some(1));
}

#[test]
fn merging_into_the_same_editor_activates_it() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::B, Kind::C, Kind::A], 0));
	system.apply_tab_drop(0, 0, TabDrop::Merge { area_id: 1 });
	assert!(system.find_area(0).is_none());
	assert_eq!(area(&system, 1), (vec![Kind::B, Kind::C, Kind::A], 2));
}

#[test]
fn dropping_onto_an_area_with_the_same_editor_previews_a_merge() {
	let system = system(tabs(0, vec![Kind::A, Kind::C], 0), tabs(1, vec![Kind::B, Kind::C], 0));
	let (bounds, geometry) = system.tab_bar_geometry(1).expect("area 1");

	// Header and body both report a merge, so no insert marker is drawn.
	let header = Point::new(geometry.tab_x(0) + 2.0, bounds.y + HEADER_HEIGHT * 0.5);
	let center = Point::new(bounds.center_x(), bounds.center_y());
	for pos in [header, center] {
		assert_eq!(system.tab_drop_at(0, 1, pos), Some(TabDrop::Merge { area_id: 1 }));
	}
	assert!(system.tab_already_open(0, 1, 1));
	assert!(!system.tab_already_open(0, 0, 1));
}

#[test]
#[should_panic(expected = "one tab at most")]
fn layout_builder_rejects_duplicate_tabs() {
	LayoutBuilder::new().tabs(vec![Kind::A, Kind::B, Kind::A]);
}

#[test]
fn closing_the_last_tab_closes_the_area() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	system.update(Msg::CloseTab(0, 0));
	assert_eq!(area(&system, 0), (vec![Kind::B], 0));

	system.update(Msg::CloseTab(1, 0));
	assert!(system.find_area(1).is_none());
	assert!(matches!(system.root(), DockNode::Leaf(area) if area.id == 0));

	// The only remaining area keeps its last tab.
	system.update(Msg::CloseTab(0, 0));
	assert_eq!(area(&system, 0), (vec![Kind::B], 0));
}

#[test]
fn moving_the_only_tab_into_another_area_closes_the_source() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::B, Kind::C], 0));
	system.apply_tab_drop(0, 0, TabDrop::Insert { area_id: 1, index: 1 });
	assert!(system.find_area(0).is_none());
	assert_eq!(area(&system, 1), (vec![Kind::B, Kind::A, Kind::C], 1));
	assert_eq!(system.focused_area, Some(1));
}

#[test]
fn merging_appends_the_tab() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	system.apply_tab_drop(0, 1, TabDrop::Merge { area_id: 1 });
	assert_eq!(area(&system, 0), (vec![Kind::A], 0));
	assert_eq!(area(&system, 1), (vec![Kind::C, Kind::B], 1));
}

#[test]
fn split_drop_tears_the_tab_out_of_its_own_area() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	system.apply_tab_drop(
		0,
		1,
		TabDrop::Split {
			area_id: 0,
			direction: SplitDirection::Vertical,
			ratio: 0.5,
			new_is_first: false,
		},
	);
	assert_eq!(area(&system, 0), (vec![Kind::A], 0));
	assert_eq!(area(&system, 10), (vec![Kind::B], 0));
	assert_eq!(system.focused_area, Some(10));
}

#[test]
fn corner_move_carries_every_tab() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 1), tabs(1, vec![Kind::C], 0));
	system.move_area_into(0, 1, SplitDirection::Vertical, 0.5, true);
	assert!(system.find_area(0).is_none());
	assert_eq!(area(&system, 10), (vec![Kind::A, Kind::B], 1));
}

#[test]
fn tab_drop_detection() {
	let system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	let (bounds, geometry) = system.tab_bar_geometry(1).expect("area 1");
	let header_y = bounds.y + HEADER_HEIGHT * 0.5;

	// Right half of the target's only tab → after it.
	let pos = Point::new(geometry.tab_x(0) + geometry.tab_width * 0.75, header_y);
	assert_eq!(
		system.tab_drop_at(0, 0, pos),
		Some(TabDrop::Insert { area_id: 1, index: 1 })
	);

	// Dropping a tab back into its own slot changes nothing.
	let (_, own) = system.tab_bar_geometry(0).expect("area 0");
	let pos = Point::new(own.tab_x(0) + own.tab_width * 0.25, header_y);
	assert_eq!(system.tab_drop_at(0, 0, pos), None);

	// Body center → merge; own body center → nothing.
	let center = Point::new(bounds.center_x(), bounds.center_y());
	assert_eq!(system.tab_drop_at(0, 0, center), Some(TabDrop::Merge { area_id: 1 }));
	assert_eq!(system.tab_drop_at(1, 0, center), None);

	// The only tab of an area cannot split off from it.
	let edge = Point::new(bounds.x + 5.0, bounds.center_y());
	assert_eq!(system.tab_drop_at(1, 0, edge), None);
	assert!(matches!(
		system.tab_drop_at(0, 1, edge),
		Some(TabDrop::Split { area_id: 1, .. })
	));
}

#[test]
fn clicking_the_active_tab_toggles_the_editor_menu() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));

	system.update(Msg::TabPress(0, 1));
	system.update(Msg::TabDragEnd);
	assert_eq!(area(&system, 0).1, 1);
	assert!(system.editor_menu.is_none(), "selecting a tab does not open the menu");

	system.update(Msg::TabPress(0, 1));
	system.update(Msg::TabDragEnd);
	assert!(
		system
			.editor_menu
			.as_ref()
			.is_some_and(|menu| menu.mode == EditorMenuMode::Replace && menu.to > 0.0)
	);
}

#[test]
fn dragging_a_tab_drops_it_on_release() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	let (bounds, _) = system.tab_bar_geometry(1).expect("area 1");

	system.update(Msg::MouseMove(Point::new(20.0, 10.0)));
	system.update(Msg::TabPress(0, 0));
	system.update(Msg::MouseMove(Point::new(bounds.center_x(), bounds.center_y())));
	system.update(Msg::TabDragEnd);

	assert_eq!(area(&system, 0), (vec![Kind::B], 0));
	assert_eq!(area(&system, 1), (vec![Kind::C, Kind::A], 1));
	assert!(matches!(system.drag_state, DragState::None));
}

#[test]
fn releasing_outside_the_panels_cancels_the_drag() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	let (bounds, _) = system.tab_bar_geometry(1).expect("area 1");

	system.update(Msg::MouseMove(Point::new(20.0, 10.0)));
	system.update(Msg::TabPress(0, 0));
	system.update(Msg::MouseMove(Point::new(bounds.center_x(), bounds.center_y())));
	system.update(Msg::PointerExited);
	system.update(Msg::PointerReleased);
	assert!(matches!(system.drag_state, DragState::None));

	// A later click in a panel body must not apply the abandoned drop.
	system.update(Msg::MouseMove(Point::new(bounds.center_x(), bounds.center_y())));
	system.update(Msg::TabDragEnd);
	assert_eq!(area(&system, 0), (vec![Kind::A, Kind::B], 0));
	assert_eq!(area(&system, 1), (vec![Kind::C], 0));
}

#[test]
fn releasing_inside_keeps_the_drop_for_the_overlay() {
	let mut system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	let (bounds, _) = system.tab_bar_geometry(1).expect("area 1");

	system.update(Msg::MouseMove(Point::new(20.0, 10.0)));
	system.update(Msg::TabPress(0, 0));
	system.update(Msg::MouseMove(Point::new(bounds.center_x(), bounds.center_y())));
	// The window-level release can arrive before the overlay's.
	system.update(Msg::PointerReleased);
	system.update(Msg::TabDragEnd);
	assert_eq!(area(&system, 1), (vec![Kind::C, Kind::A], 1));
}

#[test]
fn header_bottom_row_counts_as_header() {
	let system = system(tabs(0, vec![Kind::A, Kind::B], 0), tabs(1, vec![Kind::C], 0));
	let (bounds, geometry) = system.tab_bar_geometry(1).expect("area 1");
	let pos = Point::new(
		geometry.tab_x(0) + geometry.tab_width * 0.75,
		bounds.y + PANEL_OUTLINE_WIDTH + HEADER_HEIGHT - 0.5,
	);
	assert_eq!(
		system.tab_drop_at(0, 0, pos),
		Some(TabDrop::Insert { area_id: 1, index: 1 })
	);
}

#[test]
fn corner_move_keeps_focus_on_the_moved_area() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::C], 0));
	system.focused_area = Some(0);
	system.move_area_into(0, 1, SplitDirection::Vertical, 0.5, true);
	assert_eq!(system.focused_area, Some(10));
}

#[test]
fn pointer_move_highlights_the_area_under_the_cursor() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::B], 0));
	assert_eq!(system.hovered_area(), None);

	system.update::<()>(Msg::MouseMove(Point::new(10.0, 10.0)));
	assert_eq!(system.hovered_area(), Some(0));
	// ポインタの移動はフォーカスを変えない
	assert_eq!(system.focused_area, None);

	system.update::<()>(Msg::MouseMove(Point::new(990.0, 10.0)));
	assert_eq!(system.hovered_area(), Some(1));
}

#[test]
fn hover_uses_half_open_area_bounds_and_skips_the_gutter() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::B], 0));

	// 1000px 幅・GAP 6px の 1:1 分割では [0,497) が左、[497,503) がガター、[503,1000) が右
	for (x, expected) in [(496.9, Some(0)), (497.0, None), (502.9, None), (503.0, Some(1))] {
		system.update::<()>(Msg::MouseMove(Point::new(x, 300.0)));
		assert_eq!(system.hovered_area(), expected, "x = {x}");
	}
}

#[test]
fn hover_targets_the_area_under_the_cursor_in_a_vertical_split() {
	let mut system = PanelSystem::new()
		.with_layout(LayoutBuilder::vsplit(
			tabs(0, vec![Kind::A], 0),
			tabs(1, vec![Kind::B], 0),
			0.5,
		))
		.with_ids(10);
	system.window_size = Size::new(1000.0, 1000.0);
	system.materialize_split_sizes();

	for (y, expected) in [(10.0, Some(0)), (496.9, Some(0)), (500.0, None), (503.0, Some(1))] {
		system.update::<()>(Msg::MouseMove(Point::new(500.0, y)));
		assert_eq!(system.hovered_area(), expected, "y = {y}");
	}
}

#[test]
fn the_resize_band_hides_the_panel_highlight() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::B], 0));

	// 12px のリサイズ帯はパネル端から 3px はみ出し、その範囲のクリックはガター操作になる
	system.update::<()>(Msg::MouseMove(Point::new(495.0, 300.0)));
	assert_eq!(system.hovered_area(), Some(0));

	system.update::<()>(Msg::GutterHover(vec![], true));
	assert_eq!(system.hovered_area(), None);

	system.update::<()>(Msg::GutterHover(vec![], false));
	assert_eq!(system.hovered_area(), Some(0));
}

#[test]
fn hover_follows_the_layout_without_pointer_motion() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::B], 0));
	system.update::<()>(Msg::MouseMove(Point::new(10.0, 10.0)));
	assert_eq!(system.hovered_area(), Some(0));

	// ポインタを動かさない入れ替えでも、強調表示は新しい配置から導出される
	system.swap_areas(0, 1);
	assert_eq!(system.hovered_area(), Some(1));
}

#[test]
fn leaving_the_panels_clears_the_highlight() {
	let mut system = system(tabs(0, vec![Kind::A], 0), tabs(1, vec![Kind::B], 0));
	system.update::<()>(Msg::MouseMove(Point::new(10.0, 10.0)));
	assert_eq!(system.hovered_area(), Some(0));

	system.update::<()>(Msg::PointerExited);
	assert_eq!(system.hovered_area(), None);
}

#[test]
fn tab_geometry_shares_width_and_caps_it() {
	let bounds = Rectangle::new(Point::ORIGIN, Size::new(1000.0, 300.0));
	let wide = TabBarGeometry::new(bounds, 2, true);
	assert_eq!(wide.tab_width, TAB_MAX_WIDTH);

	let narrow = TabBarGeometry::new(Rectangle::new(Point::ORIGIN, Size::new(200.0, 300.0)), 4, true);
	assert!(narrow.tab_width < TAB_MAX_WIDTH);
	assert!(narrow.add_button_x() + constants::style::CONTROL_CONTENT_HEIGHT <= 200.0);
	assert_eq!(narrow.slot_at(-50.0), 0);
	assert_eq!(narrow.slot_at(10_000.0), 4);
}
