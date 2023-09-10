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
    score::PairwiseTileScorer,
};

pub struct State {
    pub elements: Grid<Tile>,
    pub config: Config,
    potential_moves: Grid<PotentialMoves>,
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
            potential_moves: Grid::new(width, height, |_, _| PotentialMoves::new([None; 9])),
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
        println!("Update: {}s", elapsed.as_secs_f32());
    }

    fn update_positions(&mut self) {
        let mut scorer = PairwiseTileScorer::new(self);

        for (x, y, t) in self.elements.enumerate() {
            let mut moves = t
                .phase(&self.config)
                .allowed_moves()
                .map(move |t| t.map(|(dx, dy)| (x as isize + dx, y as isize + dy)))
                .map(|t| {
                    t.filter(|(x, y)| {
                        !(*x < 0
                            || *y < 0
                            || *x as usize >= self.elements.width()
                            || *y as usize >= self.elements.height())
                    })
                });

            moves.sort_unstable_by_key(|t| {
                Reverse(t.map(|(mx, my)| {
                    OrderedFloat(scorer.position_score(
                        &self.config,
                        x as isize,
                        y as isize,
                        mx,
                        my,
                    ))
                }))
            });

            *self
                .potential_moves
                .get_mut(x as isize, y as isize)
                .unwrap() = PotentialMoves::new(moves);
        }

        let moves = reduce_potential_moves(&mut scorer, &self.config, &mut self.potential_moves);
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
                self.config.saturation_diffusion_rate * diff
            })
            .collect();
        let saturations =
            Grid::from_cells(self.elements.width(), self.elements.height(), saturations);

        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            let s = &mut t.saturation;
            s.0 = (s.0 + saturations.get(x as isize, y as isize).unwrap()).clamp(0.0, 1.0);
            match t.element {
                Element::Air if s.0 >= self.config.air_to_water_saturation_threshold => {
                    t.element = Element::Water
                }
                Element::Water if s.0 < self.config.water_to_air_saturation_threshold => {
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

    fn phase(&self, config: &Config) -> Phase {
        match self.element {
            Element::Air => Phase::Gas,
            Element::Soil => {
                if self.saturation.0 > config.soil_is_liquid_saturation_threshold {
                    Phase::Liquid
                } else {
                    Phase::Solid
                }
            }
            Element::Water => Phase::Liquid,
        }
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
}

#[derive(Debug, Clone, Copy, Ordinalize, PartialEq, Eq, Hash)]
enum Element {
    Air,
    Soil,
    Water,
}

enum Phase {
    Solid,
    Liquid,
    Gas,
}

impl Phase {
    fn allowed_moves(&self) -> [Option<(isize, isize)>; 9] {
        match self {
            Phase::Solid => [
                Some((0, -1)),
                Some((0, 0)),
                Some((-1, 1)),
                Some((0, 1)),
                Some((1, 1)),
                None,
                None,
                None,
                None,
            ],
            Phase::Liquid | Phase::Gas => [
                Some((-1, -1)),
                Some((0, -1)),
                Some((1, -1)),
                Some((-1, 0)),
                Some((0, 0)),
                Some((1, 0)),
                Some((-1, 1)),
                Some((0, 1)),
                Some((1, 1)),
            ],
        }
    }
}
