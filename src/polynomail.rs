use genetic::{Crossover, Gen, Mutate};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

use crate::clamped_f32::ClampedF32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Polynomial {
    coeffs: Vec<ClampedF32<-5, 5, 1>>,
}

impl Polynomial {
    pub fn new(coeffs: Vec<ClampedF32<-5, 5, 1>>) -> Self {
        Self { coeffs }
    }

    pub fn eval(&self, x: f32) -> f32 {
        self.coeffs
            .iter()
            .enumerate()
            .map(|(d, coeff)| coeff.as_f32() * x.powi(d as i32))
            .sum()
    }
}

impl Gen for Polynomial {
    fn gen<R: Rng>(rng: &mut R) -> Self {
        Self::new(
            (0..=3)
                .map(|_| ClampedF32::new(rng.gen_range(-5.0..=5.0)))
                .collect(),
        )
    }
}

impl Crossover for Polynomial {
    fn crossover<R: Rng>(&self, other: &Self, rng: &mut R) -> Self {
        let min = self.coeffs.len().min(other.coeffs.len());
        let max = self.coeffs.len().max(other.coeffs.len());
        let len = (min..=max).choose(rng).unwrap();

        Self::new(
            (0..len)
                .map(|i| match (self.coeffs.get(i), other.coeffs.get(i)) {
                    (None, None) => unreachable!(),
                    (None, Some(only)) | (Some(only), None) => *only,
                    (Some(a), Some(b)) => match rng.gen_range(0..3) {
                        0 => *a,
                        1 => *b,
                        2 => ClampedF32::new((a.as_f32() + b.as_f32()) / 2.0),
                        _ => unreachable!(),
                    },
                })
                .collect(),
        )
    }
}

impl Mutate for Polynomial {
    fn mutate<R: Rng>(&mut self, rate: f32, rng: &mut R) {
        let rp = 1.0 - (1.0 - rate).powf(0.5);
        if rng.gen::<f32>() < rp {
            self.coeffs.push(ClampedF32::new(rng.gen_range(-5.0..=5.0)));
        } else if rng.gen::<f32>() < rp {
            self.coeffs.pop();
        }

        for f in self.coeffs.iter_mut() {
            f.mutate(rate, rng);
        }
    }
}
