use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use palette::Srgb;
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
                Tile {
                    element: unsafe {
                        Element::from_ordinal_unsafe(
                            rng.gen_range(0..Element::variant_count() as i8),
                        )
                    },
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

        let resolutions = loop {
            let mut conflicts = find_conflicts(&intended_movements);
            let found_conflicts =
                resolve_conflicts(&self.elements, &mut conflicts, &mut intended_movements);
            if !found_conflicts {
                break conflicts;
            } else {
                continue;
            }
        };

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
        let intended_movements = self
            .elements
            .windows(3)
            .map(|w| w.get(0, 0).unwrap().intended_movement(&w))
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
}

impl Tile {
    fn color(&self) -> Srgb<u8> {
        // return Srgb::new(self.integrity, self.integrity, self.integrity).into_format();

        match self.element {
            Element::Air => Srgb::new(221, 255, 247),
            Element::Soil => Srgb::new(169, 113, 75),
            Element::Water => Srgb::new(46, 134, 171),
        }
    }

    fn intended_movement(&self, neighbors: &GridWindow<'_, Tile, Grid<Tile>>) -> (isize, isize) {
        use Element::*;

        let mut rng = rand::thread_rng();

        let adjacent = (
            (
                neighbors.get(-1, -1).map(|t| &t.element),
                neighbors.get(0, -1).map(|t| &t.element),
                neighbors.get(1, -1).map(|t| &t.element),
            ),
            (
                neighbors.get(-1, 0).map(|t| &t.element),
                neighbors.get(1, 0).map(|t| &t.element),
            ),
            (
                neighbors.get(-1, 1).map(|t| &t.element),
                neighbors.get(0, 1).map(|t| &t.element),
                neighbors.get(1, 1).map(|t| &t.element),
            ),
        );

        match self.element {
            Air => match adjacent {
                // Air rises above soil and water
                ((_, Some(Water | Soil), _), (_, _), (_, _, _)) => (0, -1),
                x => {
                    // println!("Unhandled configuration: {x:?}");
                    (0, 0)
                }
            },
            Soil => match adjacent {
                // Soil falls below air and water
                (
                    (_, _, _),
                    (_, _),
                    (None | Some(Air | Water), Some(Water | Air), None | Some(Air | Water)),
                ) => (0, 1),
                // Soil rolls down hill
                (
                    (_, _, _),
                    (Some(Air | Water), Some(Water | Air) | None),
                    (Some(Air | Water), Some(Soil), Some(Soil) | None),
                ) => (-1, 1),
                (
                    (_, _, _),
                    (None | Some(Water | Air), Some(Air | Water)),
                    (None | Some(Soil), Some(Soil), Some(Air | Water)),
                ) => (1, 1),
                (
                    (_, _, _),
                    (Some(Air | Water), Some(Air | Water)),
                    (Some(Air | Water), Some(Soil), Some(Air | Water)),
                ) => (*[-1, 1].choose(&mut rng).unwrap(), 1),
                // Soil can form structure
                ((_, _, _), (_, _), (Some(Soil), _, _)) => (0, 0),
                ((_, _, _), (_, _), (_, _, Some(Soil))) => (0, 0),
                x => {
                    // println!("Unhandled configuration: {x:?}");
                    (0, 0)
                }
            },
            Water => match adjacent {
                // Water rises above soil
                ((_, Some(Soil), _), (_, _), (_, Some(Soil | Water), _)) => (0, -1),
                // Water falls below air
                ((_, Some(Air | Water), _), (_, _), (_, Some(Air), _)) => (0, 1),
                // Waters rolls down hills
                (
                    (_, _, _),
                    (Some(Air), _),
                    (Some(Air), Some(Soil | Water), Some(Soil | Water) | None),
                ) => (-1, 1),
                (
                    (_, _, _),
                    (_, Some(Air)),
                    (None | Some(Soil | Water), Some(Soil | Water), Some(Air)),
                ) => (1, 1),
                ((_, _, _), (Some(Air), Some(Air)), (Some(Air), Some(Soil | Water), Some(Air))) => {
                    (*[-1, 1].choose(&mut rng).unwrap(), 1)
                }
                // Water tries to flatten
                (
                    (_, _, _),
                    (Some(Air), None | Some(Water)),
                    (Some(Soil | Water), Some(Soil | Water), Some(Soil | Water) | None),
                ) => (-1, 0),
                (
                    (_, _, _),
                    (None | Some(Water), Some(Air)),
                    (None | Some(Soil | Water), Some(Soil | Water), Some(Soil | Water)),
                ) => (1, 0),
                (
                    (_, _, _),
                    (Some(Air), Some(Air)),
                    (Some(Water), Some(Soil | Water), Some(Water)),
                ) => (*[-1, 1].choose(&mut rng).unwrap(), 0),
                x => {
                    // println!("Unhandled configuration: {x:?}");
                    (0, 0)
                }
            },
        }
    }
}

#[derive(Debug, Clone, Ordinalize, PartialEq, Eq)]
enum Element {
    Air,
    Soil,
    Water,
}
