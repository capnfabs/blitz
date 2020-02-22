use libraw::util::datagrid::MutableDataGrid;

// TODO: make this actually valid.
pub fn black_sub(grid: &mut MutableDataGrid<u16>) {
    for x in grid.iter_mut() {
        *x = x.saturating_sub(1022);
    }
}
