use std::cmp::Reverse;

use image::{Rgb, RgbImage};
use nalgebra::Vector2;
use ordered_float::OrderedFloat;
use palette::{FromColor, Srgb};

use crate::{
    grid::{ArrayGrid, Grid, GridEnumerator, GridLike},
    pageflip::PageFlip,
};

use super::{config::Config, conflict::PotentialMoves, Tile};

#[derive(Debug, Clone)]
pub struct ForceField {
    forces: PageFlip<Grid<Vector2<f32>>>,
    pressures: PageFlip<Grid<f32>>,
}

impl ForceField {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            forces: PageFlip::new(|| Grid::new(width, height, |_, _| Vector2::new(0.0, 0.0))),
            pressures: PageFlip::new(|| Grid::new(width, height, |_, _| 0.0)),
        }
    }

    pub fn init(&mut self, config: &Config, elements: &Grid<Tile>) {
        for (x, y) in GridEnumerator::new(elements) {
            *self.forces.write().get_mut(x as isize, y as isize).unwrap() =
                cumulative_attractive_forces_on_xy(x as isize, y as isize, elements, config);
        }
        self.forces.flip();
    }

    pub fn update(&mut self, elements: &Grid<Tile>, config: &Config) {
        let other_force_on = {
            let forces = self.forces.read();
            Grid::new(forces.width(), forces.height(), |x, y| {
                let (x, y) = (x as isize, y as isize);
                let t = elements.get(x, y).unwrap();
                let f = *forces.get(x, y).unwrap();
                ArrayGrid::<Vector2<f32>, 3, 3>::new(|dx, dy| {
                    let (dx, dy) = (dx as isize - 1, dy as isize - 1);
                    if dx != 0 || dy != 0 {
                        let (ox, oy) = (x + dx, y + dy);
                        let of = forces
                            .get(ox, oy)
                            .map(|of| {
                                let o = elements.get(ox, oy).unwrap();
                                of * o.density(config) / t.density(config)
                            })
                            .unwrap_or_else(|| -f);
                        let proj_of = project_incoming_force_onto_cell(dx, dy, &of);
                        proj_of
                    } else {
                        Vector2::zeros()
                    }
                })
            })
        };

        for (x, y) in GridEnumerator::new(self.forces.read()) {
            let (x, y) = (x as isize, y as isize);
            *self.pressures.write().get_mut(x, y).unwrap() = pressure_on_xy(x, y, &other_force_on);
        }
        self.pressures.flip();

        for (x, y) in GridEnumerator::new(self.forces.read()) {
            let (x, y) = (x as isize, y as isize);
            *self.forces.write().get_mut(x as isize, y as isize).unwrap() = new_force_for_xy(
                x,
                y,
                self.forces.read(),
                self.pressures.read(),
                &other_force_on,
            );
        }
        self.forces.flip();
    }

    pub fn get(&self, x: isize, y: isize) -> Option<&Vector2<f32>> {
        self.forces.read().get(x, y)
    }

    pub fn potential_moves(&self) -> Grid<PotentialMoves> {
        Grid::new(
            self.forces.read().width(),
            self.forces.read().height(),
            |x, y| {
                let mut moves: Vec<_> = (-1..=1)
                    .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                    .filter(|(dx, dy)| {
                        let x = x as isize + dx;
                        let y = y as isize + dy;
                        !(x < 0
                            || y < 0
                            || x as usize >= self.forces.read().width()
                            || y as usize >= self.forces.read().height())
                    })
                    .collect();

                let f = self.forces.read().get(x as isize, y as isize).unwrap();
                moves.sort_unstable_by_key(|(dx, dy)| {
                    Reverse(OrderedFloat(
                        Vector2::new(*dx as f32, *dy as f32)
                            .try_normalize(f32::EPSILON)
                            .unwrap_or(Vector2::zeros())
                            .dot(f),
                    ))
                });

                PotentialMoves::new(
                    moves
                        .into_iter()
                        .map(|(dx, dy)| (x as isize + dx, y as isize + dy))
                        .collect(),
                )
            },
        )
    }

    pub fn force_image(&self) -> RgbImage {
        let mut img = RgbImage::new(
            self.forces.read().width() as u32,
            self.forces.read().height() as u32,
        );

        let max_force = self
            .forces
            .read()
            .iter()
            .map(|f| OrderedFloat(f.norm()))
            .max()
            .unwrap()
            .0;

        for (x, y, p) in img.enumerate_pixels_mut() {
            let f = self.forces.read().get(x as isize, y as isize).unwrap();
            let up = -Vector2::y(); // Up direction

            // Compute the smallest angle between vectors
            let angle_rad = up.angle(f);

            // To determine if it's clockwise or counter-clockwise, use the determinant (cross product for 2D vectors)
            let det = up.x * f.y - f.x * up.y;

            // If det is negative, the angle is already clockwise. If positive, adjust the angle.
            let final_angle_rad = if det < 0.0 {
                angle_rad
            } else {
                2.0 * std::f32::consts::PI - angle_rad
            };

            let hue = final_angle_rad * 180.0 / std::f32::consts::PI;
            let brightness = f.norm().log10() / max_force.log10();

            let hsv = palette::Hsv::new_srgb(hue, 1.0, brightness);

            let p_color = Srgb::from_color(hsv).into_format::<u8>();

            *p = Rgb([p_color.red, p_color.green, p_color.blue]);
        }

        img
    }

    pub fn pressure_image(&self) -> RgbImage {
        let mut img = RgbImage::new(
            self.forces.read().width() as u32,
            self.forces.read().height() as u32,
        );

        let max_pressure = self
            .pressures
            .read()
            .iter()
            .map(|p| OrderedFloat(*p))
            .max()
            .unwrap()
            .0;

        for (x, y, p) in img.enumerate_pixels_mut() {
            let pr = *self.pressures.read().get(x as isize, y as isize).unwrap() / max_pressure;
            let p_color = Srgb::new(pr, pr, pr).into_format::<u8>();
            *p = Rgb([p_color.red, p_color.green, p_color.blue]);
        }

        img
    }
}

fn new_force_for_xy(
    x: isize,
    y: isize,
    forces: &Grid<Vector2<f32>>,
    pressures: &Grid<f32>,
    other_force_on: &Grid<ArrayGrid<Vector2<f32>, 3, 3>>,
) -> Vector2<f32> {
    let mut f = *forces.get(x, y).unwrap();

    let other_forces_on_xy = other_force_on.get(x, y).unwrap();
    for of in other_forces_on_xy.iter() {
        f += of;
    }

    let (dx, dy, &op) = pressures
        .window_at(3, (x as usize, y as usize))
        .enumerate()
        .min_by_key(|(_dx, _dy, op)| OrderedFloat(**op))
        .unwrap();
    let p = *pressures.get(x, y).unwrap();

    let p_diff = (p - op);
    if p_diff > 0.0 {
        f += p_diff
            * Vector2::new(dx as f32, dy as f32)
                .try_normalize(f32::EPSILON)
                .unwrap_or(Vector2::zeros());
    }

    println!("{y}: {f}");

    f
}

fn pressure_on_xy(x: isize, y: isize, other_force_on: &Grid<ArrayGrid<Vector2<f32>, 3, 3>>) -> f32 {
    let mut pressure = 0.0;

    let other_forces_on_xy = other_force_on.get(x, y).unwrap();
    for (i, of1) in other_forces_on_xy.iter().enumerate() {
        for of2 in other_forces_on_xy.iter().skip(i + 1) {
            let opposition = of1.dot(of2);
            if opposition < 0.0 {
                pressure += (-opposition).sqrt();
            }
        }
    }

    pressure
}

fn cumulative_attractive_forces_on_xy(
    x: isize,
    y: isize,
    elements: &Grid<Tile>,
    config: &Config,
) -> Vector2<f32> {
    let mut force = 10.0f32 * Vector2::y();
    // for d in 1..=2 {
    //     for i in -d..d {
    //         let edges = [(i, -d), (d, i), (d - i, d), (-d, d - i)];
    //         for (dx, dy) in edges {
    //             force += attractive_force_on_xy(x, y, dx, dy, elements, config);
    //         }
    //     }
    // }
    force
}

fn attractive_force_on_xy(
    x: isize,
    y: isize,
    dx: isize,
    dy: isize,
    elements: &Grid<Tile>,
    config: &Config,
) -> Vector2<f32> {
    let t = elements.get(x, y).unwrap();
    let d = Vector2::new(dx as f32, dy as f32).normalize();
    if let Some(ot) = elements.get(x + dx, y + dy) {
        return t.attractive_force(ot, config) * d;
    }
    Vector2::zeros()
}

fn project_incoming_force_onto_cell(dx: isize, dy: isize, of: &Vector2<f32>) -> Vector2<f32> {
    let d = -Vector2::new(dx as f32, dy as f32).normalize();
    let scale = (of).dot(&d);
    if scale < 0.0 {
        scale * d
    } else {
        Vector2::zeros()
    }
}
