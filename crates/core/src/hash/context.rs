use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::core::EvalContext;

pub(crate) fn hash_context(ctx: &EvalContext) -> u64 {
	let mut hasher = DefaultHasher::new();
	ctx.fps.to_bits().hash(&mut hasher);
	hasher.finish()
}
