use std::{cmp::Reverse, collections::HashSet};

use fixed::{types::extra::U0, FixedI32};
use kiddo::fixed::{distance::squared_euclidean, kdtree::KdTree};
use ordered_float::OrderedFloat;

use crate::grid::{Grid, GridEnumerator, GridLike};

use super::Tile;

pub(crate) fn reduce_potential_moves(
    mut potential_moves: Grid<PotentialMoves>,
    elements: &Grid<Tile>,
) -> Grid<(isize, isize)> {
    let mut iters = 0;
    let mut resolutions = loop {
        iters += 1;
        let mut conflicts = find_conflicts(&potential_moves);
        let found_conflicts = resolve_conflicts(elements, &mut conflicts, &mut potential_moves);
        if !found_conflicts {
            break conflicts;
        }
    };
    println!("Conflict iters: {iters}");

    resolve_orphans(&mut resolutions, potential_moves, elements);

    Grid::new(
        elements.width(),
        elements.height(),
        |x, y| match resolutions.get(x as isize, y as isize).unwrap() {
            MoveConflict::Resolved((old_x, old_y)) => (*old_x, *old_y),
            MoveConflict::None => {
                panic!("No cells should be empty after conflict resolution")
            }
            MoveConflict::Conflict(_) => {
                panic!("No conflicts should remain after conflict resolution")
            }
        },
    )
}

fn resolve_orphans(
    resolutions: &mut Grid<MoveConflict>,
    mut potential_moves: Grid<PotentialMoves>,
    elements: &Grid<Tile>,
) {
    let mut orphans: Vec<_> = potential_moves
        .enumerate()
        .filter(|(_x, _y, p)| p.current().is_none())
        .map(|(x, y, _p)| (x as isize, y as isize))
        .collect();

    let slots: Vec<_> = resolutions
        .enumerate()
        .filter(|(_x, _y, p)| matches!(p, MoveConflict::None))
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

    println!("Orphans: {}", orphans.len());

    orphans.sort_by_key(|(x, y)| Reverse(OrderedFloat(elements.get(*x, *y).unwrap().integrity)));

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
        potential_moves.get_mut(ox, oy).unwrap().push((sx, sy));
        *resolutions.get_mut(sx, sy).unwrap() = MoveConflict::Resolved((ox, oy));
        slots_spatial.remove(&[ssx, ssy], slot_idx);
    }
}

pub(crate) fn find_conflicts(potential_moves: &Grid<PotentialMoves>) -> Grid<MoveConflict> {
    let mut conflicts = Grid::new(
        potential_moves.width(),
        potential_moves.height(),
        |_x, _y| MoveConflict::None,
    );
    for (x, y, p) in potential_moves.enumerate() {
        if let Some((new_x, new_y)) = p.current() {
            conflicts
                .get_mut(new_x, new_y)
                .unwrap()
                .push_move((x as isize, y as isize));
        }
    }
    conflicts
}

pub(crate) fn resolve_conflicts(
    elements: &Grid<Tile>,
    conflicts: &mut Grid<MoveConflict>,
    potential_moves: &mut Grid<PotentialMoves>,
) -> bool {
    let mut found = false;
    for (x, y) in GridEnumerator::new(conflicts) {
        let c = conflicts.get_mut(x as isize, y as isize).unwrap();
        if let MoveConflict::Conflict(candidates) = c {
            found = true;

            let (winner_index, _) = candidates
                .iter()
                .enumerate()
                .max_by_key(|(_, (cx, cy))| OrderedFloat(elements.get(*cx, *cy).unwrap().integrity))
                .unwrap();

            candidates.swap_remove(winner_index);

            for (cx, cy) in candidates {
                potential_moves.get_mut(*cx, *cy).unwrap().pop();
            }
        }
    }

    found
}

#[derive(Debug, Clone)]
pub(crate) enum MoveConflict {
    None,
    Resolved((isize, isize)),
    Conflict(Vec<(isize, isize)>),
}

impl MoveConflict {
    fn push_move(&mut self, m: (isize, isize)) {
        match self {
            MoveConflict::None => *self = MoveConflict::Resolved(m),
            MoveConflict::Resolved(only) => *self = MoveConflict::Conflict(vec![*only, m]),
            MoveConflict::Conflict(candidates) => candidates.push(m),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PotentialMoves {
    reverse_preferences: Vec<(isize, isize)>,
}

impl PotentialMoves {
    pub(crate) fn new(reverse_preferences: Vec<(isize, isize)>) -> PotentialMoves {
        Self {
            reverse_preferences,
        }
    }

    fn current(&self) -> Option<(isize, isize)> {
        self.reverse_preferences.last().cloned()
    }

    fn pop(&mut self) {
        self.reverse_preferences.pop();
    }

    fn push(&mut self, m: (isize, isize)) {
        self.reverse_preferences.push(m);
    }
}
