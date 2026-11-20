//! Properties (inspector) panel: typed messages, property groups and the view.

mod draggable_number;
mod edit;
mod message;
mod state;
mod target;
mod view;

pub use edit::{EditContext, EditOutcome, apply_layer_edit};
pub use message::{EffectEdit, InspectorMessage, PropertyGroup, Section};
pub use state::InspectorUi;
pub use target::{CompositionChoice, InspectorTarget, LayerTarget, ParentChoice};
pub use view::view;
