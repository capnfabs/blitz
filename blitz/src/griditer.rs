use std::ops::Index;

pub trait GridIterator<'a>: Iterator<Item = ((usize, usize), &'a mut u16)> {}

pub trait Sizeable {
    fn size(&self) -> (usize, usize);
}

impl<T> Sizeable for ndarray::Array2<T> {
    fn size(&self) -> (usize, usize) {
        self.dim()
    }
}

impl<'a, T> GridIterator<'a> for T where T: Iterator<Item = ((usize, usize), &'a mut u16)> {}

pub trait GridRandomAccess: Index<(usize, usize), Output = u16> + Sizeable {}
impl<T> GridRandomAccess for T where T: Index<(usize, usize), Output = u16> + Sizeable {}
