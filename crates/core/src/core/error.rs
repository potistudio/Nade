use super::id::NodeId;
use super::types::OutputType;
use super::value::ValueKind;

#[derive(Debug, Clone)]
pub enum EvalError {
	NodeNotFound(NodeId),
	OutputTypeMismatch {
		expected: OutputType,
		found: OutputType,
	},
	ExpectedFloat {
		found: ValueKind,
	},
	ExpectedBool {
		found: ValueKind,
	},
	ExpectedVec4 {
		found: ValueKind,
	},
	ValueSelectMismatch {
		left: ValueKind,
		right: ValueKind,
	},
	ValueOpMismatch {
		op: &'static str,
		lhs: ValueKind,
		rhs: ValueKind,
	},
	ImageSizeMismatch {
		left: (u32, u32),
		right: (u32, u32),
	},
	InvalidImageBuffer {
		expected: usize,
		actual: usize,
	},
	ResolvedPayloadMismatch {
		expected: &'static str,
	},
}

impl std::fmt::Display for EvalError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NodeNotFound(node_id) => write!(f, "node not found: {}", node_id.0),
			Self::OutputTypeMismatch { expected, found } => {
				write!(
					f,
					"output type mismatch: expected {:?}, found {:?}",
					expected, found
				)
			}
			Self::ExpectedFloat { found } => {
				write!(f, "expected Float, found {}", found)
			}
			Self::ExpectedBool { found } => write!(f, "expected Bool, found {}", found),
			Self::ExpectedVec4 { found } => write!(f, "expected Vec4, found {}", found),
			Self::ValueSelectMismatch { left, right } => {
				write!(f, "select mismatch: left {}, right {}", left, right)
			}
			Self::ValueOpMismatch { op, lhs, rhs } => {
				write!(f, "value op mismatch: {} {} {}", lhs, op, rhs)
			}
			Self::ImageSizeMismatch { left, right } => write!(
				f,
				"image size mismatch: left {}x{}, right {}x{}",
				left.0, left.1, right.0, right.1
			),
			Self::InvalidImageBuffer { expected, actual } => write!(
				f,
				"invalid image buffer: expected {}, actual {}",
				expected, actual
			),
			Self::ResolvedPayloadMismatch { expected } => {
				write!(f, "resolved payload mismatch: expected {expected}")
			}
		}
	}
}

impl std::error::Error for EvalError {}
