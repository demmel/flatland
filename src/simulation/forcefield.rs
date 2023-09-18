use std::cmp::Reverse;

use image::{Rgb, RgbImage};
use nalgebra::{Rotation2, Vector2};
use ordered_float::OrderedFloat;
use palette::{FromColor, Srgb};
use rand::Rng;

use crate::{
    grid::{Grid, GridEnumerator, GridLike},
    pageflip::PageFlip,
};

use super::{config::Config, conflict::PotentialMoves, Tile};

#[derive(Debug, Clone)]
pub struct ForceField(PageFlip<Grid<Vector2<f32>>>);

impl ForceField {
    pub fn new(width: usize, height: usize) -> Self {
        Self(PageFlip::new(|| {
            Grid::new(width, height, |_, _| Vector2::new(0.0, 0.0))
        }))
    }

    pub fn init(&mut self, config: &Config, elements: &Grid<Tile>) {
        for (x, y) in GridEnumerator::new(elements) {
            let t = elements.get(x as isize, y as isize).unwrap();
            let mut force = 9.0f32 * Vector2::y();
            for i in -2..=2 {
                let edges = [(i, -2), (-2, i), (i, 2), (2, i)];
                for (dx, dy) in edges {
                    let d = Vector2::new(dx as f32, dy as f32).normalize();
                    if let Some(ot) = elements.get(x as isize + dx, y as isize + dy) {
                        force += t.attractive_force(ot, config) * d;
                    }
                }
            }
            *self.0.write().get_mut(x as isize, y as isize).unwrap() = force;
        }
        self.0.flip();
    }

    pub fn update<R: Rng>(&mut self, elements: &Grid<Tile>, config: &Config, rng: &mut R) {
        for (x, y) in GridEnumerator::new(self.0.read()) {
            let t = elements.get(x as isize, y as isize).unwrap();
            let mut f = *self.0.read().get(x as isize, y as isize).unwrap();
            let mut max_x = OrderedFloat(f.x);
            let mut max_y = OrderedFloat(f.y);
            let mut min_x = OrderedFloat(f.x);
            let mut min_y = OrderedFloat(f.y);
            for (dx, dy) in (-1..=1).flat_map(|y| (-1..=1).map(move |x| (x, y))) {
                if dx != 0 || dy != 0 {
                    if let Some(of) = self.0.read().get(x as isize + dx, y as isize + dy) {
                        let o = elements.get(x as isize + dx, y as isize + dy).unwrap();
                        let d = -Vector2::new(dx as f32, dy as f32).normalize();
                        let scale = 0.25 * (of * o.density(config) / t.density(config)).dot(&d);
                        if scale > 0.0 {
                            let scaled_d = d * scale;
                            f += scaled_d;
                            max_x = max_x.max(OrderedFloat(scaled_d.x));
                            max_y = max_y.max(OrderedFloat(scaled_d.y));
                            min_x = min_x.min(OrderedFloat(scaled_d.x));
                            min_y = min_y.min(OrderedFloat(scaled_d.y));
                        }
                    }
                }
            }

            f.x = f.x.clamp(min_x.0, max_x.0);
            f.y = f.y.clamp(min_y.0, max_y.0);

            let (dx, dy) = (-1..=1)
                .flat_map(|y| (-1..=1).map(move |x| (x, y)))
                .max_by_key(|(dx, dy)| {
                    OrderedFloat(
                        f.dot(
                            &Vector2::new(*dx as f32, *dy as f32)
                                .try_normalize(f32::EPSILON)
                                .unwrap_or(Vector2::zeros()),
                        ),
                    )
                })
                .unwrap();

            if elements.get(x as isize + dx, y as isize + dy).is_none() {
                f = Rotation2::new(
                    rng.gen_range(-std::f32::consts::FRAC_PI_2..=std::f32::consts::FRAC_PI_2),
                ) * -f;
            }

            *self.0.write().get_mut(x as isize, y as isize).unwrap() = f;
        }
        self.0.flip();
    }

    pub fn get(&self, x: isize, y: isize) -> Option<&Vector2<f32>> {
        self.0.read().get(x, y)
    }

    pub fn potential_moves(&self) -> Grid<PotentialMoves> {
        Grid::new(self.0.read().width(), self.0.read().height(), |x, y| {
            let mut moves: Vec<_> = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .filter(|(dx, dy)| {
                    let x = x as isize + dx;
                    let y = y as isize + dy;
                    !(x < 0
                        || y < 0
                        || x as usize >= self.0.read().width()
                        || y as usize >= self.0.read().height())
                })
                .collect();

            let f = self.0.read().get(x as isize, y as isize).unwrap();
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
        })
    }

    fn to_image(&self) -> RgbImage {
        let mut img = RgbImage::new(self.0.read().width() as u32, self.0.read().height() as u32);

        let max_force = self
            .0
            .read()
            .iter()
            .map(|f| OrderedFloat(f.norm()))
            .max()
            .unwrap()
            .0;

        for (x, y, p) in img.enumerate_pixels_mut() {
            let f = self.0.read().get(x as isize, y as isize).unwrap();
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
            let brightness = f.norm() / max_force;

            let hsv = palette::Hsv::new_srgb(hue, 1.0, brightness);

            let p_color = Srgb::from_color(hsv).into_format::<u8>();
            *p = Rgb([p_color.red, p_color.green, p_color.blue])
        }

        img
    }
}
