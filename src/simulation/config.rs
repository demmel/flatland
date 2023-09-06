use rand::{seq::IteratorRandom, thread_rng, Rng};

#[derive(Debug, Clone)]
pub struct Config {
    pub air_adhesion: Polynomial,
    pub air_cohesion: Polynomial,
    pub air_density: Polynomial,
    pub air_to_water_saturation_threshold: f32,
    pub saturation_diffusion_rate: f32,
    pub soil_adhesion: Polynomial,
    pub soil_cohesion: Polynomial,
    pub soil_density: Polynomial,
    pub soil_is_liquid_saturation_threshold: f32,
    pub water_adhesion: Polynomial,
    pub water_cohesion: Polynomial,
    pub water_density: Polynomial,
    pub water_to_air_saturation_threshold: f32,
    pub neighbor_attraction_weights: [f32; 8],
    pub neighbor_density_weights: [f32; 8],
    pub attraction_score_weight: f32,
    pub density_score_weight: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            air_to_water_saturation_threshold: 0.9,
            air_density: Polynomial::new(vec![0.5, -0.5]),
            saturation_diffusion_rate: 0.01,
            soil_is_liquid_saturation_threshold: 0.9,
            soil_density: Polynomial::new(vec![10.0, -5.0]),
            water_to_air_saturation_threshold: 0.5,
            water_density: Polynomial::new(vec![0.5, 0.5]),
            air_adhesion: Polynomial::new(vec![0.1, 0.05]),
            air_cohesion: Polynomial::new(vec![0.1, 0.4]),
            soil_adhesion: Polynomial::new(vec![0.0, 3.25, -2.5]),
            soil_cohesion: Polynomial::new(vec![0.0, 3.25, -2.5]),
            water_adhesion: Polynomial::new(vec![0.75]),
            water_cohesion: Polynomial::new(vec![0.5]),
            neighbor_attraction_weights: [
                1.0 / 16.0,
                3.0 / 16.0,
                1.0 / 16.0,
                3.0 / 16.0,
                3.0 / 16.0,
                1.0 / 16.0,
                3.0 / 16.0,
                1.0 / 16.0,
            ],
            neighbor_density_weights: [0.0, -0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0],
            attraction_score_weight: 1.0,
            density_score_weight: 1.0,
        }
    }
}

impl Config {
    pub fn gen<R: Rng>(rng: &mut R) -> Self {
        Self {
            air_to_water_saturation_threshold: rng.gen(),
            air_density: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            saturation_diffusion_rate: rng.gen(),
            soil_is_liquid_saturation_threshold: rng.gen(),
            soil_density: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            water_to_air_saturation_threshold: rng.gen(),
            water_density: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            air_adhesion: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            air_cohesion: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            soil_adhesion: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            soil_cohesion: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            water_adhesion: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            water_cohesion: Polynomial::new((0..rng.gen_range(1..3)).map(|_| rng.gen()).collect()),
            neighbor_attraction_weights: [
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
            ],
            neighbor_density_weights: [
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
                rng.gen_range(-1.0..=1.0),
            ],
            attraction_score_weight: rng.gen(),
            density_score_weight: rng.gen(),
        }
    }

    pub fn crossover(&self, other: &Self) -> Self {
        let mut rng = thread_rng();

        Self {
            air_adhesion: self.air_adhesion.crossover(&other.air_adhesion),
            air_cohesion: self.air_cohesion.crossover(&other.air_cohesion),
            air_density: self.air_density.crossover(&other.air_density),
            air_to_water_saturation_threshold: if rng.gen() {
                self.air_to_water_saturation_threshold
            } else {
                other.water_to_air_saturation_threshold
            },
            saturation_diffusion_rate: if rng.gen() {
                self.saturation_diffusion_rate
            } else {
                other.saturation_diffusion_rate
            },
            soil_adhesion: self.soil_adhesion.crossover(&other.soil_adhesion),
            soil_cohesion: self.soil_cohesion.crossover(&other.soil_cohesion),
            soil_density: self.soil_density.crossover(&other.soil_density),
            soil_is_liquid_saturation_threshold: if rng.gen() {
                self.soil_is_liquid_saturation_threshold
            } else {
                other.soil_is_liquid_saturation_threshold
            },
            water_adhesion: self.water_adhesion.crossover(&other.water_adhesion),
            water_cohesion: self.water_cohesion.crossover(&other.water_cohesion),
            water_density: self.water_density.crossover(&other.water_density),
            water_to_air_saturation_threshold: if rng.gen() {
                self.water_to_air_saturation_threshold
            } else {
                other.water_to_air_saturation_threshold
            },
            neighbor_attraction_weights: crossover_f32_arrays(
                &self.neighbor_attraction_weights,
                &other.neighbor_attraction_weights,
            ),
            neighbor_density_weights: crossover_f32_arrays(
                &self.neighbor_density_weights,
                &other.neighbor_density_weights,
            ),
            attraction_score_weight: if rng.gen() {
                self.attraction_score_weight
            } else {
                other.attraction_score_weight
            },
            density_score_weight: if rng.gen() {
                self.density_score_weight
            } else {
                other.density_score_weight
            },
        }
    }
}

fn crossover_f32_arrays(a: &[f32; 8], b: &[f32; 8]) -> [f32; 8] {
    let mut rng = thread_rng();
    let mut res = a.clone();
    for i in 0..res.len() {
        if rng.gen() {
            res[i] = b[i];
        }
    }
    res
}

#[derive(Debug, Clone)]
pub struct Polynomial {
    coeffs: Vec<f32>,
}

impl Polynomial {
    fn new(coeffs: Vec<f32>) -> Self {
        Self { coeffs }
    }

    pub fn eval(&self, x: f32) -> f32 {
        self.coeffs
            .iter()
            .enumerate()
            .map(|(d, coeff)| coeff * x.powi(d as i32))
            .sum()
    }

    pub fn crossover(&self, other: &Self) -> Self {
        let mut rng = thread_rng();

        let min = self.coeffs.len().min(other.coeffs.len());
        let max = self.coeffs.len().max(other.coeffs.len());
        let len = (min..=max).choose(&mut rng).unwrap();

        Self::new(
            (0..len)
                .map(|i| match (self.coeffs.get(i), other.coeffs.get(i)) {
                    (None, None) => unreachable!(),
                    (None, Some(only)) | (Some(only), None) => *only,
                    (Some(a), Some(b)) => {
                        if rng.gen() {
                            *a
                        } else {
                            *b
                        }
                    }
                })
                .collect(),
        )
    }
}