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
}

impl<T> GridLike<T> for Grid<T> {
    fn get(&self, x: isize, y: isize) -> Option<&T> {
        if x < 0 || y < 0 {
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
    x: usize,
    y: usize,
    size: usize,
    _p: PhantomData<T>,
}

impl<'a, T, G: GridLike<T>> GridWindows<'a, T, G> {
    fn new(grid: &'a G, size: usize) -> Self {
        Self {
            grid,
            x: 0,
            y: 0,
            size,
            _p: PhantomData,
        }
    }
}

impl<'a, T, G: GridLike<T>> Iterator for GridWindows<'a, T, G> {
    type Item = GridWindow<'a, T, G>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.y as usize >= self.grid.height() {
            return None;
        }
        let w = GridWindow {
            grid: self.grid,
            x: self.x,
            y: self.y,
            size: self.size,
            _p: PhantomData,
        };
        self.x += 1;
        if self.x >= self.grid.width() {
            self.y += 1;
            self.x = 0;
        }
        Some(w)
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
