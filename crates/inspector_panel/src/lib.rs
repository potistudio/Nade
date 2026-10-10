/// Messages for the inspector panel
#[derive(Debug, Clone)]
pub enum InspectorMessage {
	UpdateTransform { field: String, index: usize, value: f32 },
	SetOpacity(f32),
	SetText(String),
	SetFontSize(f32),
	SetFillColor { index: usize, value: f32 },
}
