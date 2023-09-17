use crate::grid::GridLike;

use super::{config::Config, State};

pub const WINDOW_SIZE: isize = 5;
pub const WINDOW_SIZE_OV_2: isize = WINDOW_SIZE / 2;
pub const BUCKET_SIZE: usize = (WINDOW_SIZE.pow(2) as usize) / 2 + 1;

pub struct PairwiseTileScorer {
    width: usize,
    scores: Vec<(f32, f32)>,
}

impl PairwiseTileScorer {
    pub fn new(state: &State) -> Self {
        let width = state.elements.width() + WINDOW_SIZE_OV_2 as usize;
        let height = state.elements.height() + WINDOW_SIZE_OV_2 as usize;
        let scores = vec![(0.0, 0.0); width * height * BUCKET_SIZE];
        let mut this = Self { width, scores };

        for y in 0..height {
            for x in 0..width {
                for dy in 0..=(WINDOW_SIZE_OV_2) {
                    for dx in if dy == 0 {
                        0..=(WINDOW_SIZE_OV_2)
                    } else {
                        -(WINDOW_SIZE_OV_2)..=(WINDOW_SIZE_OV_2)
                    } {
                        let x = x as isize - (WINDOW_SIZE_OV_2);
                        let y = y as isize - (WINDOW_SIZE_OV_2);
                        let i = this.index(x, y, dx, dy);
                        let t = state.elements.get(x as isize, y as isize);
                        let other = state.elements.get(x as isize + dx, y as isize + dy);
                        let (an, dn) = match (t, other) {
                            (None, None) => (0.0, 0.0),
                            (None, Some(t)) | (Some(t), None) => {
                                (t.adhesion(&state.config).powi(2), 0.0)
                            }
                            (Some(t), Some(o)) => {
                                let od = o.density(&state.config);
                                let td = t.density(&state.config);
                                (t.attractive_force(o, &state.config), (od - td) / (od + td))
                            }
                        };
                        let (a, d) = &mut this.scores[i];
                        *a = an;
                        *d = dn;
                    }
                }
            }
        }

        this
    }

    fn index(&self, x: isize, y: isize, dx: isize, dy: isize) -> usize {
        let x = (x + WINDOW_SIZE_OV_2) as usize;
        let y = (y + WINDOW_SIZE_OV_2) as usize;

        let i =
            (y * self.width + x) * BUCKET_SIZE + dy as usize * WINDOW_SIZE as usize + dx as usize;

        i
    }

    fn get(&self, t: (isize, isize), other: (isize, isize)) -> (f32, f32) {
        let dx = other.0 - t.0;
        let dy = other.1 - t.1;

        if dy > 0 || dy >= 0 && dx >= 0 {
            self.scores[self.index(t.0, t.1, dx, dy)]
        } else {
            let (a, d) = self.scores[self.index(other.0, other.1, -dx, -dy)];
            (a, -d)
        }
    }

    pub fn position_score(&self, config: &Config, tx: isize, ty: isize, x: isize, y: isize) -> f32 {
        let select_other = |ox: isize, oy: isize| {
            if tx == ox && ty == oy {
                (x, y)
            } else {
                (ox, oy)
            }
        };

        let neighbors = [
            select_other(x - 1, y - 1),
            select_other(x, y - 1),
            select_other(x + 1, y - 1),
            select_other(x - 1, y),
            select_other(x + 1, y),
            select_other(x - 1, y + 1),
            select_other(x, y + 1),
            select_other(x + 1, y + 1),
        ];

        let mut attraction_score = 0.0;
        let mut density_score = 0.0;
        for (i, o) in neighbors.into_iter().enumerate() {
            let aw = config.neighbor_attraction_weights[i].as_f32();
            let dw = config.neighbor_density_weights[i].as_f32();

            let (a, d) = self.get((tx, ty), o);

            attraction_score += aw * a;
            density_score += dw * d;
        }

        let dx = x - tx;
        let dy = y - ty;
        let dist = dx.abs().max(dy.abs());

        config.attraction_score_weight.as_f32() * attraction_score
            + config.density_score_weight.as_f32() * density_score
            - (0.0001 * dist as f32)
    }
}
