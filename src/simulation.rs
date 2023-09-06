pub mod config;
pub mod conflict;

use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use ordered_float::OrderedFloat;
use palette::{Mix, Srgb};
use rand::prelude::*;

use crate::grid::{Grid, GridEnumerator, GridLike};

use self::{
    config::Config,
    conflict::{reduce_potential_moves, PotentialMoves},
};

pub struct State {
    pub elements: Grid<Tile>,
    pub config: Config,
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
                    },
                    element,
                }
            }),
            config,
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
        self.update_positions();
        self.update_saturations();
    }

    fn update_positions(&mut self) {
        let potential_moves = self.potential_moves();
        let moves = reduce_potential_moves(&self.config, potential_moves, &self.elements);
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
                let total: f32 = w.iter().map(|t| t.saturation).sum();
                let count = w.iter().count();
                let avg = total / count as f32;
                let target = avg;
                let diff = target - w.get(0, 0).unwrap().saturation;
                self.config.saturation_diffusion_rate * diff
            })
            .collect();
        let saturations =
            Grid::from_cells(self.elements.width(), self.elements.height(), saturations);

        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            let s = &mut t.saturation;
            *s = (*s + saturations.get(x as isize, y as isize).unwrap()).clamp(0.0, 1.0);
            match t.element {
                Element::Air if *s >= self.config.air_to_water_saturation_threshold => {
                    t.element = Element::Water
                }
                Element::Water if *s < self.config.water_to_air_saturation_threshold => {
                    t.element = Element::Air
                }
                _ => {}
            }
        }
    }

    pub fn potential_moves(&self) -> Grid<PotentialMoves> {
        let potential_move = self
            .elements
            .enumerate()
            .map(|(x, y, t)| {
                let mut moves: Vec<_> = t
                    .phase(&self.config)
                    .allowed_moves()
                    .into_iter()
                    .map(move |(dx, dy)| (x as isize + dx, y as isize + dy))
                    .filter(|&(x, y)| {
                        !(x < 0
                            || y < 0
                            || x as usize >= self.elements.width()
                            || y as usize >= self.elements.height())
                    })
                    .collect();

                moves.sort_unstable_by_key(|(x, y)| {
                    OrderedFloat(position_score(&self.config, &self.elements, t, *x, *y))
                });

                PotentialMoves::new(moves)
            })
            .collect();
        Grid::from_cells(
            self.elements.width(),
            self.elements.height(),
            potential_move,
        )
    }
}

#[derive(Debug, Clone)]
pub struct Tile {
    element: Element,
    saturation: f32,
}

impl Tile {
    fn color(&self) -> Srgb<u8> {
        let air = Srgb::new(221u8, 255, 247)
            .into_format::<f32>()
            .into_linear();
        let water = Srgb::new(46u8, 134, 171).into_format::<f32>().into_linear();
        let soil = Srgb::new(169u8, 113, 75).into_format::<f32>().into_linear();

        let color = match self.element {
            Element::Air => air.mix(water, 0.5 * self.saturation),
            Element::Soil => soil * (1.0 - 0.5 * self.saturation),
            Element::Water => water.mix(air, 1.0 - self.saturation),
        };

        color.into()
    }

    fn phase(&self, config: &Config) -> Phase {
        match self.element {
            Element::Air => Phase::Gas,
            Element::Soil => {
                if self.saturation > config.soil_is_liquid_saturation_threshold {
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
            Element::Air => config.air_density.eval(self.saturation),
            Element::Soil => config.soil_density.eval(self.saturation),
            Element::Water => config.water_density.eval(self.saturation),
        }
    }

    fn cohesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air_cohesion.eval(self.saturation),
            Element::Soil => config.soil_cohesion.eval(self.saturation),
            Element::Water => config.water_cohesion.eval(self.saturation),
        }
    }

    fn adhesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air_adhesion.eval(self.saturation),
            Element::Soil => config.soil_adhesion.eval(self.saturation),
            Element::Water => config.water_adhesion.eval(self.saturation),
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

fn position_score(config: &Config, elements: &Grid<Tile>, t: &Tile, x: isize, y: isize) -> f32 {
    let select_other = |ox: isize, oy: isize| {
        let other = elements.get(ox, oy);
        other
            .map(|other| {
                if t as *const _ == other as *const _ {
                    elements.get(x, y)
                } else {
                    Some(other)
                }
            })
            .flatten()
    };

    let attraction_score = [
        select_other(x - 1, y - 1),
        select_other(x, y - 1),
        select_other(x + 1, y - 1),
        select_other(x - 1, y),
        select_other(x + 1, y),
        select_other(x - 1, y + 1),
        select_other(x, y + 1),
        select_other(x + 1, y + 1),
    ]
    .iter()
    .zip(config.neighbor_attraction_weights)
    .map(|(o, w)| {
        w * match o {
            Some(o) => t.attractive_force(o, config),
            None => t.adhesion(config).powi(2),
        }
    })
    .into_iter()
    .sum::<f32>();

    let density_score = [
        select_other(x - 1, y - 1),
        select_other(x, y - 1),
        select_other(x + 1, y - 1),
        select_other(x - 1, y),
        select_other(x + 1, y),
        select_other(x - 1, y + 1),
        select_other(x, y + 1),
        select_other(x + 1, y + 1),
    ]
    .iter()
    .zip(config.neighbor_density_weights)
    .map(|(o, w)| {
        w * match o {
            Some(o) => {
                (o.density(config) - t.density(config)) / (o.density(config) + t.density(config))
            }
            None => 0.0,
        }
    })
    .into_iter()
    .sum::<f32>();

    config.attraction_score_weight * attraction_score + config.density_score_weight * density_score
}

#[derive(Debug, Clone, Copy, Ordinalize, PartialEq, Eq)]
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
    fn allowed_moves(&self) -> Vec<(isize, isize)> {
        match self {
            Phase::Solid => vec![(0, -1), (0, 0), (-1, 1), (0, 1), (1, 1)],
            Phase::Liquid | Phase::Gas => vec![
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (0, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ],
        }
    }
}
