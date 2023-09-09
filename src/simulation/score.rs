use crate::grid::{GridEnumerator, GridLike};

use super::{config::Config, State};

pub struct PairwiseTileScorer {
    width: usize,
    scores: Vec<(f32, f32)>,
}

impl PairwiseTileScorer {
    pub fn new(state: &State) -> Self {
        let scores = vec![(0.0, 0.0); state.elements.width() * state.elements.height() * 25];
        let mut this = Self {
            width: state.elements.width(),
            scores,
        };

        for (x, y) in GridEnumerator::new(&state.elements) {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let i = this.index(x, y, dx, dy);
                    let t = state.elements.get(x as isize, y as isize).unwrap();
                    let other = state.elements.get(x as isize + dx, y as isize + dy);
                    let (an, dn) = match other {
                        Some(o) => {
                            let od = o.density(&state.config);
                            let td = t.density(&state.config);
                            (t.attractive_force(o, &state.config), (od - td) / (od + td))
                        }
                        None => (t.adhesion(&state.config).powi(2), 0.0),
                    };
                    let (a, d) = &mut this.scores[i];
                    *a = an;
                    *d = dn;
                }
            }
        }

        this
    }

    fn index(&self, x: usize, y: usize, dx: isize, dy: isize) -> usize {
        (y * self.width + x) * 25 + (dy + 2) as usize * 5 + (dx + 2) as usize
    }

    pub fn get(&mut self, t: (isize, isize), other: (isize, isize)) -> (f32, f32) {
        let i = self.index(t.0 as usize, t.1 as usize, other.0 - t.0, other.1 - t.1);
        self.scores[i]
    }
}

pub fn position_score(
    scorer: &mut PairwiseTileScorer,
    config: &Config,
    tx: isize,
    ty: isize,
    x: isize,
    y: isize,
) -> f32 {
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

        let (a, d) = scorer.get((tx, ty), o);

        attraction_score += aw * a;
        density_score += dw * d;
    }

    config.attraction_score_weight * attraction_score + config.density_score_weight * density_score
}
