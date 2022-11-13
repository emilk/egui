#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Easing {
	Linear,
	EaseIn,
	EaseOut,
	EaseInOut,
}

impl Easing {
	#[inline(always)]
	pub fn apply(&self, x: f64) -> f64 {
		match self {
			Easing::Linear => x,
			Easing::EaseIn => 1.0 - (1.0 - x.powf(2.0)).sqrt(),
			Easing::EaseOut =>  (1.0 - (x - 1.0).powf(2.0)).sqrt(),
			Easing::EaseInOut => {
				if x < 0.5 {
					Easing::EaseIn.apply(x * 2.0) / 2.0
				} else {
					(Easing::EaseOut.apply((x - 0.5) * 2.0) / 2.0) + 0.5
				}
			}
		}
	}
}