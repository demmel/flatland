pub mod config;
pub mod conflict;
mod score;

use std::cmp::Reverse;

use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use ordered_float::OrderedFloat;
use palette::{Mix, Srgb};
use rand::prelude::*;

use crate::grid::{Grid, GridEnumerator, GridLike};

use self::{
    config::Config,
    conflict::{reduce_potential_moves, PotentialMoves},
    score::{PairwiseTileScorer, WINDOW_SIZE_OV_2},
};

#[derive(Debug, Clone)]
pub struct State {
    pub elements: Grid<Tile>,
    pub config: Config,
    potential_moves: Grid<PotentialMoves>,
    pub conflict_iters: usize,
}

impl State {
    pub fn gen(config: Config, width: usize, height: usize) -> Self {
        Self {
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
            conflict_iters: 0,
        }
    }

    pub fn to_image(&self) -> RgbImage {
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
        self.update_positions();
        self.update_saturations();
        let elapsed = std::time::Instant::now() - start;
        // println!("Update: {}s", elapsed.as_secs_f32());
    }

    fn update_positions(&mut self) {
        let mut scorer = PairwiseTileScorer::new(self);

        for (x, y) in GridEnumerator::new(&self.elements) {
            self.update_potential_moves(x, y, &scorer);
        }

        let (moves, conflict_iters) =
            reduce_potential_moves(&mut scorer, &self.config, &mut self.potential_moves);
        self.conflict_iters = conflict_iters;
        self.elements = Grid::new(self.elements.width(), self.elements.height(), |x, y| {
            let (old_x, old_y) = moves.get(x as isize, y as isize).unwrap();
            self.elements.get(*old_x, *old_y).unwrap().clone()
        });
    }

    fn update_potential_moves(&mut self, x: usize, y: usize, scorer: &PairwiseTileScorer) {
        let mut moves: Vec<_> = (-(WINDOW_SIZE_OV_2 - 1)..=(WINDOW_SIZE_OV_2 - 1))
            .flat_map(|dy| {
                (-(WINDOW_SIZE_OV_2 - 1)..=(WINDOW_SIZE_OV_2 - 1)).map(move |dx| (dx, dy))
            })
            .map(|(dx, dy)| (x as isize + dx, y as isize + dy))
            .filter(|(x, y)| {
                !(*x < 0
                    || *y < 0
                    || *x as usize >= self.elements.width()
                    || *y as usize >= self.elements.height())
            })
            .collect();

        moves.sort_unstable_by_key(|(mx, my)| {
            Reverse(OrderedFloat(scorer.position_score(
                &self.config,
                x as isize,
                y as isize,
                *mx,
                *my,
            )))
        });

        *self
            .potential_moves
            .get_mut(x as isize, y as isize)
            .unwrap() = PotentialMoves::new(moves);
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
                self.config.saturation_diffusion_rate.0 * diff
            })
            .collect();
        let saturations =
            Grid::from_cells(self.elements.width(), self.elements.height(), saturations);

        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            let s = &mut t.saturation;
            s.0 = (s.0 + saturations.get(x as isize, y as isize).unwrap()).clamp(0.0, 1.0);
            match t.element {
                Element::Air if s.0 >= self.config.air_to_water_saturation_threshold.0 => {
                    t.element = Element::Water
                }
                Element::Water if s.0 < self.config.water_to_air_saturation_threshold.0 => {
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
            Element::Air => config.air_density.eval(self.saturation.0),
            Element::Soil => config.soil_density.eval(self.saturation.0),
            Element::Water => config.water_density.eval(self.saturation.0),
        }
    }

    fn cohesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air_cohesion.eval(self.saturation.0),
            Element::Soil => config.soil_cohesion.eval(self.saturation.0),
            Element::Water => config.water_cohesion.eval(self.saturation.0),
        }
    }

    fn adhesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air_adhesion.eval(self.saturation.0),
            Element::Soil => config.soil_adhesion.eval(self.saturation.0),
            Element::Water => config.water_adhesion.eval(self.saturation.0),
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
