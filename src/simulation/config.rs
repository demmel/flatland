use std::ops::RangeInclusive;

use genetic::{Crossover, Mutate};
use ordered_float::{Float, OrderedFloat};
use rand::{seq::IteratorRandom, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Crossover, Mutate)]
pub struct Config {
    pub air: ElementConfig,
    pub soil: ElementConfig,
    pub water: ElementConfig,
    pub air_to_water_saturation_threshold: ClampedF32<0, 1, 1>,
    pub saturation_diffusion_rate: ClampedF32<0, 1, 1>,
    pub water_to_air_saturation_threshold: ClampedF32<0, 1, 1>,
    pub neighbor_attraction_weights: [ClampedF32<-1, 1, 1>; 8],
    pub neighbor_density_weights: [ClampedF32<-1, 1, 1>; 8],
    pub attraction_score_weight: ClampedF32<0, 1, 1>,
    pub density_score_weight: ClampedF32<0, 1, 1>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            air: ElementConfig {
                adhesion: Polynomial::new(vec![ClampedF32::new(0.1), ClampedF32::new(0.05)]),
                cohesion: Polynomial::new(vec![ClampedF32::new(0.1), ClampedF32::new(0.4)]),
                density: Polynomial::new(vec![ClampedF32::new(0.1), ClampedF32::new(-0.1)]),
            },
            soil: ElementConfig {
                adhesion: Polynomial::new(vec![
                    ClampedF32::new(0.0),
                    ClampedF32::new(3.25),
                    ClampedF32::new(-2.5),
                ]),
                cohesion: Polynomial::new(vec![
                    ClampedF32::new(0.0),
                    ClampedF32::new(3.25),
                    ClampedF32::new(-2.5),
                ]),
                density: Polynomial::new(vec![ClampedF32::new(1.0), ClampedF32::new(-0.1)]),
            },
            water: ElementConfig {
                adhesion: Polynomial::new(vec![ClampedF32::new(0.75)]),
                cohesion: Polynomial::new(vec![ClampedF32::new(0.5)]),
                density: Polynomial::new(vec![ClampedF32::new(0.5), ClampedF32::new(0.1)]),
            },
            air_to_water_saturation_threshold: ClampedF32::new(0.9),
            saturation_diffusion_rate: ClampedF32::new(0.01),
            water_to_air_saturation_threshold: ClampedF32::new(0.5),
            neighbor_attraction_weights: [
                ClampedF32::new(1.0 / 16.0),
                ClampedF32::new(3.0 / 16.0),
                ClampedF32::new(1.0 / 16.0),
                ClampedF32::new(3.0 / 16.0),
                ClampedF32::new(3.0 / 16.0),
                ClampedF32::new(1.0 / 16.0),
                ClampedF32::new(3.0 / 16.0),
                ClampedF32::new(1.0 / 16.0),
            ],
            neighbor_density_weights: [
                ClampedF32::new(0.0),
                ClampedF32::new(-0.5),
                ClampedF32::new(0.0),
                ClampedF32::new(0.0),
                ClampedF32::new(0.0),
                ClampedF32::new(0.0),
                ClampedF32::new(0.5),
                ClampedF32::new(0.0),
            ],
            attraction_score_weight: ClampedF32::new(1.0),
            density_score_weight: ClampedF32::new(1.0),
        }
    }
}

impl Config {
    pub fn gen<R: Rng>(rng: &mut R) -> Self {
        Self {
            air: ElementConfig::gen(rng),
            soil: ElementConfig::gen(rng),
            water: ElementConfig::gen(rng),
            air_to_water_saturation_threshold: ClampedF32::gen(rng),
            saturation_diffusion_rate: ClampedF32::gen(rng),
            water_to_air_saturation_threshold: ClampedF32::gen(rng),
            neighbor_attraction_weights: [
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
            ],
            neighbor_density_weights: [
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
                ClampedF32::gen(rng),
            ],
            attraction_score_weight: ClampedF32::gen(rng),
            density_score_weight: ClampedF32::gen(rng),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Crossover, Mutate)]
pub struct ElementConfig {
    pub adhesion: Polynomial,
    pub cohesion: Polynomial,
    pub density: Polynomial,
}

impl ElementConfig {
    fn gen<R: Rng>(rng: &mut R) -> Self {
        Self {
            adhesion: Polynomial::gen(rng, 0..=3),
            cohesion: Polynomial::gen(rng, 0..=3),
            density: Polynomial::gen(rng, 0..=3),
        }
    }
}

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

    pub fn gen<R: Rng>(rng: &mut R) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Polynomial {
    coeffs: Vec<ClampedF32<-5, 5, 1>>,
}

impl Polynomial {
    fn new(coeffs: Vec<ClampedF32<-5, 5, 1>>) -> Self {
        Self { coeffs }
    }

    fn gen<R: Rng>(rng: &mut R, d: RangeInclusive<usize>) -> Self {
        Self::new(
            d.map(|_| ClampedF32::new(rng.gen_range(-5.0..=5.0)))
                .collect(),
        )
    }

    pub fn eval(&self, x: f32) -> f32 {
        self.coeffs
            .iter()
            .enumerate()
            .map(|(d, coeff)| coeff.0 .0 * x.powi(d as i32))
            .sum()
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
                        2 => ClampedF32::new((*a.0 + *b.0) / 2.0),
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
