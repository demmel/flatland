pub(crate) struct Config {
    pub(crate) air_adhesion: Polynomial,
    pub(crate) air_cohesion: Polynomial,
    pub(crate) air_density: Polynomial,
    pub(crate) air_to_water_saturation_threshold: f32,
    pub(crate) saturation_diffusion_rate: f32,
    pub(crate) soil_adhesion: Polynomial,
    pub(crate) soil_cohesion: Polynomial,
    pub(crate) soil_density: Polynomial,
    pub(crate) soil_is_liquid_saturation_threshold: f32,
    pub(crate) water_adhesion: Polynomial,
    pub(crate) water_cohesion: Polynomial,
    pub(crate) water_density: Polynomial,
    pub(crate) water_to_air_saturation_threshold: f32,
    pub(crate) neighbor_attraction_weights: [f32; 8],
    pub(crate) neighbor_density_weights: [f32; 8],
    pub(crate) attraction_score_weight: f32,
    pub(crate) density_score_weight: f32,
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

pub(crate) struct Polynomial {
    coeffs: Vec<f32>,
}

impl Polynomial {
    fn new(coeffs: Vec<f32>) -> Self {
        Self { coeffs }
    }

    pub(crate) fn eval(&self, x: f32) -> f32 {
        self.coeffs
            .iter()
            .enumerate()
            .map(|(d, coeff)| coeff * x.powi(d as i32))
            .sum()
    }
}
