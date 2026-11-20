//! Typed messages and property groups for the properties (inspector) panel.

use core::{BlendMode, CompositionId, InstanceId};
use domain::TransformProperty;

/// A row in the Transform section. Vector rows edit several channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyGroup {
	Anchor,
	Position,
	Rotation,
	Scale,
	Opacity,
}

impl PropertyGroup {
	/// Rows in After Effects order.
	pub const ALL: [Self; 5] = [Self::Anchor, Self::Position, Self::Scale, Self::Rotation, Self::Opacity];

	/// Scalar channels edited by this row (2D layers: X/Y and Z rotation).
	pub fn channels(self) -> &'static [TransformProperty] {
		match self {
			Self::Anchor => &[TransformProperty::AnchorX, TransformProperty::AnchorY],
			Self::Position => &[TransformProperty::PositionX, TransformProperty::PositionY],
			Self::Rotation => &[TransformProperty::RotationZ],
			Self::Scale => &[TransformProperty::ScaleX, TransformProperty::ScaleY],
			Self::Opacity => &[TransformProperty::Opacity],
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Anchor => "Anchor",
			Self::Position => "Position",
			Self::Rotation => "Rotation",
			Self::Scale => "Scale",
			Self::Opacity => "Opacity",
		}
	}

	/// Value a channel resets to.
	pub fn default_value(property: TransformProperty) -> f32 {
		match property {
			TransformProperty::ScaleX
			| TransformProperty::ScaleY
			| TransformProperty::ScaleZ
			| TransformProperty::Opacity => 1.0,
			_ => 0.0,
		}
	}
}

/// Collapsible sections of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
	Layer,
	Transform,
	Text,
	Solid,
	Footage,
}

/// Messages for the inspector panel
#[derive(Debug, Clone)]
pub enum InspectorMessage {
	/// Edit one channel at the current time (keys it when animated).
	SetChannel {
		property: TransformProperty,
		value: f32,
	},
	/// Stopwatch: add or remove keys for the row at the current time.
	ToggleKeyframe(PropertyGroup),
	/// Move the playhead to the previous / next key of the row.
	JumpToKey {
		group: PropertyGroup,
		forward: bool,
	},
	ResetTransform,
	ToggleSection(Section),
	/// Lock X/Y scale together.
	ToggleScaleLink,
	Rename(String),
	SetText(String),
	SetFontSize(f32),
	SetTracking(f32),
	SetFillChannel {
		index: usize,
		value: f32,
	},
	SetFillRgb([f32; 3]),
	HexEdited(String),
	HexSubmitted,
	/// Solid layer size in composition pixels.
	SetSolidSize {
		width: f32,
		height: f32,
	},
	/// Scale the layer so its picture fits the composition frame.
	FitToComposition,
	/// Composition length in seconds.
	SetCompositionDuration(f32),
	/// Trim / extend the composition to end with its last layer.
	FitCompositionToLayers,
	/// Composition background color (RGB, 0..1).
	SetCompositionBackground([f32; 3]),
	/// Parent layer (None = no parent); keeps the layer where it is.
	SetParent(Option<InstanceId>),
	SetBlendMode(BlendMode),
	/// Open the composition a comp layer plays.
	OpenNestedComposition,
	/// Place the shown composition as a layer in another composition.
	NestInto(CompositionId),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rows_cover_distinct_channels() {
		let mut seen = Vec::new();
		for group in PropertyGroup::ALL {
			for channel in group.channels() {
				assert!(!seen.contains(channel));
				seen.push(*channel);
			}
		}
		assert_eq!(PropertyGroup::default_value(TransformProperty::ScaleX), 1.0);
		assert_eq!(PropertyGroup::default_value(TransformProperty::PositionY), 0.0);
	}
}
