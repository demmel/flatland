mod grid;

use std::{error::Error, sync::mpsc::TryRecvError};

use enum_ordinalize::Ordinalize;
use grid::Grid;
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
                .get(x as usize, y as usize)
                .expect("Image made from grid should have same size");
            *p = t.color();
        }

        img
    }

    fn update(&mut self) {
        let new = self
            .elements
            .windows(3)
            .map(|w| {
                let t = w.get(1, 1).unwrap();

                let element = match t.element {
                    Element::Air => match w.get(1, 0) {
                        Some(other) => match other.element {
                            Element::Air => Element::Air,
                            Element::Soil => Element::Soil,
                            Element::Water => Element::Water,
                        },
                        None => Element::Air,
                    },
                    Element::Soil => match w.get(1, 2) {
                        Some(other) => match other.element {
                            Element::Air => Element::Air,
                            Element::Soil => Element::Soil,
                            Element::Water => Element::Water,
                        },
                        None => Element::Soil,
                    },
                    Element::Water => match (w.get(1, 0), w.get(1, 2)) {
                        (None, None) => Element::Water,
                        (None, Some(below)) => match below.element {
                            Element::Air => Element::Air,
                            Element::Soil => Element::Water,
                            Element::Water => Element::Water,
                        },
                        (Some(above), None) => match above.element {
                            Element::Air => Element::Water,
                            Element::Soil => Element::Soil,
                            Element::Water => Element::Water,
                        },
                        (Some(above), Some(below)) => match (&above.element, &below.element) {
                            (Element::Air, Element::Air) => Element::Air,
                            (Element::Air, Element::Soil) => Element::Water,
                            (Element::Air, Element::Water) => Element::Water,
                            (Element::Soil, Element::Air) => Element::Water,
                            (Element::Soil, Element::Soil) => Element::Soil,
                            (Element::Soil, Element::Water) => Element::Soil,
                            (Element::Water, Element::Air) => Element::Air,
                            (Element::Water, Element::Soil) => Element::Water,
                            (Element::Water, Element::Water) => Element::Water,
                        },
                    },
                };

                Tile { element }
            })
            .collect();
        self.elements = Grid::from_cells(self.elements.width(), self.elements.height(), new);
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
