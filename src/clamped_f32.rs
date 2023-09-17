use genetic::{Crossover, Gen, Mutate};
use ordered_float::{Float, OrderedFloat};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClampedF32<const MIN: i32, const MAX: i32, const DENOM: u32>(OrderedFloat<f32>);

impl<const MIN: i32, const MAX: i32, const DENOM: u32> ClampedF32<MIN, MAX, DENOM> {
    pub fn new(f: f32) -> Self {
        Self(OrderedFloat(f.clamp(Self::min(), Self::max())))
    }

    pub fn as_f32(&self) -> f32 {
        self.0 .0
    }

    pub fn min() -> f32 {
        MIN as f32 / DENOM as f32
    }

    pub fn max() -> f32 {
        MAX as f32 / DENOM as f32
    }
}

impl<const MIN: i32, const MAX: i32, const DENOM: u32> Gen for ClampedF32<MIN, MAX, DENOM> {
    fn gen<R: Rng>(rng: &mut R) -> Self {
        Self::new(rng.gen_range(Self::min()..=Self::max()))
    }
}

impl<const MIN: i32, const MAX: i32, const DENOM: u32> Crossover for ClampedF32<MIN, MAX, DENOM> {
    fn crossover<R: Rng>(&self, other: &Self, rng: &mut R) -> Self {
        match rng.gen_range(0..3) {
            0 => *self,
            1 => *other,
            2 => Self((self.0 + other.0) / 2.0),
            _ => unreachable!(),
        }
    }
}

impl<const MIN: i32, const MAX: i32, const DENOM: u32> Mutate for ClampedF32<MIN, MAX, DENOM> {
    fn mutate<R: Rng>(&mut self, rate: f32, rng: &mut R) {
        let delta = Ord::max(self.0 * rate, OrderedFloat::epsilon());
        self.0 += OrderedFloat(
            rng.gen_range(-delta.0..=delta.0)
                .clamp(Self::min(), Self::max()),
        );
    }
}
