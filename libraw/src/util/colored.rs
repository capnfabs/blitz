use crate::Color;

pub struct Colored<T>(Vec<T>);

static COLORS: [Color; 3] = [Color::Red, Color::Green, Color::Blue];

impl<T> Colored<T> {
    pub fn new(red: T, green: T, blue: T) -> Colored<T> {
        Colored(vec![red, green, blue])
    }

    pub fn iter(&self) -> UnholyIter<T> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> UnholyMutableIter<T> {
        self.into_iter()
    }

    pub fn split_mut(&mut self) -> (&mut T, &mut T, &mut T) {
        let (r, rest) = self.0.split_at_mut(1);
        let (g, b) = rest.split_at_mut(1);
        (&mut r[0], &mut g[0], &mut b[0])
    }

    pub fn split(&self) -> (&T, &T, &T) {
        (&self.0[0], &self.0[1], &self.0[2])
    }
}

impl<'a, T> IntoIterator for &'a Colored<T> {
    type Item = (Color, &'a T);
    type IntoIter = UnholyIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        COLORS.iter().copied().zip(self.0.iter())
    }
}

impl<T> std::ops::Index<Color> for Colored<T> {
    type Output = T;

    fn index(&self, index: Color) -> &Self::Output {
        match index {
            Color::Red => &self.0[0],
            Color::Green => &self.0[1],
            Color::Blue => &self.0[2],
        }
    }
}

impl<T> std::ops::IndexMut<Color> for Colored<T> {
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        match index {
            Color::Red => &mut self.0[0],
            Color::Green => &mut self.0[1],
            Color::Blue => &mut self.0[2],
        }
    }
}

type UnholyIter<'a, T> =
    std::iter::Zip<std::iter::Copied<std::slice::Iter<'static, Color>>, std::slice::Iter<'a, T>>;

type UnholyMutableIter<'a, T> =
    std::iter::Zip<std::iter::Copied<std::slice::Iter<'static, Color>>, std::slice::IterMut<'a, T>>;

impl<'a, T> IntoIterator for &'a mut Colored<T> {
    type Item = (Color, &'a mut T);
    type IntoIter = UnholyMutableIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        COLORS.iter().copied().zip(self.0.iter_mut())
    }
}
