use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use rand::{seq::SliceRandom, Rng};

use crate::grid::{Grid, GridEnumerator, GridLike};

pub struct State {
    elements: Grid<Tile>,
}

impl State {
    pub fn gen(width: usize, height: usize) -> Self {
        Self {
            elements: Grid::new(width, height, |_, _| {
                let mut rng = rand::thread_rng();
                Tile {
                    element: unsafe {
                        Element::from_ordinal_unsafe(
                            rng.gen_range(0..Element::variant_count() as i8),
                        )
                    },
                }
            }),
        }
    }

    pub fn to_image(&self) -> RgbImage {
        let mut img = RgbImage::new(self.elements.width() as u32, self.elements.height() as u32);

        for (x, y, p) in img.enumerate_pixels_mut() {
            let t = self
                .elements
                .get(x as isize, y as isize)
                .expect("Image made from grid should have same size");
            *p = t.color();
        }

        img
    }

    pub fn update(&mut self) {
        let mut intended_movements = self.intended_movements();

        let resolutions = loop {
            let mut conflicts = find_conflicts(&intended_movements);
            let found_conflicts = resolve_conflicts(&mut conflicts, &mut intended_movements);
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
    }

    fn intended_movements(&self) -> Grid<(isize, isize)> {
        use Element::*;

        let mut rng = rand::thread_rng();

        let intended_movements = self
            .elements
            .windows(3)
            .map(|w| {
                match (
                    (
                        w.get(-1, -1).map(|t| &t.element),
                        w.get(0, -1).map(|t| &t.element),
                        w.get(1, -1).map(|t| &t.element),
                    ),
                    (
                        w.get(-1, 0).map(|t| &t.element),
                        w.get(0, 0).map(|t| &t.element),
                        w.get(1, 0).map(|t| &t.element),
                    ),
                    (
                        w.get(-1, 1).map(|t| &t.element),
                        w.get(0, 1).map(|t| &t.element),
                        w.get(1, 1).map(|t| &t.element),
                    ),
                ) {
                    // Air rises above soil and water
                    ((_, Some(Water | Soil), _), (_, Some(Air), _), (_, _, _)) => (0, -1),
                    // Soil falls below air and water
                    ((_, _, _), (_, Some(Soil), _), (_, Some(Water | Air), _)) => (0, 1),
                    // Water rises above soil
                    ((_, Some(Soil), _), (_, Some(Water), _), (_, Some(Soil | Water), _)) => {
                        (0, -1)
                    }
                    // Water falls below air
                    ((_, Some(Air | Water), _), (_, Some(Water), _), (_, Some(Air), _)) => (0, 1),
                    // Waters rolls down hills
                    (
                        (_, _, _),
                        (Some(Air), Some(Water), _),
                        (Some(Air), Some(Soil | Water), Some(Soil | Water) | None),
                    ) => (-1, 1),
                    (
                        (_, _, _),
                        (_, Some(Water), Some(Air)),
                        (None | Some(Soil | Water), Some(Soil | Water), Some(Air)),
                    ) => (1, 1),
                    (
                        (_, _, _),
                        (Some(Air), Some(Water), Some(Air)),
                        (Some(Air), Some(Soil | Water), Some(Air)),
                    ) => (*[-1, 1].choose(&mut rng).unwrap(), 1),
                    (
                        (_, _, _),
                        (Some(Air), Some(Water), Some(Air)),
                        (Some(Water), Some(Soil | Water), Some(Water)),
                    ) => (*[-1, 1].choose(&mut rng).unwrap(), 0),
                    // Water tries to flatten
                    (
                        (_, _, _),
                        (Some(Air), Some(Water), None | Some(Water)),
                        (Some(Water), Some(Soil | Water), Some(Soil | Water) | None),
                    ) => (-1, 0),
                    (
                        (_, _, _),
                        (None | Some(Water), Some(Water), Some(Air)),
                        (None | Some(Soil | Water), Some(Soil | Water), Some(Water)),
                    ) => (1, 0),
                    (
                        (_, _, _),
                        (None | Some(Soil | Air), Some(Air), Some(Water)),
                        (_, Some(Water), Some(Soil | Water)),
                    ) => (1, 0),
                    (
                        (_, _, _),
                        (Some(Water), Some(Air), None | Some(Soil | Air)),
                        (Some(Soil | Water), Some(Water), _),
                    ) => (-1, 0),
                    x => {
                        // println!("Unhandled configuration: {x:?}");
                        (0, 0)
                    }
                }
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
            slots.shuffle(&mut rng);
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
}

impl Tile {
    fn color(&self) -> Rgb<u8> {
        match self.element {
            Element::Air => Rgb([221, 255, 247]),
            Element::Soil => Rgb([169, 113, 75]),
            Element::Water => Rgb([46, 134, 171]),
        }
    }
}

#[derive(Debug, Clone, Ordinalize)]
enum Element {
    Air,
    Soil,
    Water,
}
