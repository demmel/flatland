use std::{cmp::Reverse, collections::HashSet};

use ordered_float::OrderedFloat;

use crate::grid::{Grid, GridEnumerator, GridLike};

use super::Tile;

pub(crate) fn reduce_potential_moves(
    mut potential_moves: Grid<PotentialMoves>,
    elements: &Grid<Tile>,
) -> Grid<(isize, isize)> {
    let mut iters = 0;
    let resolutions = loop {
        iters += 1;
        let mut conflicts = find_conflicts(&potential_moves);
        let found_conflicts = resolve_conflicts(elements, &mut conflicts, &mut potential_moves);
        if !found_conflicts {
            break conflicts;
        }
    };
    println!("Conflict iters: {iters}");

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

    if !found {
        let mut orphans: Vec<_> = potential_moves
            .enumerate()
            .filter(|(_x, _y, p)| p.current().is_none())
            .map(|(x, y, _p)| (x as isize, y as isize))
            .collect();

        let mut slots: HashSet<_> = conflicts
            .enumerate()
            .filter(|(_x, _y, p)| matches!(p, MoveConflict::None))
            .map(|(x, y, _p)| (x as isize, y as isize))
            .collect();

        orphans
            .sort_by_key(|(x, y)| Reverse(OrderedFloat(elements.get(*x, *y).unwrap().integrity)));

        for (ox, oy) in orphans {
            let (sx, sy) = slots
                .iter()
                .min_by_key(|(x, y)| (*x - ox).pow(2) + (*y - oy).pow(2))
                .unwrap();
            potential_moves.get_mut(ox, oy).unwrap().push((*sx, *sy));
            *conflicts.get_mut(*sx, *sy).unwrap() = MoveConflict::Resolved((ox, oy));
            slots.remove(&(*sx, *sy));
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
