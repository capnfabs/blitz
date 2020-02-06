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

pub trait Grid<'a, T: Copy> {
    fn size(&self) -> Size;
    fn at(&self, offset: Position) -> T;
}

#[derive(Debug)]
pub struct DataGrid<'a, T: Copy> {
    data: &'a [T],
    size: Size,
}

pub fn wrap<T: Copy>(vals: &[T], size: Size) -> DataGrid<T> {
    if size.count() != vals.len() {
        panic!("Noooooooo")
    }
    DataGrid { data: vals, size }
}

struct WrapperGrid<'a, T: Copy> {
    grid: &'a DataGrid<'a, T>,
    anchor_pos: Position,
    size: Size,
}

impl<'a, T: Copy> Grid<'a, T> for WrapperGrid<'a, T> {
    fn size(&self) -> Size {
        self.size
    }

    fn at(&self, pos: Position) -> T {
        let Position(x, y) = pos.extending(self.anchor_pos);
        let Size(width, height) = self.size;
        let x = x.rem_euclid(width);
        let y = y.rem_euclid(height);
        self.grid.at(Position(x, y))
    }
}

impl<'a, T: Copy> Grid<'a, T> for DataGrid<'a, T> {
    fn size(&self) -> Size {
        self.size
    }

    fn at(&self, pos: Position) -> T {
        let Position(x, y) = pos;
        let Size(width, height) = self.size;
        let x = x.rem_euclid(width);
        let y = y.rem_euclid(height);
        self.data[y * width + x]
    }
}

impl<'a, T: Copy> DataGrid<'a, T> {
    pub fn row(&self, which: Y) -> &[T] {
        let Size(width, _) = self.size;
        &self.data[which * width..(which + 1) * width]
    }

    pub fn subgrid(&'a self, offset: Position, size: Size) -> impl Grid<'a, T> {
        WrapperGrid {
            grid: &self,
            anchor_pos: offset,
            size,
        }
    }
}
