use std::cmp::Ordering;

pub struct Ranker<T> {
    i: usize,
    j: usize,
    remaining: Vec<(usize, usize)>,
    competitors: Vec<T>,
    needed: usize,
}

impl<T> Ranker<T> {
    pub fn new(competitors: Vec<T>, needed: usize) -> Self {
        Self {
            i: 0,
            j: 0,
            remaining: vec![(0, competitors.len() - 1)],
            competitors,
            needed,
        }
    }

    pub fn current(&self) -> usize {
        self.j
    }

    pub fn pivot(&self) -> usize {
        self.remaining.last().unwrap().1
    }

    pub fn competitors(&self) -> &[T] {
        &self.competitors
    }

    pub fn rank(&mut self, ordering: Ordering) -> bool {
        if matches!(ordering, Ordering::Greater) {
            self.competitors.swap(self.i, self.j);
            self.i += 1;
        }
        self.j += 1;
        if self.j == self.pivot() {
            let (start, end) = self.remaining.pop().unwrap();
            self.competitors.swap(self.i, end);
            if start + 1 < self.i && start <= self.needed - 1 {
                self.remaining.push((start, self.i - 1));
            }
            if self.i + 1 < end && self.i + 1 <= self.needed - 1 {
                self.remaining.push((self.i + 1, end));
            }
            if let Some((start, _end)) = self.remaining.last() {
                self.i = *start;
                self.j = *start;
                true
            } else {
                false
            }
        } else {
            true
        }
    }
}

#[cfg(test)]
mod test {
    use super::Ranker;

    #[test]
    fn test_rank_sll() {
        let mut ranker = Ranker::new(vec![4, 2, 5, 1, 6, 3], 6);
        loop {
            let ordering =
                ranker.competitors()[ranker.current()].cmp(&ranker.competitors()[ranker.pivot()]);
            if !ranker.rank(ordering) {
                break;
            }
        }
        assert_eq!(ranker.competitors(), &[6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_rank_2() {
        let mut ranker = Ranker::new(vec![1, 2, 3, 4, 5, 6], 2);
        loop {
            let ordering =
                ranker.competitors()[ranker.current()].cmp(&ranker.competitors()[ranker.pivot()]);
            if !ranker.rank(ordering) {
                break;
            }
        }
        assert_eq!(ranker.competitors(), &[6, 5, 3, 4, 2, 1]);
    }
}
