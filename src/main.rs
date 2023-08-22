use std::{error::Error, sync::mpsc::TryRecvError};

use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use rand::Rng;
use show_image::{
    create_window,
    event::{VirtualKeyCode, WindowEvent},
    WindowOptions,
};

#[show_image::main]
fn main() -> Result<(), Box<dyn Error>> {
    let mut state = State::gen();
    let mut running = false;

    let window = create_window(
        "",
        WindowOptions {
            size: Some([512, 512]),
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
        let new = (0..self.elements.height())
            .flat_map(|y| (0..self.elements.width()).map(move |x| (x as isize, y as isize)))
            .map(|(x, y)| {
                let t = self.elements.get(x, y).unwrap();
                match t.element {
                    Element::Air => match self.elements.get(x, y - 1) {
                        Some(e) => e.clone(),
                        None => t.clone(),
                    },
                    Element::Soil => match self.elements.get(x, y + 1) {
                        Some(e) => e.clone(),
                        None => t.clone(),
                    },
                }
            })
            .collect();

        self.elements = Grid::from_cells(self.elements.width(), self.elements.height(), new);
    }
}

#[derive(Debug, Clone)]
struct Grid<T> {
    width: usize,
    height: usize,
    cells: Vec<T>,
}

impl<T> Grid<T> {
    fn new(width: usize, height: usize, init: impl Fn(usize, usize) -> T + Copy) -> Self {
        Self {
            width,
            height,
            cells: (0..height)
                .flat_map(|y| (0..width).map(move |x| init(x, y)))
                .collect(),
        }
    }

    fn from_cells(width: usize, height: usize, cells: Vec<T>) -> Self {
        Self {
            width,
            height,
            cells,
        }
    }

    fn get(&self, x: isize, y: isize) -> Option<&T> {
        if x < 0 || y < 0 {
            return None;
        }
        self.cells.get(x as usize + self.width * y as usize)
    }

    fn height(&self) -> usize {
        self.height
    }

    fn width(&self) -> usize {
        self.width
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
        }
    }
}

#[derive(Debug, Clone, Ordinalize)]
enum Element {
    Air,
    Soil,
}
