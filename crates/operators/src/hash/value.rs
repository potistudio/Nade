use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use core::Value;

pub(crate) fn hash_f32(value: f32) -> u64 {
	let mut hasher = DefaultHasher::new();
	value.to_bits().hash(&mut hasher);
	hasher.finish()
}

pub(crate) fn hash_f32_pair(a: f32, b: f32) -> u64 {
	let mut hasher = DefaultHasher::new();
	a.to_bits().hash(&mut hasher);
	b.to_bits().hash(&mut hasher);
	hasher.finish()
}

pub(crate) fn hash_value(value: Value) -> u64 {
	let mut hasher = DefaultHasher::new();
	hash_value_into(value, &mut hasher);
	hasher.finish()
}

pub(crate) fn hash_value_pair(a: Value, b: Value) -> u64 {
	let mut hasher = DefaultHasher::new();
	hash_value_into(a, &mut hasher);
	hash_value_into(b, &mut hasher);
	hasher.finish()
}

pub(crate) fn hash_select_inputs(cond: bool, a: Value, b: Value) -> u64 {
	let mut hasher = DefaultHasher::new();
	cond.hash(&mut hasher);
	hash_value_into(a, &mut hasher);
	hash_value_into(b, &mut hasher);
	hasher.finish()
}

fn hash_value_into(value: Value, hasher: &mut DefaultHasher) {
	match value {
		Value::Float(v) => {
			0u8.hash(hasher);
			v.to_bits().hash(hasher);
		}
		Value::Vec2(v) => {
			1u8.hash(hasher);
			v[0].to_bits().hash(hasher);
			v[1].to_bits().hash(hasher);
		}
		Value::Vec3(v) => {
			2u8.hash(hasher);
			v[0].to_bits().hash(hasher);
			v[1].to_bits().hash(hasher);
			v[2].to_bits().hash(hasher);
		}
		Value::Vec4(v) => {
			3u8.hash(hasher);
			v[0].to_bits().hash(hasher);
			v[1].to_bits().hash(hasher);
			v[2].to_bits().hash(hasher);
			v[3].to_bits().hash(hasher);
		}
		Value::Bool(v) => {
			4u8.hash(hasher);
			v.hash(hasher);
		}
	}
}
