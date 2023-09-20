pub mod config;
pub mod conflict;
pub mod forcefield;

use enum_ordinalize::Ordinalize;
use image::{GenericImage, Pixel, Rgb, RgbImage};
use ordered_float::OrderedFloat;
use palette::{convert::IntoColorUnclamped, FromColor, IntoColor, LinSrgb, Mix, Srgb};
use rand::prelude::*;

use crate::grid::{Grid, GridEnumerator, GridLike};

use self::{
    config::Config,
    conflict::{reduce_potential_moves, PotentialMoves},
    forcefield::ForceField,
};

#[derive(Debug, Clone)]
pub struct State {
    pub elements: Grid<Tile>,
    pub config: Config,
    potential_moves: Grid<PotentialMoves>,
    forces: ForceField,
    pub conflict_iters: usize,
}

impl State {
    pub fn gen(config: Config, width: usize, height: usize) -> Self {
        let mut _self = Self {
            elements: Grid::new(width, height, |_, _| {
                let mut rng = rand::thread_rng();
                let element = unsafe {
                    Element::from_ordinal_unsafe(rng.gen_range(0..Element::variant_count() as i8))
                };
                Tile {
                    saturation: match element {
                        Element::Air => rng.gen_range(0.5..=0.75),
                        Element::Soil => rng.gen_range(0.5..=0.9),
                        Element::Water => 1.0,
                    }
                    .into(),
                    element,
                }
            }),
            config,
            potential_moves: Grid::new(width, height, |_, _| PotentialMoves::new(vec![])),
            forces: ForceField::new(width, height),
            conflict_iters: 0,
        };

        _self.forces.init(&_self.config, &_self.elements);

        _self
    }

    pub fn to_image(&self) -> RgbImage {
        let f = self.forces.force_image();
        let pr = self.forces.pressure_image();
        let e = self.element_image();

        let mut img = RgbImage::new(self.elements.width() as u32, self.elements.height() as u32);

        fn image_to_palette(p: &Rgb<u8>) -> LinSrgb<f32> {
            let [r, g, b] = p.0;
            let color = Srgb::new(r, g, b).into_format::<f32>().into_linear();
            color
        }

        for (x, y, p) in img.enumerate_pixels_mut() {
            // let mut color = image_to_palette(e.get_pixel(x, y));
            // let mut color = image_to_palette(pr.get_pixel(x, y));
            let mut color = image_to_palette(f.get_pixel(x, y));
            // color = color.mix(image_to_palette(pr.get_pixel(x, y)), 0.75);
            // color = color.mix(image_to_palette(f.get_pixel(x, y)), 0.25);
            let color = Srgb::from_color(color).into_format::<u8>();
            *p = Rgb([color.red, color.green, color.blue])
        }

        img
    }

    fn element_image(&self) -> RgbImage {
        let mut img = RgbImage::new(self.elements.width() as u32, self.elements.height() as u32);

        for (x, y, p) in img.enumerate_pixels_mut() {
            let t = self
                .elements
                .get(x as isize, y as isize)
                .expect("Image made from grid should have same size");
            let p_color = t.color();
            *p = Rgb([p_color.red, p_color.green, p_color.blue])
        }

        img
    }

    pub fn update(&mut self) {
        let start = std::time::Instant::now();
        self.update_position();
        self.update_saturations();
        self.update_elements();
        let elapsed = std::time::Instant::now() - start;
        // println!("Update: {}s", elapsed.as_secs_f32());
    }

    fn update_position(&mut self) {
        // self.forces.update(&self.elements, &self.config, &mut rng);

        self.forces.init(&self.config, &self.elements);
        for _ in 0..10 {
            self.forces.update(&self.elements, &self.config);
        }
        self.potential_moves = self.forces.potential_moves();

        let (moves, conflict_iters) =
            reduce_potential_moves(&self.forces, &mut self.potential_moves);
        self.conflict_iters = conflict_iters;
        self.elements = Grid::new(self.elements.width(), self.elements.height(), |x, y| {
            let (old_x, old_y) = moves.get(x as isize, y as isize).unwrap();
            self.elements.get(*old_x, *old_y).unwrap().clone()
        });
    }

    fn update_saturations(&mut self) {
        let saturations = self
            .elements
            .windows(3)
            .map(|w| {
                let total: f32 = w.iter().map(|t| t.saturation.0).sum();
                let count = w.iter().count();
                let avg = total / count as f32;
                let target = avg;
                let diff = target - w.get(0, 0).unwrap().saturation.0;
                self.config.saturation_diffusion_rate.as_f32() * diff
            })
            .collect();
        let saturations =
            Grid::from_cells(self.elements.width(), self.elements.height(), saturations);

        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            let s = &mut t.saturation;
            s.0 = (s.0 + saturations.get(x as isize, y as isize).unwrap()).clamp(0.0, 1.0);
        }
    }

    fn update_elements(&mut self) {
        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            match t.element {
                Element::Air
                    if t.saturation().0
                        >= self.config.air_to_water_saturation_threshold.as_f32() =>
                {
                    t.element = Element::Water
                }
                Element::Water
                    if t.saturation().0
                        < self.config.water_to_air_saturation_threshold.as_f32() =>
                {
                    t.element = Element::Air
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tile {
    element: Element,
    saturation: OrderedFloat<f32>,
}

impl Tile {
    fn color(&self) -> Srgb<u8> {
        let air = Srgb::new(221u8, 255, 247)
            .into_format::<f32>()
            .into_linear();
        let water = Srgb::new(46u8, 134, 171).into_format::<f32>().into_linear();
        let soil = Srgb::new(169u8, 113, 75).into_format::<f32>().into_linear();

        let color = match self.element {
            Element::Air => air.mix(water, 0.5 * self.saturation.0),
            Element::Soil => soil * (1.0 - 0.5 * self.saturation.0),
            Element::Water => water.mix(air, 1.0 - self.saturation.0),
        };

        color.into()
    }

    fn density(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air.density.eval(self.saturation.0),
            Element::Soil => config.soil.density.eval(self.saturation.0),
            Element::Water => config.water.density.eval(self.saturation.0),
        }
    }

    fn cohesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air.cohesion.eval(self.saturation.0),
            Element::Soil => config.soil.cohesion.eval(self.saturation.0),
            Element::Water => config.water.cohesion.eval(self.saturation.0),
        }
    }

    fn adhesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air.adhesion.eval(self.saturation.0),
            Element::Soil => config.soil.adhesion.eval(self.saturation.0),
            Element::Water => config.water.adhesion.eval(self.saturation.0),
        }
    }

    fn attractive_force(&self, other: &Self, config: &Config) -> f32 {
        if self.element == other.element {
            self.cohesion(config) * other.cohesion(config)
        } else {
            self.adhesion(config) * other.adhesion(config)
        }
    }

    pub fn element(&self) -> Element {
        self.element
    }

    pub fn saturation(&self) -> OrderedFloat<f32> {
        self.saturation
    }
}

#[derive(Debug, Clone, Copy, Ordinalize, PartialEq, Eq, Hash)]
pub enum Element {
    Air,
    Soil,
    Water,
}
