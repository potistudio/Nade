//! Viewer-local state: display settings and the viewport area size.

use crate::message::{PreviewMessage, ViewerRequest};
use crate::settings::ViewerSettings;
use crate::widgets::viewport::{MAX_ZOOM, MIN_ZOOM, Viewport};
use iced::Size;
use nade_core::PreviewModel;

/// Viewer-local view state.
///
/// Pan/zoom values stay in the core preview model (the render pipeline
/// displays them); this owns the display settings and the viewport area
/// size reported by the overlay.
#[derive(Debug, Clone)]
pub struct ViewerState {
	pub settings: ViewerSettings,
	/// Size of the viewport area (used for fit / actual-pixels math).
	pub size: Size,
}

impl Default for ViewerState {
	fn default() -> Self {
		Self {
			settings: ViewerSettings::default(),
			/// Default area until the overlay reports the real size.
			size: Size::new(960.0, 540.0),
		}
	}
}

// -------- Public API ---------------------------------------------------------
impl ViewerState {
	/// Applies viewer-local messages (pan/zoom, display settings) to the
	/// state and returns the shell work that remains.
	pub fn apply(&mut self, preview: &mut PreviewModel, msg: PreviewMessage) -> Option<ViewerRequest> {
		match msg {
			PreviewMessage::ViewChanged { zoom, offset } => {
				preview.view_zoom = zoom;
				preview.view_offset = offset;
				None
			}
			PreviewMessage::ViewportSized(size) => {
				self.size = size;
				None
			}
			PreviewMessage::FitView => {
				preview.view_zoom = 1.0;
				preview.view_offset = [0.0, 0.0];
				None
			}
			PreviewMessage::ZoomBy(factor) => {
				let zoom = (preview.view_zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
				let applied = zoom / preview.view_zoom;
				preview.view_zoom = zoom;
				preview.view_offset = [preview.view_offset[0] * applied, preview.view_offset[1] * applied];
				None
			}
			PreviewMessage::ActualPixels => {
				let viewport = Viewport::new(
					self.size,
					Size::new(preview.width as f32, preview.height as f32),
					preview.view_zoom,
					iced::Vector::new(preview.view_offset[0], preview.view_offset[1]),
				);
				let (zoom, offset) = viewport.with_magnification(1.0);
				preview.view_zoom = zoom;
				preview.view_offset = [offset.x, offset.y];
				None
			}
			PreviewMessage::ToggleGrid => {
				self.settings.show_grid = !self.settings.show_grid;
				None
			}
			PreviewMessage::ToggleSafeAreas => {
				self.settings.show_safe_areas = !self.settings.show_safe_areas;
				None
			}
			PreviewMessage::SetResolution(resolution) => {
				self.settings.resolution = resolution;
				Some(ViewerRequest::RefreshPreview)
			}
			PreviewMessage::Transport(transport) => Some(ViewerRequest::Transport(transport)),
			PreviewMessage::SelectLayer(instance) => Some(ViewerRequest::SelectLayer(instance)),
			PreviewMessage::MoveLayer { delta } => Some(ViewerRequest::MoveLayer { delta }),
			PreviewMessage::ScaleLayer { scale } => Some(ViewerRequest::ScaleLayer { scale }),
			PreviewMessage::MoveAnchor { anchor, position } => Some(ViewerRequest::MoveAnchor { anchor, position }),
			PreviewMessage::RotateLayer { degrees } => Some(ViewerRequest::RotateLayer { degrees }),
			PreviewMessage::CreateText { position } => Some(ViewerRequest::CreateText { position }),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::message::TransportMessage;
	use crate::settings::Resolution;

	fn preview() -> PreviewModel {
		PreviewModel {
			time: 0.0,
			fps: 30.0,
			width: 1920,
			height: 1080,
			is_playing: false,
			frame: None,
			selection: None,
			view_zoom: 1.0,
			view_offset: [0.0, 0.0],
		}
	}

	/// Zooming scales the pan offset along, keeping the anchor point.
	#[test]
	fn zoom_by_scales_the_offset() {
		let mut state = ViewerState::default();
		let mut preview = preview();
		preview.view_offset = [10.0, 20.0];

		state.apply(&mut preview, PreviewMessage::ZoomBy(2.0));
		assert_eq!(preview.view_zoom, 2.0);
		assert_eq!(preview.view_offset, [20.0, 40.0]);
	}

	/// Zoom stops at the configured limits.
	#[test]
	fn zoom_by_clamps_to_the_limits() {
		let mut state = ViewerState::default();
		let mut preview = preview();

		state.apply(&mut preview, PreviewMessage::ZoomBy(1000.0));
		assert_eq!(preview.view_zoom, MAX_ZOOM);
	}

	/// Fit resets zoom and pan.
	#[test]
	fn fit_view_resets_zoom_and_pan() {
		let mut state = ViewerState::default();
		let mut preview = preview();
		preview.view_zoom = 3.0;
		preview.view_offset = [8.0, -4.0];

		state.apply(&mut preview, PreviewMessage::FitView);
		assert_eq!(preview.view_zoom, 1.0);
		assert_eq!(preview.view_offset, [0.0, 0.0]);
	}

	/// Pan/zoom changes from the viewport land verbatim.
	#[test]
	fn view_changed_stores_the_values() {
		let mut state = ViewerState::default();
		let mut preview = preview();

		state.apply(
			&mut preview,
			PreviewMessage::ViewChanged {
				zoom: 0.5,
				offset: [4.0, 6.0],
			},
		);
		assert_eq!(preview.view_zoom, 0.5);
		assert_eq!(preview.view_offset, [4.0, 6.0]);
	}

	/// The overlay-reported size is kept for fit/actual-pixels math.
	#[test]
	fn viewport_sized_tracks_the_area() {
		let mut state = ViewerState::default();
		let mut preview = preview();

		state.apply(&mut preview, PreviewMessage::ViewportSized(Size::new(320.0, 180.0)));
		assert_eq!(state.size, Size::new(320.0, 180.0));
	}

	/// Grid and safe-area guides toggle independently.
	#[test]
	fn guides_toggle_independently() {
		let mut state = ViewerState::default();
		let mut preview = preview();

		state.apply(&mut preview, PreviewMessage::ToggleGrid);
		assert!(state.settings.show_grid);
		assert!(!state.settings.show_safe_areas);

		state.apply(&mut preview, PreviewMessage::ToggleSafeAreas);
		assert!(state.settings.show_safe_areas);
	}

	/// Resolution changes apply and ask for a preview refresh.
	#[test]
	fn set_resolution_reports_a_refresh() {
		let mut state = ViewerState::default();
		let mut preview = preview();

		let result = state.apply(&mut preview, PreviewMessage::SetResolution(Resolution::Half));
		assert_eq!(state.settings.resolution, Resolution::Half);
		assert!(matches!(result, Some(ViewerRequest::RefreshPreview)));
	}

	/// App-level messages come back as requests for the shell.
	#[test]
	fn app_level_messages_return_requests() {
		let mut state = ViewerState::default();
		let mut preview = preview();

		assert!(matches!(
			state.apply(&mut preview, PreviewMessage::Transport(TransportMessage::TogglePlay)),
			Some(ViewerRequest::Transport(TransportMessage::TogglePlay))
		));
		assert!(matches!(
			state.apply(&mut preview, PreviewMessage::MoveLayer { delta: [1.0, 2.0] }),
			Some(ViewerRequest::MoveLayer { delta: [1.0, 2.0] })
		));
		assert!(matches!(
			state.apply(&mut preview, PreviewMessage::CreateText { position: [0.0, 0.0] }),
			Some(ViewerRequest::CreateText { position: [0.0, 0.0] })
		));
	}
}
