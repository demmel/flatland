use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use palette::{Mix, Srgb};
use rand::{seq::SliceRandom, Rng};

use crate::grid::{Grid, GridEnumerator, GridLike, GridWindow};

pub struct State {
    elements: Grid<Tile>,
}

impl State {
    pub fn gen(width: usize, height: usize) -> Self {
        let mut ret = Self {
            elements: Grid::new(width, height, |_, _| {
                let mut rng = rand::thread_rng();
                let element = unsafe {
                    Element::from_ordinal_unsafe(rng.gen_range(0..Element::variant_count() as i8))
                };
                Tile {
                    saturation: match element {
                        Element::Air => rng.gen_range(0.1..=0.9),
                        Element::Soil => 0.0,
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
        let mut intended_movements = self.intended_movements();

        let mut iters = 0;
        let resolutions = loop {
            iters += 1;
            let mut conflicts = find_conflicts(&intended_movements);
            let found_conflicts =
                resolve_conflicts(&self.elements, &mut conflicts, &mut intended_movements);
            if !found_conflicts {
                break conflicts;
            } else {
                continue;
            }
        };
        println!("Conflict iters: {iters}");

        self.elements =
            Grid::new(
                self.elements.width(),
                self.elements.height(),
                |x, y| match resolutions.get(x as isize, y as isize).unwrap() {
                    MoveConflict::Resolved((old_x, old_y)) => {
                        self.elements.get(*old_x, *old_y).unwrap().clone()
                    }
                    MoveConflict::None => {
                        panic!("No cells should be empty after conflict resolution")
                    }
                    MoveConflict::Conflict(_) => {
                        panic!("No conflicts should remain after conflict resolution")
                    }
                },
            );

        self.update_integrities();
    }

    fn update_integrities(&mut self) {
        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            t.integrity = match t.element {
                Element::Air => 0.1,
                Element::Soil => 0.3,
                Element::Water => 0.2,
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
                        Some(o) => o.integrity * if o.element == t.element { 1.0 } else { 0.1 },
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

    fn intended_movements(&self) -> Grid<(isize, isize)> {
        let mut rng = rand::thread_rng();
        let intended_movements = self
            .elements
            .windows(3)
            .map(|w| {
                let ref this = w.get(0, 0).unwrap();
                let (mut dx, mut dy): (isize, isize) = (0, 0);

                match this.phase() {
                    Phase::Solid => {
                        if !(-1..=1).any(|x| match w.get(x, 1) {
                            Some(below) => match below.phase() {
                                Phase::Solid => true,
                                Phase::Liquid => false,
                                Phase::Gas => false,
                            },
                            None => true,
                        }) && w.get(0, 1).unwrap().weight() < this.weight()
                        {
                            dy += 1;
                        }
                    }
                    Phase::Liquid => {
                        let above = w.get(0, -1);
                        if let Some(above) = above {
                            if this.weight() < above.weight() {
                                dy += -1;
                            }
                        }
                        for x in -1..=1 {
                            let below = w.get(x, 1);
                            if let Some(below) = below {
                                if this.weight() > below.weight() {
                                    dx += x;
                                    dy += 1;
                                }
                            }
                            let level = w.get(x, 0);
                            if let Some(level) = level {
                                if this.weight() > level.weight() {
                                    dx += if rng.gen() { x } else { 0 };
                                }
                            }
                        }
                    }
                    Phase::Gas => {
                        for x in -1..=1 {
                            let above = w.get(x, -1);
                            if let Some(above) = above {
                                if this.weight() < above.weight() {
                                    dx += x;
                                    dy += -1;
                                }
                            }
                            let below = w.get(x, 1);
                            if let Some(below) = below {
                                if this.weight() > below.weight() {
                                    dx += x;
                                    dy += 1;
                                }
                            }
                        }
                    }
                }

                (
                    if dy.abs() >= 2 * dx.abs() {
                        0
                    } else {
                        dx.clamp(-1, 1)
                    },
                    if dx.abs() >= 2 * dy.abs() {
                        0
                    } else {
                        dy.clamp(-1, 1)
                    },
                )
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
                let (dx, dy) = intended_movements.get(x as isize, y as isize).unwrap();
                (x as isize + dx, y as isize + dy)
            },
        )
    }
}

fn find_conflicts(intended_movements: &Grid<(isize, isize)>) -> Grid<MoveConflict> {
    let mut conflicts = Grid::new(
        intended_movements.width(),
        intended_movements.height(),
        |_x, _y| MoveConflict::None,
    );
    for (x, y, (new_x, new_y)) in intended_movements.enumerate() {
        conflicts
            .get_mut(*new_x, *new_y)
            .unwrap()
            .push_move((x as isize, y as isize));
    }
    conflicts
}

fn resolve_conflicts(
    elements: &Grid<Tile>,
    conflicts: &mut Grid<MoveConflict>,
    intended_movements: &mut Grid<(isize, isize)>,
) -> bool {
    let mut found = false;
    let mut rng = rand::thread_rng();

    for (x, y) in GridEnumerator::new(conflicts) {
        let c = conflicts.get_mut(x as isize, y as isize).unwrap();
        if let MoveConflict::Conflict(candidates) = c {
            found = true;

            let mut slots = candidates.clone();
            slots.retain(|(sx, sy)| *sx != x as isize || *sy != y as isize);
            slots.shuffle(&mut rng);

            candidates.sort_unstable_by(|(ax, ay), (bx, by)| {
                elements
                    .get(*ax, *ay)
                    .unwrap()
                    .integrity
                    .partial_cmp(&elements.get(*bx, *by).unwrap().integrity)
                    .unwrap()
            });

            candidates.pop();

            for ((sx, sy), (cx, cy)) in slots.into_iter().zip(candidates) {
                *intended_movements.get_mut(*cx, *cy).unwrap() = (sx, sy);
            }
        }
    }
    found
}

#[derive(Debug, Clone)]
enum MoveConflict {
    None,
    Resolved((isize, isize)),
    Conflict(Vec<(isize, isize)>),
}

impl MoveConflict {
    fn push_move(&mut self, m: (isize, isize)) {
        match self {
            MoveConflict::None => *self = MoveConflict::Resolved(m),
            MoveConflict::Resolved(only) => *self = MoveConflict::Conflict(vec![*only, m]),
            MoveConflict::Conflict(candidates) => candidates.push(m),
        }
    }
}

#[derive(Debug, Clone)]
struct Tile {
    element: Element,
    integrity: f32,
    saturation: f32,
}

impl Tile {
    fn color(&self) -> Srgb<u8> {
        // return Srgb::new(self.integrity, self.integrity, self.integrity).into_format();

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
            Element::Soil => Phase::Solid,
            Element::Water => Phase::Liquid,
        }
    }

    fn weight(&self) -> f32 {
        match self.element {
            Element::Air => 0.1 - 0.1 * self.saturation,
            Element::Soil => 10.0 - 5.0 * self.saturation,
            Element::Water => self.saturation,
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
