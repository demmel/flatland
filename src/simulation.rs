pub(crate) mod conflict;

use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use ordered_float::OrderedFloat;
use palette::{Mix, Srgb};
use rand::prelude::*;

use crate::grid::{Grid, GridEnumerator, GridLike};

use self::conflict::{reduce_potential_moves, PotentialMoves};

pub(crate) struct State {
    pub(crate) elements: Grid<Tile>,
}

impl State {
    pub(crate) fn gen(width: usize, height: usize) -> Self {
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
        }
    }

    pub(crate) fn to_image(&self) -> RgbImage {
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

    pub(crate) fn update(&mut self) {
        self.update_positions();
        self.update_saturations();
    }

    fn update_positions(&mut self) {
        let potential_moves = self.potential_moves();
        let moves = reduce_potential_moves(potential_moves, &self.elements);
        self.elements = Grid::new(self.elements.width(), self.elements.height(), |x, y| {
            let (old_x, old_y) = moves.get(x as isize, y as isize).unwrap();
            self.elements.get(*old_x, *old_y).unwrap().clone()
        });
    }

    fn update_saturations(&mut self) {
        let mut rng = rand::thread_rng();

        let saturations = self
            .elements
            .windows(3)
            .map(|w| {
                let total: f32 = w.iter().map(|t| t.saturation).sum();
                let count = w.iter().count();
                let avg = total / count as f32;
                let max = w
                    .iter()
                    .map(|t| OrderedFloat(t.saturation))
                    .max()
                    .unwrap()
                    .0;
                let min = w
                    .iter()
                    .map(|t| OrderedFloat(t.saturation))
                    .min()
                    .unwrap()
                    .0;

                let target = match w.get(0, 0).unwrap().element {
                    Element::Air | Element::Soil => rng.gen_range(avg..=max),
                    Element::Water => rng.gen_range(min..=avg),
                };

                let diff = target - w.get(0, 0).unwrap().saturation;
                0.01 * diff
            })
            .collect();
        let saturations =
            Grid::from_cells(self.elements.width(), self.elements.height(), saturations);

        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            let s = &mut t.saturation;
            *s = (*s + saturations.get(x as isize, y as isize).unwrap()).clamp(0.0, 1.0);
            match t.element {
                Element::Air if *s >= 0.9 => t.element = Element::Water,
                Element::Water if *s < 0.5 => t.element = Element::Air,
                _ => {}
            }
        }
    }

    pub(crate) fn potential_moves(&self) -> Grid<PotentialMoves> {
        let potential_move = self
            .elements
            .enumerate()
            .map(|(x, y, t)| {
                let mut moves: Vec<_> = t
                    .phase()
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
                    OrderedFloat(position_score(&self.elements, t, *x, *y))
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
pub(crate) struct Tile {
    element: Element,
    saturation: f32,
}

impl Tile {
    fn color(&self) -> Srgb<u8> {
        // return Srgb::new(self.integrity, 0.0, self.saturation).into_format();

        let air = Srgb::new(221u8, 255, 247)
            .into_format::<f32>()
            .into_linear();
        let water = Srgb::new(46u8, 134, 171).into_format::<f32>().into_linear();
        let soil = Srgb::new(169u8, 113, 75).into_format::<f32>().into_linear();

        let color = match self.element {
            Element::Air => air.mix(water, 0.5 * self.saturation),
            Element::Soil => soil.mix(water, 0.5 * self.saturation),
            Element::Water => water.mix(air, 1.0 - self.saturation),
        };

        color.into()
    }

    fn phase(&self) -> Phase {
        match self.element {
            Element::Air => Phase::Gas,
            Element::Soil => {
                if self.saturation > 0.9 {
                    Phase::Liquid
                } else {
                    Phase::Solid
                }
            }
            Element::Water => Phase::Liquid,
        }
    }

    fn density(&self) -> f32 {
        match self.element {
            Element::Air => 0.5 - 0.5 * self.saturation,
            Element::Soil => 10.0 - 5.0 * self.saturation,
            Element::Water => 0.5 + 0.5 * self.saturation,
        }
    }

    fn cohesion(&self) -> f32 {
        match self.element {
            Element::Air => 0.1 + 0.4 * self.saturation,
            Element::Soil => -2.5 * self.saturation.powi(2) + 3.25 * self.saturation,
            Element::Water => 0.5,
        }
    }

    fn adhesion(&self) -> f32 {
        match self.element {
            Element::Air => 0.1 + 0.65 * self.saturation,
            Element::Soil => self.cohesion(),
            Element::Water => 0.75,
        }
    }

    fn attractive_force(&self, other: &Self) -> f32 {
        if self.element == other.element {
            self.cohesion() * other.cohesion()
        } else {
            self.adhesion() * other.adhesion()
        }
    }
}

fn position_score(elements: &Grid<Tile>, t: &Tile, x: isize, y: isize) -> f32 {
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
        (select_other(x - 1, y - 1), 1.0 / 16.0),
        (select_other(x, y - 1), 3.0 / 16.0),
        (select_other(x + 1, y - 1), 1.0 / 16.0),
        (select_other(x - 1, y), 3.0 / 16.0),
        (select_other(x + 1, y), 3.0 / 16.0),
        (select_other(x - 1, y + 1), 1.0 / 16.0),
        (select_other(x, y + 1), 3.0 / 16.0),
        (select_other(x + 1, y + 1), 1.0 / 16.0),
    ]
    .map(|(o, w)| {
        w * match o {
            Some(o) => t.attractive_force(o),
            None => t.adhesion().powi(2),
        }
    })
    .into_iter()
    .sum::<f32>();

    let density_score = match select_other(x, y + 1) {
        Some(below) => (below.density() - t.density()) / (below.density() + t.density()),
        None => 0.0,
    };

    (attraction_score + density_score) / 2.0
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
