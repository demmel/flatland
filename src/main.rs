use image::{Rgb, RgbImage};
use rand::Rng;

fn main() {
    let state = State::gen();

    let img = state.to_image();

    img.save("test.png").unwrap();
}

struct State {
    elements: Grid<Element>,
}

impl State {
    fn gen() -> Self {
        Self {
            elements: Grid::new(512, 512, |_, _| {
                let mut rng = rand::thread_rng();
                match rng.gen_range(0..3) {
                    0 => Element::Air,
                    1 => Element::Rock,
                    2 => Element::Soil,
                    _ => unreachable!(),
                }
            }),
        }
    }

    fn to_image(&self) -> RgbImage {
        let mut img = RgbImage::new(self.elements.width as u32, self.elements.height as u32);

        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = match self.elements.get(x as usize, y as usize) {
                Element::Soil => Rgb([234, 140, 85]),
                Element::Rock => Rgb([83, 77, 65]),
                Element::Air => Rgb([124, 198, 254]),
            }
        }

        img
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

enum Element {
    Soil,
    Rock,
    Air,
}
