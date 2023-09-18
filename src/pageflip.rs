#[derive(Debug, Clone)]
pub struct PageFlip<T> {
    p1: T,
    p2: T,
    flipped: bool,
}

impl<T> PageFlip<T> {
    pub fn new<F>(init: F) -> Self
    where
        F: Fn() -> T,
    {
        Self {
            p1: init(),
            p2: init(),
            flipped: false,
        }
    }

    pub fn read(&self) -> &T {
        if self.flipped {
            &self.p2
        } else {
            &self.p1
        }
    }

    pub fn write(&mut self) -> &mut T {
        if self.flipped {
            &mut self.p1
        } else {
            &mut self.p2
        }
    }

    pub fn flip(&mut self) {
        self.flipped = !self.flipped;
    }
}
