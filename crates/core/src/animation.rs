//! Scalar keyframe animation curves.

const KEY_TIME_EPSILON: f32 = 0.0001;
const DEFAULT_HANDLE_TIME: f32 = 1.0 / 3.0;

/// Interpolation used from a keyframe to the following keyframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
	#[default]
	Bezier,
	Linear,
	Hold,
}

/// One side of a Bezier keyframe handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleSide {
	Left,
	Right,
}

/// A scalar keyframe. Handle values are offsets from `(time, value)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe {
	pub id: u64,
	pub time: f32,
	pub value: f32,
	pub left_handle: [f32; 2],
	pub right_handle: [f32; 2],
	pub interpolation: Interpolation,
}

impl Keyframe {
	fn new(id: u64, time: f32, value: f32) -> Self {
		Self {
			id,
			time,
			value,
			left_handle: [-DEFAULT_HANDLE_TIME, 0.0],
			right_handle: [DEFAULT_HANDLE_TIME, 0.0],
			interpolation: Interpolation::Bezier,
		}
	}

	pub fn handle_position(&self, side: HandleSide) -> [f32; 2] {
		let offset = match side {
			HandleSide::Left => self.left_handle,
			HandleSide::Right => self.right_handle,
		};
		[self.time + offset[0], self.value + offset[1]]
	}
}

/// An ordered collection of scalar keyframes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimationCurve {
	keys: Vec<Keyframe>,
	next_id: u64,
}

impl AnimationCurve {
	pub fn keys(&self) -> &[Keyframe] {
		&self.keys
	}

	pub fn is_empty(&self) -> bool {
		self.keys.is_empty()
	}

	pub fn key(&self, id: u64) -> Option<&Keyframe> {
		self.keys.iter().find(|key| key.id == id)
	}

	/// Adds a key or replaces the value of an existing key at the same time.
	pub fn add_key(&mut self, time: f32, value: f32) -> u64 {
		if let Some(key) = self
			.keys
			.iter_mut()
			.find(|key| (key.time - time).abs() <= KEY_TIME_EPSILON)
		{
			key.value = value;
			return key.id;
		}

		let id = self.next_id;
		self.next_id += 1;
		self.keys.push(Keyframe::new(id, time.max(0.0), value));
		self.sort_keys();
		id
	}

	pub fn remove_key(&mut self, id: u64) -> bool {
		let Some(index) = self.keys.iter().position(|key| key.id == id) else {
			return false;
		};
		self.keys.remove(index);
		true
	}

	pub fn set_key(&mut self, id: u64, time: f32, value: f32) -> bool {
		let Some(index) = self.keys.iter().position(|key| key.id == id) else {
			return false;
		};

		let min_time = index
			.checked_sub(1)
			.and_then(|previous| self.keys.get(previous))
			.map_or(0.0, |key| key.time + KEY_TIME_EPSILON);
		let max_time = self
			.keys
			.get(index + 1)
			.map_or(f32::INFINITY, |key| key.time - KEY_TIME_EPSILON);
		let key = &mut self.keys[index];
		key.time = time.clamp(min_time, max_time);
		key.value = value;
		true
	}

	pub fn set_interpolation(&mut self, id: u64, interpolation: Interpolation) -> bool {
		let Some(key) = self.keys.iter_mut().find(|key| key.id == id) else {
			return false;
		};
		key.interpolation = interpolation;
		true
	}

	/// Moves one Bezier handle while keeping its time inside the adjacent segment.
	pub fn set_handle(&mut self, id: u64, side: HandleSide, time: f32, value: f32) -> bool {
		let Some(index) = self.keys.iter().position(|key| key.id == id) else {
			return false;
		};
		let key_time = self.keys[index].time;
		let constrained_time = match side {
			HandleSide::Left => {
				let minimum = index
					.checked_sub(1)
					.and_then(|previous| self.keys.get(previous))
					.map_or(key_time, |key| key.time);
				time.clamp(minimum, key_time)
			}
			HandleSide::Right => {
				let maximum = self.keys.get(index + 1).map_or(key_time, |key| key.time);
				time.clamp(key_time, maximum)
			}
		};
		let offset = [constrained_time - key_time, value - self.keys[index].value];
		match side {
			HandleSide::Left => self.keys[index].left_handle = offset,
			HandleSide::Right => self.keys[index].right_handle = offset,
		}
		true
	}

	pub fn evaluate(&self, base_value: f32, time: f32) -> f32 {
		let Some(first) = self.keys.first() else {
			return base_value;
		};
		if time <= first.time {
			return first.value;
		}
		let Some(last) = self.keys.last() else {
			return base_value;
		};
		if time >= last.time {
			return last.value;
		}

		let next_index = self.keys.partition_point(|key| key.time < time);
		let left = self.keys[next_index - 1];
		let right = self.keys[next_index];
		match left.interpolation {
			Interpolation::Hold => left.value,
			Interpolation::Linear => {
				let amount = (time - left.time) / (right.time - left.time);
				left.value + (right.value - left.value) * amount
			}
			Interpolation::Bezier => evaluate_bezier(left, right, time),
		}
	}

	pub fn bounds(&self) -> Option<([f32; 2], [f32; 2])> {
		let first = self.keys.first()?;
		let mut min = [first.time, first.value];
		let mut max = min;
		for key in &self.keys {
			min[0] = min[0].min(key.time);
			min[1] = min[1].min(key.value);
			max[0] = max[0].max(key.time);
			max[1] = max[1].max(key.value);
		}
		Some((min, max))
	}

	fn sort_keys(&mut self) {
		self.keys.sort_by(|a, b| a.time.total_cmp(&b.time));
	}
}

fn evaluate_bezier(left: Keyframe, right: Keyframe, time: f32) -> f32 {
	let p0 = [left.time, left.value];
	let p1 = [
		(left.time + left.right_handle[0]).clamp(left.time, right.time),
		left.value + left.right_handle[1],
	];
	let p2 = [
		(right.time + right.left_handle[0]).clamp(left.time, right.time),
		right.value + right.left_handle[1],
	];
	let p3 = [right.time, right.value];

	let mut low = 0.0;
	let mut high = 1.0;
	for _ in 0..24 {
		let parameter = (low + high) * 0.5;
		if cubic(p0[0], p1[0], p2[0], p3[0], parameter) < time {
			low = parameter;
		} else {
			high = parameter;
		}
	}
	cubic(p0[1], p1[1], p2[1], p3[1], (low + high) * 0.5)
}

fn cubic(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
	let inverse = 1.0 - t;
	inverse.powi(3) * p0 + 3.0 * inverse.powi(2) * t * p1 + 3.0 * inverse * t.powi(2) * p2 + t.powi(3) * p3
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_curve_returns_base_value() {
		assert_eq!(AnimationCurve::default().evaluate(3.0, 1.0), 3.0);
	}

	#[test]
	fn adding_at_same_time_replaces_value() {
		let mut curve = AnimationCurve::default();
		let first_id = curve.add_key(1.0, 2.0);
		let replacement_id = curve.add_key(1.0, 4.0);
		assert_eq!(first_id, replacement_id);
		assert_eq!(curve.keys().len(), 1);
		assert_eq!(curve.evaluate(0.0, 1.0), 4.0);
	}

	#[test]
	fn linear_curve_interpolates_between_keys() {
		let mut curve = AnimationCurve::default();
		let first = curve.add_key(0.0, 10.0);
		curve.add_key(2.0, 20.0);
		curve.set_interpolation(first, Interpolation::Linear);
		assert!((curve.evaluate(0.0, 0.5) - 12.5).abs() < 0.0001);
	}

	#[test]
	fn hold_curve_keeps_previous_value() {
		let mut curve = AnimationCurve::default();
		let first = curve.add_key(0.0, 10.0);
		curve.add_key(2.0, 20.0);
		curve.set_interpolation(first, Interpolation::Hold);
		assert_eq!(curve.evaluate(0.0, 1.5), 10.0);
	}

	#[test]
	fn bezier_curve_reaches_endpoints() {
		let mut curve = AnimationCurve::default();
		curve.add_key(1.0, 4.0);
		curve.add_key(3.0, 8.0);
		assert_eq!(curve.evaluate(0.0, 0.0), 4.0);
		assert_eq!(curve.evaluate(0.0, 4.0), 8.0);
		assert!((curve.evaluate(0.0, 2.0) - 6.0).abs() < 0.001);
	}

	#[test]
	fn key_movement_cannot_cross_neighbor() {
		let mut curve = AnimationCurve::default();
		let first = curve.add_key(1.0, 1.0);
		curve.add_key(2.0, 2.0);
		curve.set_key(first, 3.0, 3.0);
		assert!(curve.key(first).expect("key").time < 2.0);
	}

	#[test]
	fn handle_time_stays_inside_segment() {
		let mut curve = AnimationCurve::default();
		let first = curve.add_key(1.0, 1.0);
		curve.add_key(2.0, 2.0);
		curve.set_handle(first, HandleSide::Right, 8.0, 3.0);
		assert_eq!(
			curve.key(first).expect("key").handle_position(HandleSide::Right)[0],
			2.0
		);
	}
}
