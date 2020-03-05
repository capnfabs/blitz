use itertools::Itertools;
use libraw::util::datagrid::MutableDataGrid;

// TODO: make this actually valid.
pub fn black_sub(grid: &mut MutableDataGrid<u16>) {
    for x in grid.iter_mut() {
        *x = x.saturating_sub(1022);
    }
}

pub fn gamma_curve(power: f32, max: u16) -> Vec<u16> {
    let fmax = max as f32;
    (0..=max)
        .map(|x| fmax * (x as f32 / fmax).powf(1.0 / power))
        .map(|x| x as u16)
        .collect_vec()
}

pub fn apply_gamma(grid: &mut MutableDataGrid<u16>) {
    let gamma = gamma_curve(2.2, (1 << 14) - 1);
    for x in grid.iter_mut() {
        *x = gamma[*x as usize];
    }
}
