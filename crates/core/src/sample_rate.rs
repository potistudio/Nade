/// Represents the sample rate of audio data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SampleRate {
	Hz44100,
	Hz48000,
	Hz96000,
	Hz192000,
}

impl SampleRate {
	/// Converts the `SampleRate` enum variant to its corresponding `u32` value.
	pub fn as_u32(&self) -> u32 {
		match self {
			SampleRate::Hz44100 => 44100,
			SampleRate::Hz48000 => 48000,
			SampleRate::Hz96000 => 96000,
			SampleRate::Hz192000 => 192000,
		}
	}
}
