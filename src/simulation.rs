pub(crate) mod conflict;

use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use ordered_float::OrderedFloat;
use palette::{Mix, Srgb};
use rand::{seq::SliceRandom, Rng};

use crate::grid::{Grid, GridEnumerator, GridLike, GridWindow};

use self::conflict::{reduce_potential_moves, PotentialMoves};

pub(crate) struct State {
    pub(crate) elements: Grid<Tile>,
}

impl State {
    pub(crate) fn gen(width: usize, height: usize) -> Self {
        let mut ret = Self {
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
                    integrity: 0.0001,
                }
            }),
        };

        ret.update_integrities();

        ret
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
        self.update_integrities();
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

    fn update_integrities(&mut self) {
        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            t.integrity = match t.element {
                Element::Air => 0.1 + 0.1 * t.saturation,
                Element::Soil => 0.3 * (0.25 - (t.saturation - 0.5).powi(2)) / 0.25,
                Element::Water => 0.2 + 0.1 * t.saturation,
            }
        }

        for (x, y) in GridEnumerator::new(&self.elements).rev() {
            let (x, y) = (x as isize, y as isize);

            let integrity = {
                let t = self.elements.get(x, y).unwrap();
                [
                    (self.elements.get(x - 1, y - 1), 1.0 / 16.0),
                    (self.elements.get(x, y - 1), 3.0 / 16.0),
                    (self.elements.get(x + 1, y - 1), 1.0 / 16.0),
                    (self.elements.get(x - 1, y), 3.0 / 16.0),
                    (self.elements.get(x + 1, y), 3.0 / 16.0),
                    (self.elements.get(x - 1, y + 1), 1.0 / 16.0),
                    (self.elements.get(x, y + 1), 3.0 / 16.0),
                    (self.elements.get(x + 1, y + 1), 1.0 / 16.0),
                ]
                .map(|(o, w)| {
                    w * match o {
                        Some(o) => o.integrity * if o.element == t.element { 1.0 } else { 0.5 },
                        None => 0.1,
                    }
                })
                .into_iter()
                .sum::<f32>()
                .clamp(0.0, 1.0)
            };

            self.elements.get_mut(x, y).unwrap().integrity = integrity;
        }
    }

    pub(crate) fn potential_moves(&self) -> Grid<PotentialMoves> {
        let mut rng = rand::thread_rng();
        let intended_movements = self
            .elements
            .windows(3)
            .map(|w| {
                let ref this = w.get(0, 0).unwrap();

                fn filter_and_shuffle<F: Fn(&Tile) -> bool>(
                    rng: &mut rand::rngs::ThreadRng,
                    w: &GridWindow<'_, Tile, Grid<Tile>>,
                    candidates: &mut Vec<(isize, isize)>,
                    check: F,
                ) {
                    candidates.retain(|(x, y)| w.get(*x, *y).map(|t| check(t)).unwrap_or(false));
                    candidates.shuffle(rng);
                }

                let candidates = match this.phase() {
                    Phase::Solid => {
                        let candidates: Vec<(Vec<_>, Box<dyn Fn(&Tile) -> bool>)> = vec![(
                            vec![(0, 1)],
                            Box::new(|o: &Tile| {
                                !(-1..=1).any(|x| match w.get(x, 1) {
                                    Some(below) => match below.phase() {
                                        Phase::Solid => true,
                                        Phase::Liquid => false,
                                        Phase::Gas => false,
                                    },
                                    None => true,
                                }) && o.density() < this.density()
                            }),
                        )];
                        candidates
                    }
                    Phase::Liquid => {
                        let candidates: Vec<(Vec<_>, Box<dyn Fn(&Tile) -> bool>)> = vec![
                            (
                                vec![(0, -1)],
                                Box::new(|o: &Tile| o.density() > this.density()),
                            ),
                            (
                                vec![(0, 1)],
                                Box::new(|o: &Tile| o.density() < this.density()),
                            ),
                            (
                                vec![(-1, 1), (1, 1)],
                                Box::new(|o: &Tile| o.density() < this.density()),
                            ),
                            (
                                vec![(-1, 0), (1, 0)],
                                Box::new(|o: &Tile| o.density() < this.density()),
                            ),
                        ];
                        candidates
                    }

                    Phase::Gas => {
                        let candidates: Vec<(Vec<_>, Box<dyn Fn(&Tile) -> bool>)> = vec![
                            (
                                vec![(0, -1)],
                                Box::new(|o: &Tile| o.density() > this.density()),
                            ),
                            (
                                vec![(-1, -1), (1, -1)],
                                Box::new(|o: &Tile| o.density() > this.density()),
                            ),
                            (
                                vec![(0, 1)],
                                Box::new(|o: &Tile| o.density() < this.density()),
                            ),
                            (
                                vec![(-1, 1), (1, 1)],
                                Box::new(|o: &Tile| o.density() < this.density()),
                            ),
                        ];
                        candidates
                    }
                };

                let potential_moves: Vec<_> = candidates
                    .into_iter()
                    .flat_map(|(mut tiles, check)| {
                        filter_and_shuffle(&mut rng, &w, &mut tiles, check);
                        tiles
                    })
                    .chain(std::iter::once((0, 0)))
                    .rev()
                    .collect();

                potential_moves
            })
            .collect();
        let intended_movements = Grid::from_cells(
            self.elements.width(),
            self.elements.height(),
            intended_movements,
        );
        Grid::new(
            intended_movements.width(),
            intended_movements.height(),
            |x, y| {
                PotentialMoves::new(
                    intended_movements
                        .get(x as isize, y as isize)
                        .unwrap()
                        .iter()
                        .map(|(dx, dy)| (x as isize + dx, y as isize + dy))
                        .collect(),
                )
            },
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Tile {
    element: Element,
    integrity: f32,
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
}

#[derive(Debug, Clone, Ordinalize, PartialEq, Eq)]
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
