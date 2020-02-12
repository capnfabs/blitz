use std::ops::{Index, IndexMut};
use std::slice::from_raw_parts_mut;

struct OddMut<T>(T);

struct EvenConst<T>(T);

impl<T> Index<usize> for OddMut<T>
where
    T: Index<usize>,
{
    type Output = T::Output;

    fn index(&self, index: usize) -> &Self::Output {
        if index % 2 == 0 {
            panic!("Tried to index Odd<T> by an even index");
        } else {
            self.0.index(index)
        }
    }
}

impl<T> IndexMut<usize> for OddMut<T>
where
    T: IndexMut<usize>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index % 2 == 0 {
            panic!("Tried to index Odd<T> by an even index");
        } else {
            self.0.index_mut(index)
        }
    }
}

impl<T> Index<usize> for EvenConst<T>
where
    T: Index<usize>,
{
    type Output = T::Output;

    fn index(&self, index: usize) -> &Self::Output {
        if index % 2 == 1 {
            panic!("Tried to index Even<T> by an odd index");
        } else {
            self.0.index(index)
        }
    }
}

trait EvenOddSplittable<U>
where
    Self: Sized,
{
    fn split_odd_mut(self) -> (OddMut<Self>, EvenConst<Self>);
}

impl<'a, T> EvenOddSplittable<T> for &'a mut [T] {
    fn split_odd_mut(self) -> (OddMut<Self>, EvenConst<Self>) {
        let (a, b) = unsafe {
            (
                from_raw_parts_mut(self.as_mut_ptr(), self.len()),
                from_raw_parts_mut(self.as_mut_ptr(), self.len()),
            )
        };
        (OddMut(a), EvenConst(b))
    }
}
