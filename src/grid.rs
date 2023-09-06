use std::marker::PhantomData;

pub trait GridLike<T> {
    fn get(&self, x: isize, y: isize) -> Option<&T>;
    fn height(&self) -> usize;
    fn width(&self) -> usize;

    fn windows(&self, size: usize) -> GridWindows<'_, T, Self>
    where
        Self: Sized,
    {
        GridWindows::new(self, size)
    }
}

#[derive(Debug, Clone)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    cells: Vec<T>,
}

impl<T> Grid<T> {
    pub fn new(width: usize, height: usize, init: impl Fn(usize, usize) -> T + Copy) -> Self {
        Self {
            width,
            height,
            cells: (0..height)
                .flat_map(|y| (0..width).map(move |x| init(x, y)))
                .collect(),
        }
    }

    pub fn from_cells(width: usize, height: usize, cells: Vec<T>) -> Self {
        Self {
            width,
            height,
            cells,
        }
    }

    pub fn enumerate(&self) -> impl Iterator<Item = (usize, usize, &T)> {
        (0..self.height()).flat_map(move |y| {
            (0..self.width()).map(move |x| (x, y, self.get(x as isize, y as isize).unwrap()))
        })
    }

    pub fn get_mut(&mut self, x: isize, y: isize) -> Option<&mut T> {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return None;
        }
        self.cells.get_mut(x as usize + self.width * y as usize)
    }
}

impl<T> GridLike<T> for Grid<T> {
    fn get(&self, x: isize, y: isize) -> Option<&T> {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return None;
        }
        self.cells.get(x as usize + self.width * y as usize)
    }

    fn height(&self) -> usize {
        self.height
    }

    fn width(&self) -> usize {
        self.width
    }
}

pub struct GridWindows<'a, T, G: GridLike<T>> {
    grid: &'a G,
    enumerator: GridEnumerator,
    size: usize,
    _p: PhantomData<T>,
}

impl<'a, T, G: GridLike<T>> GridWindows<'a, T, G> {
    fn new(grid: &'a G, size: usize) -> Self {
        Self {
            grid,
            enumerator: GridEnumerator::new(grid),
            size,
            _p: PhantomData,
        }
    }
}

impl<'a, T, G: GridLike<T>> Iterator for GridWindows<'a, T, G> {
    type Item = GridWindow<'a, T, G>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((x, y)) = self.enumerator.next() {
            Some(GridWindow {
                grid: self.grid,
                x,
                y,
                size: self.size,
                _p: PhantomData,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct GridWindow<'a, T, G: GridLike<T>> {
    grid: &'a G,
    x: usize,
    y: usize,
    size: usize,
    _p: PhantomData<T>,
}

impl<'a, T, G: GridLike<T>> GridLike<T> for GridWindow<'a, T, G> {
    fn get(&self, x: isize, y: isize) -> Option<&T> {
        self.grid.get(self.x as isize + x, self.y as isize + y)
    }

    fn height(&self) -> usize {
        self.size
    }

    fn width(&self) -> usize {
        self.size
    }
}

impl<'a, T, G: GridLike<T>> GridWindow<'a, T, G> {
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.height())
            .flat_map(move |y| {
                (0..self.width()).map(move |x| {
                    (
                        x as isize - (self.width() / 2) as isize,
                        y as isize - (self.height() / 2) as isize,
                    )
                })
            })
            .filter_map(|(x, y)| self.get(x, y))
    }
}

pub struct GridEnumerator {
    width: usize,
    height: usize,
    x: isize,
    y: isize,
    rx: isize,
    ry: isize,
}

impl GridEnumerator {
    pub fn new<T, G: GridLike<T>>(grid: &G) -> Self {
        Self {
            width: grid.width(),
            height: grid.height(),
            x: 0,
            y: 0,
            rx: (grid.width() - 1) as isize,
            ry: (grid.height() - 1) as isize,
        }
    }
}

impl Iterator for GridEnumerator {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y as usize >= self.height {
            return None;
        }
        let ret = (self.x as usize, self.y as usize);
        self.x += 1;
        if self.x as usize >= self.width {
            self.y += 1;
            self.x = 0;
        }
        Some(ret)
    }
}

impl DoubleEndedIterator for GridEnumerator {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.ry < 0 {
            return None;
        }
        let ret = (self.rx as usize, self.ry as usize);
        self.rx -= 1;
        if self.rx < 0 {
            self.ry -= 1;
            self.rx = (self.width - 1) as isize;
        }
        Some(ret)
    }
}
