use nom::lib::std::slice::from_raw_parts_mut;
use std::ops::{Index, IndexMut};

type X = usize;
type Y = usize;

// TODO: add bounds checking throughout

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Position(pub X, pub Y);

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Offset(pub i32, pub i32);

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Size(pub X, pub Y);

impl Size {
    pub fn count(&self) -> usize {
        self.0 * self.1
    }
}

impl std::ops::Add<Size> for Position {
    type Output = Position;

    fn add(self, rhs: Size) -> Self::Output {
        let Position(x, y) = self;
        let Size(width, height) = rhs;
        Position(x + width, y + height)
    }
}

impl Position {
    fn extending(&self, reference: Position) -> Position {
        Position(self.0 + reference.0, self.1 + reference.1)
    }

    fn wrap_within(self, size: Size) -> Position {
        let Position(x, y) = self;
        let Size(width, height) = size;
        Position(x % width, y % height)
    }
}

impl std::ops::Add<Offset> for Position {
    type Output = Position;

    fn add(self, rhs: Offset) -> Self::Output {
        let Position(x, y) = self;
        let xi = x as i32;
        let yi = y as i32;
        let Offset(xo, yo) = rhs;
        Position((xi + xo) as usize, (yi + yo) as usize)
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct DataGrid<'a, T: Copy> {
    #[derivative(Debug = "ignore")]
    data: &'a [T],
    data_size: Size,
    size: Size,
    anchor_pos: Position,
}

impl<'a, T: Copy> DataGrid<'a, T> {
    pub fn wrap(vals: &[T], size: Size) -> DataGrid<T> {
        if size.count() != vals.len() {
            panic!("dimensions of size and vals don't match up")
        }
        DataGrid {
            data: vals,
            data_size: size,
            size,
            anchor_pos: Position(0, 0),
        }
    }

    pub fn subgrid(&self, offset: Position, size: Size) -> DataGrid<T> {
        DataGrid {
            data: &self.data,
            data_size: self.data_size,
            anchor_pos: offset.extending(self.anchor_pos),
            size,
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn at(&self, pos: Position) -> T {
        let Position(data_x, data_y) = pos.wrap_within(self.size).extending(self.anchor_pos);

        let Size(data_width, _) = self.data_size;

        self.data[data_y * data_width + data_x]
    }

    pub fn row(&self, which: Y) -> &[T] {
        let Position(_, data_y) = Position(0, which).extending(self.anchor_pos);
        let Size(row_width, _) = self.size;
        let Size(data_width, _) = self.data_size;
        let start = data_y * data_width;
        &self.data[start..start + row_width]
    }
}

impl<'a, T: Copy> IntoIterator for &DataGrid<'a, T> {
    type Item = T;
    type IntoIter = DataGridIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        DataGridIterator {
            grid: self.clone(),
            pos: 0,
        }
    }
}

pub struct DataGridIterator<'a, T: Copy> {
    grid: DataGrid<'a, T>,
    pos: usize,
}

impl<'a, T: Copy> Iterator for DataGridIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let Size(width, height) = self.grid.size;
        if self.pos < width * height {
            let row = self.pos / width; // Y
            let col = self.pos % width; // X
            let val = self.grid.at(Position(col, row));
            self.pos += 1;
            Some(val)
        } else {
            None
        }
    }
}

// TODO: unify everything into MutableDataGrid if it's possible, and add a 'wrapping' datagrid which is the same but supports wrapping for reads.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct MutableDataGrid<'a, T: Copy> {
    #[derivative(Debug = "ignore")]
    data: &'a mut [T],
    data_size: Size,
    size: Size,
    anchor_pos: Position,
}

impl<'a, T: Copy> MutableDataGrid<'a, T> {
    pub fn into_inner(self) -> &'a mut [T] {
        self.data
    }

    #[inline]
    fn check_pos(&self, position: Position) {
        let Size(width, height) = self.size;
        let Position(x, y) = position;
        if x >= width {
            panic!("X value {} is out of bounds for width {}", x, width);
        }
        if y >= height {
            panic!("Y value {} is out of bounds for height {}", y, height);
        }
    }

    pub fn new(vals: &mut [T], size: Size) -> MutableDataGrid<T> {
        if size.count() != vals.len() {
            panic!("dimensions of size and vals don't match up")
        }
        MutableDataGrid {
            data: vals,
            data_size: size,
            size,
            anchor_pos: Position(0, 0),
        }
    }

    pub fn subgrid(&mut self, offset: Position, size: Size) -> MutableDataGrid<T> {
        self.check_pos(offset);
        // size 0 is always ok
        if size.0 > 0 && size.1 > 0 {
            // Subtract one here because size is exclusive and the check is
            // inclusive.
            self.check_pos(Position(offset.0 + size.0 - 1, offset.1 + size.1 - 1))
        }
        // Subtract one from these because e.g. 0 + 5 - 1
        MutableDataGrid {
            data: self.data,
            data_size: self.data_size,
            anchor_pos: offset.extending(self.anchor_pos),
            size,
        }
    }

    pub fn row_mut(&mut self, which: Y) -> &mut [T] {
        let Position(data_x, data_y) = Position(0, which).extending(self.anchor_pos);
        let Size(row_width, _) = self.size;
        let Size(data_width, _) = self.data_size;
        let start = data_y * data_width + data_x;
        &mut self.data[start..start + row_width]
    }

    pub fn vertical_stripes_mut(&mut self, width: X) -> Vec<MutableDataGrid<T>> {
        let count_whole_stripes = self.size.0 / width;
        let last_width = self.size.0 % width;

        let mut stripes = vec![];

        for stripe_num in 0..count_whole_stripes {
            let data_copy = unsafe { from_raw_parts_mut(self.data.as_mut_ptr(), self.data.len()) };
            let grid = MutableDataGrid {
                data: data_copy,
                data_size: self.data_size,
                anchor_pos: Position(stripe_num * width, 0).extending(self.anchor_pos),
                size: Size(width, self.size.1),
            };
            stripes.push(grid)
        }

        if last_width != 0 {
            let data_copy = unsafe { from_raw_parts_mut(self.data.as_mut_ptr(), self.data.len()) };
            let x_offset = width * count_whole_stripes;
            let grid = MutableDataGrid {
                data: data_copy,
                data_size: self.data_size,
                anchor_pos: Position(x_offset, 0).extending(self.anchor_pos),
                size: Size(last_width, self.size.1),
            };
            stripes.push(grid);
        }

        stripes
    }

    pub fn iter_pos_mut(&mut self) -> MutPosIter<T> {
        let Position(x_start, y_start) = self.anchor_pos;
        let Size(data_width, _) = self.data_size;
        let Size(width, height) = self.size;
        let x_end = x_start + width;
        let y_end = y_start + height;
        let row_skip = data_width - width;
        let mut iter = self.data.iter_mut();
        let initial_elem = y_start * data_width + x_start;
        if initial_elem != 0 {
            // Skip the first n-1 elements
            iter.nth(initial_elem - 1);
        }
        MutPosIter {
            iter,
            x_start,
            x_end,
            row_skip,
            y_start,
            y_end,
            x: x_start,
            y: y_start,
            started: false,
        }
    }

    pub fn iter_mut(&mut self) -> MutIter<T> {
        MutIter(self.iter_pos_mut())
    }

    pub fn size(&self) -> Size {
        self.size
    }
}

impl<'a, T: Copy> Index<Position> for MutableDataGrid<'a, T> {
    type Output = T;

    fn index(&self, index: Position) -> &Self::Output {
        self.check_pos(index);

        let Position(data_x, data_y) = index.extending(self.anchor_pos);

        let Size(data_width, _) = self.data_size;

        &self.data[data_y * data_width + data_x]
    }
}

impl<'a, T: Copy> IndexMut<Position> for MutableDataGrid<'a, T> {
    fn index_mut(&mut self, index: Position) -> &mut Self::Output {
        self.check_pos(index);

        let Position(data_x, data_y) = index.extending(self.anchor_pos);

        let Size(data_width, _) = self.data_size;

        &mut self.data[data_y * data_width + data_x]
    }
}

pub struct MutPosIter<'a, T: Copy> {
    iter: std::slice::IterMut<'a, T>,
    x_start: usize,
    x_end: usize,
    row_skip: usize,
    y_start: usize,
    y_end: usize,
    // The x-y pos of the next value to take.
    x: usize,
    y: usize,
    started: bool,
}

impl<'a, T: Copy> Iterator for MutPosIter<'a, T> {
    type Item = (Position, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y == self.y_end && self.x == self.x_start {
            return None;
        }
        let pos = Position(self.x - self.x_start, self.y - self.y_start);
        let val = if self.x == self.x_start && self.started {
            // Need to skip elements that were there before.
            self.iter.nth(self.row_skip)
        } else {
            self.iter.next()
        };
        self.started = true;
        self.x += 1;
        if self.x == self.x_end {
            self.y += 1;
            self.x = self.x_start;
        }
        val.map(|v| (pos, v))
    }
}

pub struct MutIter<'a, T: Copy>(MutPosIter<'a, T>);

impl<'a, T: Copy> Iterator for MutIter<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::datagrid::{MutableDataGrid, Position, Size};
    use itertools::Itertools;

    #[test]
    fn grid_iterator() {
        let mut data = (0..36).collect_vec();
        let mut mdg = MutableDataGrid::new(&mut data, Size(6, 6));
        assert_eq!(mdg.iter_mut().map(|x| *x).collect_vec(), data);
    }

    /*
       x0 x1 x2
    y0 00 01 02 03 04 05
    y1 06 07 08 09 10 11
    y2 12 13 14 15 16 17
    y3 18 19 20 21 22 23
    y4 24 25 26 27 28 29
    y5 30 31 32 33 34 35
    */

    #[test]
    fn subgrid_iterator() {
        let mut data = (0..36).collect_vec();
        let mut mdg = MutableDataGrid::new(&mut data, Size(6, 6));
        let mut subgrid = mdg.subgrid(Position(0, 0), Size(2, 6));
        assert_eq!(
            subgrid.iter_mut().map(|x| *x).collect_vec(),
            vec![0, 1, 6, 7, 12, 13, 18, 19, 24, 25, 30, 31]
        );

        let mut subgrid = mdg.subgrid(Position(2, 3), Size(4, 2));
        assert_eq!(
            subgrid.iter_mut().map(|x| *x).collect_vec(),
            vec![20, 21, 22, 23, 26, 27, 28, 29]
        );
    }

    #[test]
    fn subgrid_iterator_with_pos() {
        let mut data = (0..36).collect_vec();
        let mut mdg = MutableDataGrid::new(&mut data, Size(6, 6));
        let mut subgrid = mdg.subgrid(Position(0, 0), Size(2, 3));
        let (pos, val): (Vec<_>, Vec<_>) = subgrid.iter_pos_mut().unzip();

        assert_eq!(
            pos,
            vec![
                Position(0, 0),
                Position(1, 0),
                Position(0, 1),
                Position(1, 1),
                Position(0, 2),
                Position(1, 2)
            ]
        );
        assert_eq!(
            val.iter().map(|x| **x).collect_vec(),
            vec![0, 1, 6, 7, 12, 13]
        );

        let mut subgrid = mdg.subgrid(Position(2, 3), Size(4, 2));
        let (pos, val): (Vec<_>, Vec<_>) = subgrid.iter_pos_mut().unzip();
        assert_eq!(
            pos,
            vec![
                Position(0, 0),
                Position(1, 0),
                Position(2, 0),
                Position(3, 0),
                Position(0, 1),
                Position(1, 1),
                Position(2, 1),
                Position(3, 1),
            ]
        );
        assert_eq!(
            val.iter().map(|x| **x).collect_vec(),
            vec![20, 21, 22, 23, 26, 27, 28, 29]
        );
    }
}
