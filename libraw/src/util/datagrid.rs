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

    pub fn encloses(&self, p: Position) -> bool {
        p.0 < self.0 && p.1 < self.1
    }
}

pub trait Sizeable {
    fn size(&self) -> Size;
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
