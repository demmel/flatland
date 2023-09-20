use genetic::{Crossover, Gen, Mutate};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{clamped_f32::ClampedF32, polynomail::Polynomial};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Crossover, Mutate, Gen)]
pub struct Config {
    pub air: ElementConfig,
    pub soil: ElementConfig,
    pub water: ElementConfig,
    pub air_to_water_saturation_threshold: ClampedF32<0, 1, 1>,
    pub saturation_diffusion_rate: ClampedF32<0, 1, 1>,
    pub water_to_air_saturation_threshold: ClampedF32<0, 1, 1>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            air: ElementConfig {
                adhesion: Polynomial::new(vec![ClampedF32::new(0.1), ClampedF32::new(0.05)]),
                cohesion: Polynomial::new(vec![ClampedF32::new(0.1), ClampedF32::new(0.4)]),
                density: Polynomial::new(vec![ClampedF32::new(0.1), ClampedF32::new(-0.99)]),
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Crossover, Mutate, Gen)]
pub struct ElementConfig {
    pub adhesion: Polynomial,
    pub cohesion: Polynomial,
    pub density: Polynomial,
}
