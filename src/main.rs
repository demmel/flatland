use std::{error::Error, sync::mpsc::TryRecvError};

use image::{Rgb, RgbImage};
use palette::Srgb;
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
        "Roots",
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
                    water: rng.gen(),
                    air: rng.gen(),
                    mineral: rng.gen(),
                    organic: rng.gen(),
                }
            }),
        }
    }

    fn to_image(&self) -> RgbImage {
        let mut img = RgbImage::new(self.elements.width as u32, self.elements.height as u32);

        let [color_water, color_air, color_mineral, color_organic] = [
            [46u8, 134, 171],
            [221, 255, 247],
            [71, 67, 80],
            [119, 181, 44],
        ]
        .map(|c| Srgb::<u8>::new(c[0], c[1], c[2]).into_linear::<f32>());

        for (x, y, p) in img.enumerate_pixels_mut() {
            let t = self.elements.get(x as usize, y as usize);
            let (r, g, b) = Srgb::<u8>::from_linear(
                (color_water * (t.water as f32 / 255.)
                    + color_air * (t.air as f32 / 255.)
                    + color_mineral * (t.mineral as f32 / 255.)
                    + color_organic * (t.organic as f32 / 255.))
                    / 4.0,
            )
            .into_components();
            *p = Rgb([r, g, b]);
        }

        img
    }

    fn update(&mut self) {
        todo!()
    }
}

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

    fn get(&self, x: usize, y: usize) -> &T {
        &self.cells[x + self.width * y]
    }
}

struct Tile {
    water: u8,
    air: u8,
    mineral: u8,
    organic: u8,
}
