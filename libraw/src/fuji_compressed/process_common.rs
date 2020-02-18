use crate::util::colored::Colored;
use crate::Color;

// We need to pass some of the lines from previous lines to future lines, because they're used in calculations.
// For now, we clone them. It would be entirely possible to make that _not_ the case, but I couldn't be bothered
// for a v1, and this is mega-fast anyway.
pub fn collect_carry_lines(results: &Colored<Vec<Vec<u16>>>) -> Colored<Vec<Vec<u16>>> {
    let reds = &results[Color::Red];
    let greens = &results[Color::Green];
    let blues = &results[Color::Blue];

    Colored::new(
        vec![reds[reds.len() - 2].clone(), reds[reds.len() - 1].clone()],
        vec![
            greens[greens.len() - 2].clone(),
            greens[greens.len() - 1].clone(),
        ],
        vec![
            blues[blues.len() - 2].clone(),
            blues[blues.len() - 1].clone(),
        ],
    )
}

// We make samples interleaving two 'color lines' at a time.
pub const PROCESS: [((Color, usize), (Color, usize), usize); 6] = [
    // Key for this: ((ColorA, row), (ColorB, row), gradient_set)
    ((Color::Red, 0), (Color::Green, 0), 0),
    ((Color::Green, 1), (Color::Blue, 0), 1),
    ((Color::Red, 1), (Color::Green, 2), 2),
    ((Color::Green, 3), (Color::Blue, 1), 0),
    ((Color::Red, 2), (Color::Green, 4), 1),
    ((Color::Green, 5), (Color::Blue, 2), 2),
];

// Just a utility function.
pub fn flatten<A, B>(opt: Option<(A, B)>) -> (Option<A>, Option<B>) {
    match opt {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    }
}

// This is a hardcoded function defining pixels which are interpolated. We should maybe
// do something else, like, store which things are computed / inferred and
// which one's aren't, but this works for the time being.
pub fn is_interpolated(color: Color, row: usize, idx: usize) -> bool {
    if idx % 2 == 1 {
        // Odd indices are never interpolated
        false
    } else {
        match color {
            Color::Red => {
                (row == 0) || (row == 1 && (idx & 3 == 0)) || (row == 2 && (idx & 3 == 2))
            }
            Color::Green => (row == 2) || (row == 5),
            Color::Blue => {
                (row == 0) || (row == 1 && (idx & 3 == 2)) || (row == 2 && (idx & 3 == 0))
            }
        }
    }
}

// Safe to use as a sentinel because the image is only ever 14 bits
// and this is bigger than that.
pub const UNSET: u16 = 0xFFFF;

#[derive(Debug, Clone, Copy)]
pub struct EvenCoefficients {
    north: u16,
    northwest: u16,
    northeast: u16,
    very_north: u16,
}

// Ok, the idea here is:
// it's a weighted average of the values around it.
// - take the value immediately 'above' this one. Call this rb.
// - of the other three values:
//   - choose the two that are closest to rb
//   - call these two values `close`
// - now, compute close[0] + close[1] + 2*rb / 4.
// BUT ALSO
// if both north_west and north_east are equidistant from north, than use those for the weighted
// average regardless of how big they are. This feels like an implementation detail in the original
// fuji code. My guess is we can get better compression ratios by doing something different, but I don't know!
pub fn compute_weighted_average_even(ec: EvenCoefficients) -> u16 {
    let distance = |v: u16| (v as i32 - ec.north as i32).abs();

    let (other_a, other_b) = if distance(ec.northwest) > distance(ec.very_north)
        && distance(ec.northwest) > distance(ec.northeast)
    {
        (ec.very_north, ec.northeast)
    } else if distance(ec.northeast) > distance(ec.northwest)
        && distance(ec.northeast) > distance(ec.very_north)
    {
        (ec.northwest, ec.very_north)
    } else {
        (ec.northwest, ec.northeast)
    };

    (other_a + other_b + 2 * ec.north) / 4
}

pub fn compute_weighted_average_odd(oc: OddCoefficients) -> u16 {
    // If `north` is not in-between `north_west` and `north_east`
    // Then presumably it represents that there's not a continuous variation
    // horizontally. In that case, we want to factor `north` into the
    // computation, otherwise we don't care about it.
    // I'm entirely unsure _why_ this wasn't done for everything, it feels like
    // additional complexity for little benefit.
    if (oc.north > oc.north_west && oc.north > oc.north_east)
        || (oc.north < oc.north_west && oc.north < oc.north_east)
    {
        // Note on typing here: This will all fit in a u16, because we've got 4x max u14s.
        (oc.east + oc.west + 2 * oc.north) / 4
    } else {
        (oc.west + oc.east) / 2
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OddCoefficients {
    west: u16,       // Ra
    north: u16,      // Rb
    north_west: u16, // Rc
    north_east: u16, // Rd
    east: u16,       // Rg
}

pub fn load_odd_coefficients(
    rprevprev: &[u16],
    rprev: &[u16],
    rthis: &[u16],
    idx: usize,
) -> OddCoefficients {
    // Ensure that the vector is even-lengthed
    assert_eq!(rprev.len() % 2, 0);
    assert_eq!(rprev.len(), rprevprev.len());
    assert_eq!(rprev.len(), rthis.len());
    // Ensure that we're targeting an odd value
    assert_eq!(idx % 2, 1);

    // the rightmost value is in a vaguely tricky situation. If it doesn't
    // exist, use the value immediately above instead.
    let last_idx = rprev.len() - 1;
    let (rightmost_rprev, rightmost_rthis) = if idx == last_idx {
        (rprevprev[rprevprev.len() - 1], rprev[rprev.len() - 1])
    } else {
        (rprev[idx + 1], rthis[idx + 1])
    };
    OddCoefficients {
        west: rthis[idx - 1],
        north: rprev[idx],
        north_west: rprev[idx - 1],
        north_east: rightmost_rprev,
        east: rightmost_rthis,
    }
}

pub fn load_even_coefficients(rprev: &[u16], rprevprev: &[u16], idx: usize) -> EvenCoefficients {
    let leftmost_other = if idx == 0 {
        rprevprev[0]
    } else {
        rprev[idx - 1]
    };
    // Ensure that the vector is even-lengthed
    assert_eq!(rprev.len() % 2, 0);
    // Ensure that we're targeting an even value
    assert_eq!(idx % 2, 0);
    EvenCoefficients {
        north: rprev[idx],
        northwest: leftmost_other,
        // Note that row width is always a multiple of 2, so for an even index
        // there will always be at least one value to the right of it.
        northeast: rprev[idx + 1],
        very_north: rprevprev[idx],
    }
}

fn q_value(v: i32) -> i32 {
    if v <= -0x114 {
        -4
    } else if v <= -0x43 {
        -3
    } else if v <= -0x12 {
        -2
    } else if v < 0 {
        -1
    } else if v == 0 {
        0
    } else if v < 0x12 {
        1
    } else if v < 0x43 {
        2
    } else if v < 0x114 {
        3
    } else {
        4
    }
}

pub fn grad_and_weighted_avg_even(idx: usize, rprevprev: &[u16], rprev: &[u16]) -> (u16, i32) {
    let ec = load_even_coefficients(rprev, rprevprev, idx);
    let weighted_average = compute_weighted_average_even(ec);
    let which_grad = 9 * q_value(ec.north as i32 - ec.very_north as i32)
        + q_value(ec.northwest as i32 - ec.north as i32);
    (weighted_average, which_grad)
}

pub fn grad_and_weighted_avg_odd(
    idx: usize,
    rprevprev: &[u16],
    rprev: &[u16],
    rthis: &[u16],
) -> (u16, i32) {
    let oc = load_odd_coefficients(rprevprev, rprev, rthis, idx);

    let weighted_average = compute_weighted_average_odd(oc);
    let which_grad = 9 * q_value(oc.north as i32 - oc.north_west as i32)
        + q_value(oc.north_west as i32 - oc.west as i32);
    (weighted_average, which_grad)
}

/// Splits a value at `bit`, such that for return value `(a, b)`, `value == (a << bit | b)`.
/// ```ignore
/// let value = 0b110011_0101010101;
/// let (a,b) = split_at(value, 10);
/// assert_eq!(a, 0b110011);
/// assert_eq!(b, 0b0101010101);
/// ```
pub fn split_at(value: u16, bit: u8) -> (u16, u16) {
    let bit = bit as u16;
    let split_mask = (1 << bit) - 1;
    // 'sample' in libraw terminology
    let upper = (value & (!split_mask)) >> bit;
    let lower = value & split_mask;
    (upper, lower)
}

#[cfg(test)]
mod test {
    use crate::fuji_compressed::process_common::split_at;
    use test_case::test_case;

    #[test_case(0b110011_0101010101, 10 => (0b110011, 0b0101010101))]
    #[test_case(0b1100110101010101, 0 => (0b1100110101010101, 0b0))]
    #[test_case(0b1100110101010101, 16 => panics "overflow")]
    #[test_case(0b1_100110101010101, 15 => (0b1, 0b100110101010101))]
    fn bit_diff(value: u16, at: u8) -> (u16, u16) {
        split_at(value, at)
    }
}
