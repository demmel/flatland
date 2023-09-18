pub mod config;
pub mod conflict;

use std::cmp::Reverse;

use enum_ordinalize::Ordinalize;
use image::{Rgb, RgbImage};
use nalgebra::{Rotation2, Vector2};
use ordered_float::OrderedFloat;
use palette::{FromColor, Mix, Srgb};
use rand::prelude::*;

use crate::{
    grid::{Grid, GridEnumerator, GridLike},
    pageflip::PageFlip,
};

use self::{
    config::Config,
    conflict::{reduce_potential_moves, PotentialMoves},
};

#[derive(Debug, Clone)]
pub struct State {
    pub elements: Grid<Tile>,
    pub config: Config,
    potential_moves: Grid<PotentialMoves>,
    forces: ForceField,
    pub conflict_iters: usize,
}

impl State {
    pub fn gen(config: Config, width: usize, height: usize) -> Self {
        let mut _self = Self {
            elements: Grid::new(width, height, |_, _| {
                let mut rng = rand::thread_rng();
                let element = unsafe {
                    Element::from_ordinal_unsafe(rng.gen_range(0..Element::variant_count() as i8))
                };
                Tile {
                    saturation: match element {
                        Element::Air => rng.gen_range(0.5..=0.75),
                        Element::Soil => rng.gen_range(0.5..=0.9),
                        Element::Water => 1.0,
                    }
                    .into(),
                    element,
                }
            }),
            config,
            potential_moves: Grid::new(width, height, |_, _| PotentialMoves::new(vec![])),
            forces: ForceField::new(width, height),
            conflict_iters: 0,
        };

        _self.forces.init(&_self.config, &_self.elements);

        _self
    }

    pub fn to_image(&self) -> RgbImage {
        // return self.forces.to_image();

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
        let start = std::time::Instant::now();
        self.update_position();
        self.update_saturations();
        self.update_elements();
        let elapsed = std::time::Instant::now() - start;
        // println!("Update: {}s", elapsed.as_secs_f32());
    }

    fn update_position(&mut self) {
        let mut rng = thread_rng();

        // self.forces.update(&self.elements, &self.config, &mut rng);

        self.forces.init(&self.config, &self.elements);
        for _ in 0..10 {
            self.forces.update(&self.elements, &self.config, &mut rng);
        }
        self.potential_moves = self.forces.potential_moves();

        let (moves, conflict_iters) =
            reduce_potential_moves(&self.forces, &mut self.potential_moves);
        self.conflict_iters = conflict_iters;
        self.elements = Grid::new(self.elements.width(), self.elements.height(), |x, y| {
            let (old_x, old_y) = moves.get(x as isize, y as isize).unwrap();
            self.elements.get(*old_x, *old_y).unwrap().clone()
        });
    }

    fn update_saturations(&mut self) {
        let saturations = self
            .elements
            .windows(3)
            .map(|w| {
                let total: f32 = w.iter().map(|t| t.saturation.0).sum();
                let count = w.iter().count();
                let avg = total / count as f32;
                let target = avg;
                let diff = target - w.get(0, 0).unwrap().saturation.0;
                self.config.saturation_diffusion_rate.as_f32() * diff
            })
            .collect();
        let saturations =
            Grid::from_cells(self.elements.width(), self.elements.height(), saturations);

        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            let s = &mut t.saturation;
            s.0 = (s.0 + saturations.get(x as isize, y as isize).unwrap()).clamp(0.0, 1.0);
        }
    }

    fn update_elements(&mut self) {
        for (x, y) in GridEnumerator::new(&self.elements) {
            let t = self.elements.get_mut(x as isize, y as isize).unwrap();
            match t.element {
                Element::Air
                    if t.saturation().0
                        >= self.config.air_to_water_saturation_threshold.as_f32() =>
                {
                    t.element = Element::Water
                }
                Element::Water
                    if t.saturation().0
                        < self.config.water_to_air_saturation_threshold.as_f32() =>
                {
                    t.element = Element::Air
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tile {
    element: Element,
    saturation: OrderedFloat<f32>,
}

impl Tile {
    fn color(&self) -> Srgb<u8> {
        let air = Srgb::new(221u8, 255, 247)
            .into_format::<f32>()
            .into_linear();
        let water = Srgb::new(46u8, 134, 171).into_format::<f32>().into_linear();
        let soil = Srgb::new(169u8, 113, 75).into_format::<f32>().into_linear();

        let color = match self.element {
            Element::Air => air.mix(water, 0.5 * self.saturation.0),
            Element::Soil => soil * (1.0 - 0.5 * self.saturation.0),
            Element::Water => water.mix(air, 1.0 - self.saturation.0),
        };

        color.into()
    }

    fn density(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air.density.eval(self.saturation.0),
            Element::Soil => config.soil.density.eval(self.saturation.0),
            Element::Water => config.water.density.eval(self.saturation.0),
        }
    }

    fn cohesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air.cohesion.eval(self.saturation.0),
            Element::Soil => config.soil.cohesion.eval(self.saturation.0),
            Element::Water => config.water.cohesion.eval(self.saturation.0),
        }
    }

    fn adhesion(&self, config: &Config) -> f32 {
        match self.element {
            Element::Air => config.air.adhesion.eval(self.saturation.0),
            Element::Soil => config.soil.adhesion.eval(self.saturation.0),
            Element::Water => config.water.adhesion.eval(self.saturation.0),
        }
    }

    fn attractive_force(&self, other: &Self, config: &Config) -> f32 {
        if self.element == other.element {
            self.cohesion(config) * other.cohesion(config)
        } else {
            self.adhesion(config) * other.adhesion(config)
        }
    }

    pub fn element(&self) -> Element {
        self.element
    }

    pub fn saturation(&self) -> OrderedFloat<f32> {
        self.saturation
    }
}

#[derive(Debug, Clone, Copy, Ordinalize, PartialEq, Eq, Hash)]
pub enum Element {
    Air,
    Soil,
    Water,
}

#[derive(Debug, Clone)]
pub struct ForceField(PageFlip<Grid<Vector2<f32>>>);

impl ForceField {
    fn new(width: usize, height: usize) -> Self {
        Self(PageFlip::new(|| {
            Grid::new(width, height, |_, _| Vector2::new(0.0, 0.0))
        }))
    }

    fn init(&mut self, config: &Config, elements: &Grid<Tile>) {
        for (x, y) in GridEnumerator::new(elements) {
            let t = elements.get(x as isize, y as isize).unwrap();
            let mut force = 1.0f32 * Vector2::y();
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

    fn update<R: Rng>(&mut self, elements: &Grid<Tile>, config: &Config, rng: &mut R) {
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

    fn get(&self, x: isize, y: isize) -> Option<&Vector2<f32>> {
        self.0.read().get(x, y)
    }

    fn potential_moves(&self) -> Grid<PotentialMoves> {
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
