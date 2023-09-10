use crate::grid::GridLike;

use super::{config::Config, State};

pub struct PairwiseTileScorer {
    width: usize,
    scores: Vec<(f32, f32)>,
}

impl PairwiseTileScorer {
    pub fn new(state: &State) -> Self {
        let scores =
            vec![(0.0, 0.0); (state.elements.width() + 2) * (state.elements.height() + 2) * 13];
        let mut this = Self {
            width: state.elements.width() + 2,
            scores,
        };

        for y in 0..(state.elements.height() + 2) {
            for x in 0..this.width {
                for dy in 0..=2 {
                    for dx in if dy == 0 { 0..=2 } else { -2..=2 } {
                        let x = x as isize - 2;
                        let y = y as isize - 2;
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
        let x = (x + 2) as usize;
        let y = (y + 2) as usize;

        let i = (y * self.width + x) * 13 + dy as usize * 5 + dx as usize;

        i
    }

    fn get(&self, t: (isize, isize), other: (isize, isize)) -> (f32, f32) {
        let dx = other.0 - t.0;
        let dy = other.1 - t.1;

        if dy > 0 || dy >= 0 && dx >= 0 {
            self.scores[self.index(t.0, t.1, dx, dy)]
        } else {
            let (a, d) = self.scores[self.index(other.0, other.1, -dx, -dy)];
            (a, -1.0 * d)
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
            let aw = config.neighbor_attraction_weights[i];
            let dw = config.neighbor_density_weights[i];

            let (a, d) = self.get((tx, ty), o);

            attraction_score += aw * a;
            density_score += dw * d;
        }

        config.attraction_score_weight * attraction_score
            + config.density_score_weight * density_score
    }
}
