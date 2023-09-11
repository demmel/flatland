use fixed::{types::extra::U0, FixedI32};
use kiddo::fixed::{distance::squared_euclidean, kdtree::KdTree};
use ordered_float::OrderedFloat;

use crate::grid::{Grid, GridEnumerator, GridLike};

use super::{config::Config, score::WINDOW_SIZE, PairwiseTileScorer};

pub fn reduce_potential_moves(
    scorer: &mut PairwiseTileScorer,
    config: &Config,
    potential_moves: &mut Grid<PotentialMoves>,
) -> Grid<(isize, isize)> {
    let mut iters = 0;
    let mut max_in_conflict = 0;
    let mut conflicts = Grid::new(
        potential_moves.width(),
        potential_moves.height(),
        |_x, _y| MoveConflict::new(),
    );
    let mut resolutions = loop {
        iters += 1;
        for (x, y) in GridEnumerator::new(&conflicts) {
            let c = conflicts.get_mut(x as isize, y as isize).unwrap();
            c.reset();
        }
        for (x, y, p) in potential_moves.enumerate() {
            if let Some((new_x, new_y)) = p.current() {
                let c = conflicts.get_mut(new_x, new_y).unwrap();
                c.push_move((x as isize, y as isize));
                max_in_conflict = max_in_conflict.max(c.len);
            }
        }
        let found_conflicts = resolve_conflicts(scorer, config, &mut conflicts, potential_moves);
        if !found_conflicts {
            break conflicts;
        }
    };
    // println!("Conflict resolution iterations: {iters}");
    // println!("Max in conflict: {max_in_conflict}");

    resolve_orphans(&mut resolutions, potential_moves);

    Grid::new(potential_moves.width(), potential_moves.height(), |x, y| {
        let r = resolutions.get(x as isize, y as isize).unwrap();
        if r.is_resolved() {
            let (ox, oy) = r.resolved_move();
            (ox, oy)
        } else {
            panic!("Unresolved conflict");
        }
    })
}

fn resolve_orphans(
    resolutions: &mut Grid<MoveConflict>,
    potential_moves: &mut Grid<PotentialMoves>,
) {
    let orphans: Vec<_> = potential_moves
        .enumerate()
        .filter(|(_x, _y, p)| p.current().is_none())
        .map(|(x, y, _p)| (x as isize, y as isize))
        .collect();

    // println!("Orphans: {}", orphans.len());

    let slots: Vec<_> = resolutions
        .enumerate()
        .filter(|(_x, _y, p)| p.is_empty())
        .map(|(x, y, _p)| {
            [
                FixedI32::<U0>::from(x as i32),
                FixedI32::<U0>::from(y as i32),
            ]
        })
        .collect();

    let mut slots_spatial: KdTree<FixedI32<U0>, usize, 2, 128, u32> =
        KdTree::with_capacity(slots.len());
    for (i, s) in slots.iter().enumerate() {
        slots_spatial.add(s, i);
    }

    for (ox, oy) in orphans {
        let (_, slot_idx) = slots_spatial.nearest_one(
            &[
                FixedI32::<U0>::from(ox as i32),
                FixedI32::<U0>::from(oy as i32),
            ],
            &squared_euclidean,
        );

        let [ssx, ssy] = slots[slot_idx];
        let [sx, sy]: [i32; 2] = [ssx.to_num(), ssy.to_num()];
        let [sx, sy] = [sx as isize, sy as isize];
        resolutions.get_mut(sx, sy).unwrap().resolve((ox, oy));
        slots_spatial.remove(&[ssx, ssy], slot_idx);
    }
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
}

impl MoveConflict {
    fn new() -> Self {
        Self {
            candidates: [None; (WINDOW_SIZE * WINDOW_SIZE) as usize],
            len: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0
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

    fn push_move(&mut self, m: (isize, isize)) {
        self.candidates[self.len] = Some(m);
        self.len += 1;
    }

    fn reset(&mut self) {
        self.len = 0;
    }

    fn resolve(&mut self, m: (isize, isize)) {
        self.reset();
        self.push_move(m);
    }

    fn swap_remove(&mut self, i: usize) {
        self.candidates[i] = self.candidates[self.len - 1];
        self.len -= 1;
    }
}

#[derive(Debug)]
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
