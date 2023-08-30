mod grid;

use std::{error::Error, sync::mpsc::TryRecvError};

use enum_ordinalize::Ordinalize;
use grid::{Grid, GridEnumerator, GridLike};
use image::{Rgb, RgbImage};
use rand::Rng;
use show_image::{
    create_window,
    event::{VirtualKeyCode, WindowEvent},
    WindowOptions,
};

#[show_image::main]
fn main() -> Result<(), Box<dyn Error>> {
    let mut state: State = State::gen();
    let mut running: bool = false;

    let window = create_window(
        "",
        WindowOptions {
            size: Some([1024, 1024]),
            ..Default::default()
        },
    )?;
    window.set_image("image", state.to_image())?;

    let window_events = window.event_channel()?;
    loop {
        match window_events.try_recv() {
            Ok(WindowEvent::KeyboardInput(event)) => {
                if !event.input.state.is_pressed() {
                    continue;
                }
                match event.input.key_code {
                    Some(VirtualKeyCode::Escape) => return Ok(()),
                    Some(VirtualKeyCode::Space) if !running => state.update(),
                    Some(VirtualKeyCode::S) => running = !running,
                    _ => continue,
                }
                window.set_image("image", state.to_image())?;
            }
            Err(TryRecvError::Empty) if running => {
                state.update();
                window.set_image("image", state.to_image())?;
            }
            Err(TryRecvError::Disconnected) => return Ok(()),
            _ => continue,
        }
    }
}

struct State {
    elements: Grid<Tile>,
}

impl State {
    fn gen() -> Self {
        Self {
            elements: Grid::new(512, 512, |_, _| {
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

    fn to_image(&self) -> RgbImage {
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

    fn update(&mut self) {
        let mut intended_movements = self.intended_movements();

        let resolutions = loop {
            let mut conflicts = find_conflicts(&intended_movements);
            let found_conflicts = self.resolve_conflicts(&mut conflicts, &mut intended_movements);
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

    fn resolve_conflicts(
        &mut self,
        conflicts: &mut Grid<MoveConflict>,
        intended_movements: &mut Grid<(isize, isize)>,
    ) -> bool {
        let mut found = false;
        for (x, y) in GridEnumerator::new(conflicts) {
            let c = conflicts.get_mut(x as isize, y as isize).unwrap();
            if let MoveConflict::Conflict(candidates) = c {
                found = true;
                candidates.sort_unstable_by_key(|(x, y)| (*y, *x));
                let gravity_slots = candidates.clone();
                candidates.sort_unstable_by_key(|(x, y)| {
                    match self.elements.get(*x, *y).unwrap().element {
                        Element::Air => -1,
                        Element::Soil => 1,
                        Element::Water => 0,
                    }
                });
                for ((sx, sy), (cx, cy)) in gravity_slots.into_iter().zip(candidates) {
                    *intended_movements.get_mut(*cx, *cy).unwrap() = (sx, sy);
                }
            }
        }
        found
    }

    fn intended_movements(&self) -> Grid<(isize, isize)> {
        let intended_movements = self
            .elements
            .windows(3)
            .map(|w| {
                let t = w.get(0, 0).unwrap();

                match t.element {
                    Element::Air => match w.get(0, -1) {
                        Some(above) => match above.element {
                            Element::Air => (0, 0),
                            Element::Soil => (0, -1),
                            Element::Water => (0, -1),
                        },
                        None => (0, 0),
                    },
                    Element::Soil => match w.get(0, 1) {
                        Some(below) => match below.element {
                            Element::Air => (0, 1),
                            Element::Soil => (0, 0),
                            Element::Water => (0, 1),
                        },
                        None => (0, 0),
                    },
                    Element::Water => match (w.get(0, -1), w.get(0, 1)) {
                        (None, None) => (0, 0),
                        (None, Some(below)) => match below.element {
                            Element::Air => (0, 1),
                            Element::Soil => (0, 0),
                            Element::Water => (0, 0),
                        },
                        (Some(above), None) => match above.element {
                            Element::Air => (0, 0),
                            Element::Soil => (0, -1),
                            Element::Water => (0, 0),
                        },
                        (Some(above), Some(below)) => match (&above.element, &below.element) {
                            (Element::Air, Element::Air) => (0, 1),
                            (Element::Air, Element::Soil) => (0, 0),
                            (Element::Air, Element::Water) => (0, 0),
                            (Element::Soil, Element::Air) => (0, 0),
                            (Element::Soil, Element::Soil) => (0, -1),
                            (Element::Soil, Element::Water) => (0, -1),
                            (Element::Water, Element::Air) => (0, 1),
                            (Element::Water, Element::Soil) => (0, 0),
                            (Element::Water, Element::Water) => (0, 0),
                        },
                    },
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
        let n = conflicts.get_mut(*new_x, *new_y).unwrap();
        let old = (x as isize, y as isize);
        match n {
            MoveConflict::None => *n = MoveConflict::Resolved(old),
            MoveConflict::Resolved(only) => *n = MoveConflict::Conflict(vec![*only, old]),
            MoveConflict::Conflict(candidates) => candidates.push(old),
        }
    }
    conflicts
}

#[derive(Debug, Clone)]
enum MoveConflict {
    None,
    Resolved((isize, isize)),
    Conflict(Vec<(isize, isize)>),
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
