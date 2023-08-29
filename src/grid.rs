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

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        self.cells.get(x + self.width * y)
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn windows(&self, size: usize) -> impl Iterator<Item = GridWindow<T>> + '_ {
        GridWindows::new(self, size)
    }
}

pub struct GridWindows<'a, T> {
    grid: &'a Grid<T>,
    x: usize,
    y: usize,
    size: usize,
}

impl<'a, T> GridWindows<'a, T> {
    fn new(grid: &'a Grid<T>, size: usize) -> Self {
        Self {
            grid,
            x: 0,
            y: 0,
            size,
        }
    }
}

impl<'a, T> Iterator for GridWindows<'a, T> {
    type Item = GridWindow<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.y as usize >= self.grid.height() {
            return None;
        }
        let w = GridWindow {
            grid: self.grid,
            x: self.x,
            y: self.y,
            size: self.size,
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
pub struct GridWindow<'a, T> {
    grid: &'a Grid<T>,
    x: usize,
    y: usize,
    size: usize,
}

impl<'a, T> GridWindow<'a, T> {
    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        let x = if let Some(x) = (self.x + x).checked_sub(self.size / 2) {
            x
        } else {
            return None;
        };

        let y = if let Some(y) = (self.y + y).checked_sub(self.size / 2) {
            y
        } else {
            return None;
        };

        self.grid.get(x, y)
    }

    fn enumerate(&self) -> impl Iterator<Item = (usize, usize, Option<&T>)> {
        (0..self.size).flat_map(move |y| (0..self.size).map(move |x| (x, y, self.get(x, y))))
    }
}
