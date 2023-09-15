use ordered_float::OrderedFloat;

use crate::grid::{Grid, GridEnumerator, GridLike};

use super::{config::Config, score::WINDOW_SIZE, PairwiseTileScorer};

pub fn reduce_potential_moves(
    scorer: &mut PairwiseTileScorer,
    config: &Config,
    potential_moves: &mut Grid<PotentialMoves>,
) -> (Grid<(isize, isize)>, usize) {
    let mut iters = 0;
    let mut conflicts = Grid::new(
        potential_moves.width(),
        potential_moves.height(),
        |_x, _y| MoveConflict::new(),
    );
    let resolutions = loop {
        iters += 1;
        for (x, y) in GridEnumerator::new(&conflicts) {
            let c = conflicts.get_mut(x as isize, y as isize).unwrap();
            if !c.is_locked() {
                c.reset();
            }
        }
        for (x, y) in GridEnumerator::new(potential_moves) {
            let p = potential_moves.get_mut(x as isize, y as isize).unwrap();
            while let Some((new_x, new_y)) = p.current() {
                let c = conflicts.get_mut(new_x, new_y).unwrap();
                if !c.push_move((x as isize, y as isize)) {
                    p.pop();
                } else {
                    break;
                }
            }
        }
        let mut found_conflicts =
            resolve_conflicts(scorer, config, &mut conflicts, potential_moves);
        for (x, y) in GridEnumerator::new(potential_moves) {
            let p = potential_moves.get_mut(x as isize, y as isize).unwrap();
            if p.current().is_none() {
                let c = conflicts.get_mut(x as isize, y as isize).unwrap();
                if !c.is_locked() {
                    c.lock((x as isize, y as isize));
                    found_conflicts = true;
                }
            }
        }
        if !found_conflicts {
            break conflicts;
        }
    };
    // println!("Conflict resolution iterations: {iters}");

    (
        Grid::new(potential_moves.width(), potential_moves.height(), |x, y| {
            let r = resolutions.get(x as isize, y as isize).unwrap();
            if r.is_resolved() {
                let (ox, oy) = r.resolved_move();
                (ox, oy)
            } else {
                panic!("Unresolved conflict");
            }
        }),
        iters,
    )
}

pub fn resolve_conflicts(
    scorer: &mut PairwiseTileScorer,
    config: &Config,
    conflicts: &mut Grid<MoveConflict>,
    potential_moves: &mut Grid<PotentialMoves>,
) -> bool {
    let mut found = false;
    for (x, y) in GridEnumerator::new(conflicts) {
        let c = conflicts.get_mut(x as isize, y as isize).unwrap();
        if c.is_in_conflict() {
            found = true;

            let (winner_index, _) = c
                .iter()
                .enumerate()
                .max_by_key(|(_, (cx, cy))| {
                    OrderedFloat(scorer.position_score(config, *cx, *cy, x as isize, y as isize))
                })
                .unwrap();

            c.swap_remove(winner_index);

            for (cx, cy) in c.iter() {
                potential_moves.get_mut(*cx, *cy).unwrap().pop();
            }
        }
    }

    found
}

#[derive(Debug, Clone)]
pub struct MoveConflict {
    candidates: [Option<(isize, isize)>; (WINDOW_SIZE * WINDOW_SIZE) as usize],
    len: usize,
    locked: bool,
}

impl MoveConflict {
    fn new() -> Self {
        Self {
            candidates: [None; (WINDOW_SIZE * WINDOW_SIZE) as usize],
            len: 0,
            locked: false,
        }
    }

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn is_resolved(&self) -> bool {
        self.len == 1
    }

    fn is_in_conflict(&self) -> bool {
        self.len > 1
    }

    fn iter(&self) -> impl Iterator<Item = &(isize, isize)> {
        self.candidates
            .iter()
            .take(self.len)
            .filter_map(|x| x.as_ref())
    }

    fn resolved_move(&self) -> (isize, isize) {
        self.candidates[0].unwrap()
    }

    fn push_move(&mut self, m: (isize, isize)) -> bool {
        if self.locked {
            false
        } else {
            self.candidates[self.len] = Some(m);
            self.len += 1;
            true
        }
    }

    fn reset(&mut self) {
        self.len = 0;
        self.locked = false;
    }

    fn resolve(&mut self, m: (isize, isize)) {
        self.reset();
        self.push_move(m);
    }

    fn lock(&mut self, m: (isize, isize)) {
        self.resolve(m);
        self.locked = true;
    }

    fn swap_remove(&mut self, i: usize) {
        self.candidates[i] = self.candidates[self.len - 1];
        self.len -= 1;
    }
}

#[derive(Debug, Clone)]
pub struct PotentialMoves {
    preferences: Vec<(isize, isize)>,
    current: usize,
}

impl PotentialMoves {
    pub fn new(preferences: Vec<(isize, isize)>) -> PotentialMoves {
        Self {
            current: 0,
            preferences,
        }
    }

    fn current(&self) -> Option<(isize, isize)> {
        self.preferences.get(self.current).cloned()
    }

    fn pop(&mut self) {
        self.current += 1;
    }
}
