//! Typed messages and property groups for the properties (inspector) panel.

use domain::TransformProperty;

/// A row in the Transform section. Vector rows edit several channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyGroup {
	Position,
	Rotation,
	Scale,
	Opacity,
}

impl PropertyGroup {
	pub const ALL: [Self; 4] = [Self::Position, Self::Rotation, Self::Scale, Self::Opacity];

	/// Scalar channels edited by this row (2D layers: X/Y and Z rotation).
	pub fn channels(self) -> &'static [TransformProperty] {
		match self {
			Self::Position => &[TransformProperty::PositionX, TransformProperty::PositionY],
			Self::Rotation => &[TransformProperty::RotationZ],
			Self::Scale => &[TransformProperty::ScaleX, TransformProperty::ScaleY],
			Self::Opacity => &[TransformProperty::Opacity],
		}
	}

	pub fn label(self) -> &'static str {
		match self {
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
	Transform,
	Text,
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
