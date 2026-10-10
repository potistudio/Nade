//! UI icon set (16×16 grid, 1.5px rounded strokes, SF Symbols–like).
//!
//! Icons are monochrome white SVGs; tint them through [`icon`] so they follow
//! the current text color hierarchy.

use iced::widget::svg;
use iced::{Color, Element, Rotation};

use crate::widgets::icon_slot;

macro_rules! icon_set {
	($($variant:ident => $file:literal,)*) => {
		/// Every icon shipped in `assets/icons/ui`.
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
		pub enum Icon {
			$($variant,)*
		}

		impl Icon {
			fn bytes(self) -> &'static [u8] {
				match self {
					$(Self::$variant => include_bytes!(concat!("../../../assets/icons/ui/", $file, ".svg")),)*
				}
			}
		}
	};
}

icon_set! {
	Appearance => "appearance",
	Audio => "audio",
	Check => "check",
	ChevronDown => "chevron_down",
	ChevronLeft => "chevron_left",
	ChevronRight => "chevron_right",
	ChevronUpDown => "chevron_up_down",
	Close => "close",
	Comp => "comp",
	Eye => "eye",
	EyeOff => "eye_off",
	Fit => "fit",
	Folder => "folder",
	FolderPlus => "folder_plus",
	Grid => "grid",
	Image => "image",
	Import => "import",
	Insert => "insert",
	InterpBezier => "interp_bezier",
	InterpHold => "interp_hold",
	InterpLinear => "interp_linear",
	Keyframe => "keyframe",
	KeyframeFilled => "keyframe_filled",
	LayerNull => "layer_null",
	LayerText => "layer_text",
	Lock => "lock",
	Loop => "loop",
	Minus => "minus",
	PanelAssets => "panel_assets",
	PanelCurves => "panel_curves",
	PanelInspector => "panel_inspector",
	PanelLayers => "panel_layers",
	PanelTimeline => "panel_timeline",
	PanelViewer => "panel_viewer",
	Pause => "pause",
	Play => "play",
	Plus => "plus",
	Reset => "reset",
	SafeArea => "safe_area",
	SkipEnd => "skip_end",
	SkipStart => "skip_start",
	Solo => "solo",
	Sparkle => "sparkle",
	StepBack => "step_back",
	StepForward => "step_forward",
	Stopwatch => "stopwatch",
	ToolHand => "tool_hand",
	ToolRotate => "tool_rotate",
	ToolSelect => "tool_select",
	ToolText => "tool_text",
	ToolZoom => "tool_zoom",
	Transform => "transform",
	Trash => "trash",
	Video => "video",
}

impl Icon {
	/// SVG handle for use with `iced::widget::svg`.
	pub fn handle(self) -> svg::Handle {
		svg::Handle::from_memory(self.bytes())
	}
}

/// Tinted icon centered in a square slot of `size`.
pub fn icon<'a, Message: 'a>(icon: Icon, size: f32, color: Color) -> Element<'a, Message> {
	icon_slot(
		svg(icon.handle())
			.width(size)
			.height(size)
			.style(move |_theme, _status| svg::Style { color: Some(color) }),
		size,
	)
}

/// Tinted icon with rotation (radians) and opacity, for animated glyphs.
pub fn icon_transformed<'a, Message: 'a>(
	icon: Icon,
	size: f32,
	color: Color,
	rotation: f32,
	opacity: f32,
) -> Element<'a, Message> {
	icon_slot(
		svg(icon.handle())
			.width(size)
			.height(size)
			.rotation(Rotation::Floating(iced::Radians(rotation)))
			.opacity(opacity)
			.style(move |_theme, _status| svg::Style { color: Some(color) }),
		size,
	)
}
