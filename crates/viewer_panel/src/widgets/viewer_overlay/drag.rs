//! Tool presses and drags turned into viewer messages.

use crate::message::PreviewMessage;
use crate::tool::Tool;
use crate::widgets::viewer_overlay::layer_box::LayerBox;
use crate::widgets::viewer_overlay::{CLICK_ZOOM_FACTOR, Drag, OverlayState, ViewerOverlay};
use crate::widgets::viewport::Viewport;
use iced::widget::canvas;
use iced::{Point, Vector};
use nade_core::InstanceId;

impl ViewerOverlay {
	/// Convert a comp-space delta into the layer's parent space.
	pub(super) fn to_parent(layer: &LayerBox, delta: [f32; 2]) -> [f32; 2] {
		layer
			.parent
			.inverse()
			.map_or(delta, |inverse| inverse.apply_vector(delta))
	}

	pub(super) fn view_changed(zoom: f32, offset: Vector) -> PreviewMessage {
		PreviewMessage::ViewChanged {
			zoom,
			offset: [offset.x, offset.y],
		}
	}

	pub(super) fn press_left(
		&self,
		state: &mut OverlayState,
		viewport: &Viewport,
		point: Point,
	) -> Option<canvas::Action<PreviewMessage>> {
		let comp = viewport.screen_to_comp(point);
		match self.tool {
			Tool::Hand => {
				state.drag = Some(Drag::Pan { last: point });
				Some(canvas::Action::capture())
			}
			Tool::Zoom => {
				let factor = if state.modifiers.alt() {
					1.0 / CLICK_ZOOM_FACTOR
				} else {
					CLICK_ZOOM_FACTOR
				};
				let (zoom, offset) = viewport.zoomed_at(factor, point);
				Some(canvas::Action::publish(Self::view_changed(zoom, offset)).and_capture())
			}
			Tool::Text => Some(canvas::Action::publish(PreviewMessage::CreateText { position: comp }).and_capture()),
			Tool::Select => {
				if let Some((layer, handle)) = self.hit_handle(viewport, point) {
					state.drag = Some(Drag::Scale {
						handle,
						local: layer.handle_local(handle),
						inverse: layer.matrix.inverse()?,
						anchor: layer.anchor,
						scale: layer.scale,
					});
					return Some(canvas::Action::capture());
				}
				let Some(hit) = self.hit_test(viewport, point) else {
					return Some(canvas::Action::publish(PreviewMessage::SelectLayer(None)).and_capture());
				};
				state.drag = Some(Drag::Move { last: comp });
				Some(Self::select_then_capture(hit.id, self.selected))
			}
			Tool::Anchor => {
				let target = self.hit_test(viewport, point).or_else(|| self.selected_layer())?;
				state.drag = Some(Drag::Anchor { last: comp });
				Some(Self::select_then_capture(target.id, self.selected))
			}
			Tool::Rotate => {
				let target = self.hit_test(viewport, point).or_else(|| self.selected_layer())?;
				let center = viewport.comp_to_screen(target.anchor_point());
				state.drag = Some(Drag::Rotate {
					center,
					last_angle: (point.y - center.y).atan2(point.x - center.x).to_degrees(),
				});
				Some(Self::select_then_capture(target.id, self.selected))
			}
		}
	}

	pub(super) fn select_then_capture(id: InstanceId, selected: Option<InstanceId>) -> canvas::Action<PreviewMessage> {
		if selected == Some(id) {
			canvas::Action::capture()
		} else {
			canvas::Action::publish(PreviewMessage::SelectLayer(Some(id))).and_capture()
		}
	}

	pub(super) fn drag_to(
		&self,
		state: &mut OverlayState,
		viewport: &Viewport,
		point: Point,
	) -> Option<PreviewMessage> {
		match state.drag? {
			Drag::Pan { last } => {
				state.drag = Some(Drag::Pan { last: point });
				Some(Self::view_changed(
					self.zoom,
					Vector::new(self.offset.x + point.x - last.x, self.offset.y + point.y - last.y),
				))
			}
			Drag::Move { last } => {
				let comp = viewport.screen_to_comp(point);
				state.drag = Some(Drag::Move { last: comp });
				let mut delta = [comp[0] - last[0], comp[1] - last[1]];
				// Shift: 主軸方向だけに拘束
				if state.modifiers.shift() {
					if delta[0].abs() > delta[1].abs() {
						delta[1] = 0.0;
					} else {
						delta[0] = 0.0;
					}
				}
				let layer = self.selected_layer()?;
				Some(PreviewMessage::MoveLayer {
					delta: Self::to_parent(layer, delta),
				})
			}
			Drag::Scale {
				handle,
				local,
				inverse,
				anchor,
				scale,
			} => {
				let pointer = inverse.apply(viewport.screen_to_comp(point));
				let from = [local[0] - anchor[0], local[1] - anchor[1]];
				let to = [pointer[0] - anchor[0], pointer[1] - anchor[1]];
				let ratio = |axis: usize| {
					if handle[axis] == 0.0 || from[axis].abs() < 1e-3 {
						1.0
					} else {
						to[axis] / from[axis]
					}
				};
				let mut factor = [ratio(0), ratio(1)];
				// Shift: 縦横比を保つ（角はアンカー→ハンドル方向へ射影）
				if state.modifiers.shift() {
					let uniform = if handle[0] != 0.0 && handle[1] != 0.0 {
						let length = from[0] * from[0] + from[1] * from[1];
						if length > 1e-6 {
							(to[0] * from[0] + to[1] * from[1]) / length
						} else {
							1.0
						}
					} else if handle[0] != 0.0 {
						factor[0]
					} else {
						factor[1]
					};
					factor = [uniform, uniform];
				}
				Some(PreviewMessage::ScaleLayer {
					scale: [scale[0] * factor[0], scale[1] * factor[1]],
				})
			}
			Drag::Anchor { last } => {
				let comp = viewport.screen_to_comp(point);
				state.drag = Some(Drag::Anchor { last: comp });
				let delta = [comp[0] - last[0], comp[1] - last[1]];
				let layer = self.selected_layer()?;
				let anchor = layer.matrix.inverse()?.apply_vector(delta);
				Some(PreviewMessage::MoveAnchor {
					anchor,
					position: Self::to_parent(layer, delta),
				})
			}
			Drag::Rotate { center, last_angle } => {
				let angle = (point.y - center.y).atan2(point.x - center.x).to_degrees();
				let mut delta = angle - last_angle;
				if delta > 180.0 {
					delta -= 360.0;
				} else if delta < -180.0 {
					delta += 360.0;
				}
				state.drag = Some(Drag::Rotate {
					center,
					last_angle: angle,
				});
				Some(PreviewMessage::RotateLayer { degrees: delta })
			}
		}
	}
}
