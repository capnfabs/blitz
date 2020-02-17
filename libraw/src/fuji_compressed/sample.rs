// These gradients *adapt over time*. They're comprised of two numbers:
// - The first is SUM(ABS(difference between actual value and weighted average of previous pixels))
// - The second is COUNT(processed pixels).
// Periodically, they're 'squashed down' by dividing both values by two.
// Two talk about the effect of this action, it's important to talk about what they're used for.
// These two numbers are used as follows:
// The 'bit diff' between the two is computed, which is effectively something vaguely logarithmic? I don't really understand how this works, and I probably should.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grad(i32, i32);

pub type Gradients = [[Grad; 41]; 3];

// hardcoded for 14-bit sample size.
const GRADIENT_START_SUM_VALUE: i32 = 256;
const GRADIENT_MAX_COUNT: i32 = 64;

impl Grad {
    pub fn bit_diff(self) -> usize {
        let Grad(a, b) = self;
        let a = a as usize;
        let b = b as usize;

        if b < a {
            let mut dec_bits = 1;
            while dec_bits <= 12 && (b << dec_bits) < a {
                dec_bits += 1;
            }
            dec_bits
        } else {
            0
        }
    }

    pub fn update_from_value(&mut self, value: i32) {
        self.0 += value;
        if self.1 == GRADIENT_MAX_COUNT {
            self.0 /= 2;
            self.1 /= 2;
        }
        self.1 += 1;
    }
}

impl Default for Grad {
    fn default() -> Self {
        Grad(GRADIENT_START_SUM_VALUE, 1)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Sample {
    // Just the 'upper' section.
    JustUpper(u16, bool),
    // This represents the _entire delta_ between the weighted average and the
    // actual value. Use this when we're unable to use split-encoding because
    // we've got a large value of `upper`.
    // the second part of this tuple indicates whether the value should be
    // inverted, i.e. made negative.
    EntireDelta(u16, bool),
    // This is the default 'split encoding' mechanism.
    SplitDelta {
        upper: u16,
        lower: u16,
        lower_bits: usize,
        invert: bool,
    },
}

#[cfg(test)]
mod test {
    use crate::fuji_compressed::sample::Grad;
    use test_case::test_case;

    #[test_case(Grad(256, 1) => 8)]
    #[test_case(Grad(256, 2) => 7)]
    #[test_case(Grad(256, 3) => 7)]
    #[test_case(Grad(397, 32) => 4)]
    #[test_case(Grad(397, 63) => 3)]
    #[test_case(Grad(397, 64) => 3)]
    #[test_case(Grad(397, 65) => 3)]
    #[test_case(Grad(397, 140) => 2)]
    #[test_case(Grad(397, 141) => 2)]
    #[test_case(Grad(397, 142) => 2)]
    fn bit_diff(grad: Grad) -> usize {
        grad.bit_diff()
    }

    #[test_case(Grad(256, 1), 100 => Grad(356, 2); "works outside wrapping range")]
    #[test_case(Grad(14343, 63), 100 => Grad(14443, 64); "at the wrapping boundary")]
    #[test_case(Grad(14343, 64), 1 => Grad(7172, 33); "test wrapping")]
    fn update(mut grad: Grad, update_val: i32) -> Grad {
        grad.update_from_value(update_val);
        grad
    }
}
