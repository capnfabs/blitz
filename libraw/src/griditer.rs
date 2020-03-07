use crate::Color;
use ndarray::{Array2, ArrayBase, Axis, Data, Ix1, Ix2};
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

pub type FilterMap = Array2<Color>;

pub trait IndexWrapped1<T> {
    fn index_wrapped(&self, a: usize) -> &T;
}

pub trait IndexWrapped2<T> {
    fn index_wrapped(&self, a: usize, b: usize) -> &T;
}

impl<S, T> IndexWrapped1<T> for ArrayBase<S, Ix1>
where
    S: Data<Elem = T>,
{
    fn index_wrapped(&self, a: usize) -> &T {
        let a_max = self.len_of(Axis(0));
        &self[a % a_max]
    }
}

impl<S, T> IndexWrapped2<T> for ArrayBase<S, Ix2>
where
    S: Data<Elem = T>,
{
    fn index_wrapped(&self, a: usize, b: usize) -> &T {
        let a_max = self.len_of(Axis(0));
        let b_max = self.len_of(Axis(1));
        &self[(a % a_max, b % b_max)]
    }
}

#[cfg(test)]
mod test {
    use crate::griditer::IndexWrapped2;
    use itertools::Itertools;
    use ndarray::Array2;

    #[test]
    fn test_a() {
        let v = (0..36).collect_vec();
        let wm = Array2::from_shape_vec((6, 6), v).unwrap();
        assert_eq!(*wm.index_wrapped(0, 0), 0);
        assert_eq!(*wm.index_wrapped(0, 1), 1);
        assert_eq!(*wm.index_wrapped(0, 2), 2);
        assert_eq!(*wm.index_wrapped(0, 6), 0);
        assert_eq!(*wm.index_wrapped(0, 7), 1);
        assert_eq!(*wm.index_wrapped(0, 8), 2);
        assert_eq!(*wm.index_wrapped(8, 2), 14);
        assert_eq!(*wm.index_wrapped(9, 2), 20);
        assert_eq!(*wm.index_wrapped(10, 2), 26);
    }

    #[test]
    fn test_b() {
        let v = (0..36).collect_vec();
        let wm = Array2::from_shape_vec((4, 9), v).unwrap();
        assert_eq!(*wm.index_wrapped(0, 0), 0);
        assert_eq!(*wm.index_wrapped(0, 1), 1);
        assert_eq!(*wm.index_wrapped(0, 2), 2);
        assert_eq!(*wm.index_wrapped(0, 9), 0);
        assert_eq!(*wm.index_wrapped(0, 10), 1);
        assert_eq!(*wm.index_wrapped(0, 11), 2);
        assert_eq!(*wm.index_wrapped(7, 2), 29);
        assert_eq!(*wm.index_wrapped(8, 2), 2);
        assert_eq!(*wm.index_wrapped(9, 2), 11);
    }
}
