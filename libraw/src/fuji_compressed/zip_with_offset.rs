use std::iter::Fuse;

// ZipWithOffset, copied from itertools::zip_longest

#[derive(Clone, Debug)]
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct ZipWithOffset<T, U> {
    a: Fuse<T>,
    b: Fuse<U>,
    a_delay_initial: usize,
    b_delay_initial: usize,
    a_delay: usize,
    b_delay: usize,
}

/// Create a new `ZipLongest` iterator.
pub fn zip_with_offset<T, U>(a: T, a_delay: usize, b: U, b_delay: usize) -> ZipWithOffset<T, U>
where
    T: Iterator,
    U: Iterator,
{
    ZipWithOffset {
        a: a.fuse(),
        b: b.fuse(),
        a_delay_initial: a_delay,
        b_delay_initial: b_delay,
        a_delay,
        b_delay,
    }
}

impl<T, U> Iterator for ZipWithOffset<T, U>
where
    T: Iterator,
    U: Iterator,
{
    type Item = (Option<T::Item>, Option<U::Item>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // TODO: the nested options make this confusing; maybe there's a better
        // way to do it (i.e. with Option::and_then and storing a boolean
        // indicating whether the iterators have started or not).
        let a_val = if self.a_delay == 0 {
            Some(self.a.next())
        } else {
            self.a_delay -= 1;
            None
        };
        let b_val = if self.b_delay == 0 {
            Some(self.b.next())
        } else {
            self.b_delay -= 1;
            None
        };
        match (a_val, b_val) {
            // Both iterators have started already, so we check their values,
            // and if they're both None it means we're done
            (Some(a), Some(b)) => match (a, b) {
                (None, None) => None,
                (a, b) => Some((a, b)),
            },
            // One or None of the iterators has started
            (a, b) => {
                // Unwrap a level of (a0
                let a = match a {
                    Some(a) => a,
                    None => None,
                };
                let b = match b {
                    Some(b) => b,
                    None => None,
                };
                Some((a, b))
            }
        }
    }
}
