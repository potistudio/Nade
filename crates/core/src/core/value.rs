use super::error::EvalError;
use super::id::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
	Float,
	Vec2,
	Vec3,
	Vec4,
	Bool,
}

impl std::fmt::Display for ValueKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Float => write!(f, "Float"),
			Self::Vec2 => write!(f, "Vec2"),
			Self::Vec3 => write!(f, "Vec3"),
			Self::Vec4 => write!(f, "Vec4"),
			Self::Bool => write!(f, "Bool"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
	Float(f32),
	Vec2([f32; 2]),
	Vec3([f32; 3]),
	Vec4([f32; 4]),
	Bool(bool),
}

impl Value {
	pub fn kind(self) -> ValueKind {
		match self {
			Self::Float(_) => ValueKind::Float,
			Self::Vec2(_) => ValueKind::Vec2,
			Self::Vec3(_) => ValueKind::Vec3,
			Self::Vec4(_) => ValueKind::Vec4,
			Self::Bool(_) => ValueKind::Bool,
		}
	}

	pub fn as_f32(self) -> Result<f32, EvalError> {
		match self {
			Self::Float(v) => Ok(v),
			other => Err(EvalError::ExpectedFloat { found: other.kind() }),
		}
	}

	pub fn as_bool(self) -> Result<bool, EvalError> {
		match self {
			Self::Bool(v) => Ok(v),
			other => Err(EvalError::ExpectedBool { found: other.kind() }),
		}
	}

	pub fn as_vec4(self) -> Result<[f32; 4], EvalError> {
		match self {
			Self::Vec4(v) => Ok(v),
			other => Err(EvalError::ExpectedVec4 { found: other.kind() }),
		}
	}

	pub fn add_value(self, other: Value) -> Result<Value, EvalError> {
		match (self, other) {
			(Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
			(Value::Vec2(a), Value::Vec2(b)) => Ok(Value::Vec2([a[0] + b[0], a[1] + b[1]])),
			(Value::Vec3(a), Value::Vec3(b)) => Ok(Value::Vec3([a[0] + b[0], a[1] + b[1], a[2] + b[2]])),
			(Value::Vec4(a), Value::Vec4(b)) => Ok(Value::Vec4([a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]])),
			(Value::Float(a), Value::Vec2(b)) => Ok(Value::Vec2([a + b[0], a + b[1]])),
			(Value::Vec2(a), Value::Float(b)) => Ok(Value::Vec2([a[0] + b, a[1] + b])),
			(Value::Float(a), Value::Vec3(b)) => Ok(Value::Vec3([a + b[0], a + b[1], a + b[2]])),
			(Value::Vec3(a), Value::Float(b)) => Ok(Value::Vec3([a[0] + b, a[1] + b, a[2] + b])),
			(Value::Float(a), Value::Vec4(b)) => Ok(Value::Vec4([a + b[0], a + b[1], a + b[2], a + b[3]])),
			(Value::Vec4(a), Value::Float(b)) => Ok(Value::Vec4([a[0] + b, a[1] + b, a[2] + b, a[3] + b])),
			(lhs, rhs) => Err(EvalError::ValueOpMismatch {
				op: "+",
				lhs: lhs.kind(),
				rhs: rhs.kind(),
			}),
		}
	}

	pub fn mul_value(self, other: Value) -> Result<Value, EvalError> {
		match (self, other) {
			(Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
			(Value::Vec2(a), Value::Vec2(b)) => Ok(Value::Vec2([a[0] * b[0], a[1] * b[1]])),
			(Value::Vec3(a), Value::Vec3(b)) => Ok(Value::Vec3([a[0] * b[0], a[1] * b[1], a[2] * b[2]])),
			(Value::Vec4(a), Value::Vec4(b)) => Ok(Value::Vec4([a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]])),
			(Value::Float(a), Value::Vec2(b)) => Ok(Value::Vec2([a * b[0], a * b[1]])),
			(Value::Vec2(a), Value::Float(b)) => Ok(Value::Vec2([a[0] * b, a[1] * b])),
			(Value::Float(a), Value::Vec3(b)) => Ok(Value::Vec3([a * b[0], a * b[1], a * b[2]])),
			(Value::Vec3(a), Value::Float(b)) => Ok(Value::Vec3([a[0] * b, a[1] * b, a[2] * b])),
			(Value::Float(a), Value::Vec4(b)) => Ok(Value::Vec4([a * b[0], a * b[1], a * b[2], a * b[3]])),
			(Value::Vec4(a), Value::Float(b)) => Ok(Value::Vec4([a[0] * b, a[1] * b, a[2] * b, a[3] * b])),
			(lhs, rhs) => Err(EvalError::ValueOpMismatch {
				op: "*",
				lhs: lhs.kind(),
				rhs: rhs.kind(),
			}),
		}
	}
}

#[derive(Debug, Clone)]
pub struct ValueParam {
	pub base: Value,
	pub mapping: Option<NodeId>,
}

impl ValueParam {
	pub fn new(base: Value) -> Self {
		Self { base, mapping: None }
	}

	pub fn with_mapping(base: Value, mapping: NodeId) -> Self {
		Self {
			base,
			mapping: Some(mapping),
		}
	}
}
