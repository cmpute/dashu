//! Logarithm

use crate::{ibig::IBig, ubig::UBig, ops::Log2Bounds};

impl UBig {
    /// Calculate the (truncated) logarithm of the [UBig]
    ///
    /// This function could takes a long time when the integer is very large.
    /// In applications where an exact result is not necessary,
    /// [log2_bounds][UBig::log2_bounds] could be used.
    ///
    /// # Panics
    ///
    /// Panics if the number is 0, or the base is 0 or 1
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let base = UBig::from(3u8);
    /// assert_eq!(UBig::from(81u8).ilog(&base), 4);
    /// assert_eq!(UBig::from(1000u16).ilog(&base), 6);
    /// ```
    #[inline]
    pub fn ilog(&self, base: &UBig) -> usize {
        self.repr().log(base.repr()).0
    }
}

impl Log2Bounds for UBig {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        self.repr().log2_bounds()
    }
}

impl IBig {
    /// Calculate the (truncated) logarithm of the magnitude of [IBig]
    ///
    /// This function could takes a long time when the integer is very large.
    /// In applications where an exact result is not necessary,
    /// [log2_bounds][IBig::log2_bounds] could be used.
    ///
    /// # Panics
    ///
    /// Panics if the number is 0, or the base is 0 or 1
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::{UBig, IBig};
    /// let base = UBig::from(3u8);
    /// assert_eq!(IBig::from(-81).ilog(&base), 4);
    /// assert_eq!(IBig::from(-1000).ilog(&base), 6);
    /// ```
    #[inline]
    pub fn ilog(&self, base: &UBig) -> usize {
        self.as_sign_repr().1.log(base.repr()).0
    }
}

impl Log2Bounds for IBig {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        self.as_sign_repr().1.log2_bounds()
    }
}

pub(crate) mod repr {
    use core::cmp::Ordering;

    use dashu_base::Log2Bounds;

    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        cmp::cmp_in_place,
        div,
        error::panic_invalid_log_oprand,
        helper_macros::debug_assert_zero,
        math::max_exp_in_word,
        mul, mul_ops, pow,
        primitive::{extend_word, highest_dword, shrink_dword, split_dword, WORD_BITS_USIZE},
        radix,
        repr::{
            Repr,
            TypedReprRef::{self, *},
        },
    };

    impl TypedReprRef<'_> {
        /// Floor logarithm, returns (log(self), base^log(self))
        pub fn log(self, base: TypedReprRef<'_>) -> (usize, Repr) {
            // shortcuts
            if let RefSmall(dw) = base {
                match dw {
                    0 | 1 => panic_invalid_log_oprand(),
                    2 => {
                        return (
                            self.bit_len() - 1,
                            Repr::zero().into_typed().set_bit(self.bit_len()),
                        )
                    }
                    b if b.is_power_of_two() => {
                        let base_bits = b.trailing_zeros() as usize;
                        let exp = (self.bit_len() - 1) / base_bits;
                        return (exp, Repr::zero().into_typed().set_bit(exp * base_bits));
                    }
                    _ => {}
                }
            }

            match (self, base) {
                (RefSmall(dword), RefSmall(base_dword)) => log_dword(dword, base_dword),
                (RefSmall(_), RefLarge(_)) => (0, Repr::one()),
                (RefLarge(words), RefSmall(base_dword)) => {
                    if let Some(base_word) = shrink_dword(base_dword) {
                        log_word_base(words, base_word)
                    } else {
                        let mut buffer: [Word; 2] = [0; 2];
                        let (lo, hi) = split_dword(base_dword);
                        buffer[0] = lo;
                        buffer[1] = hi;
                        log_large(words, &buffer)
                    }
                }
                (RefLarge(words), RefLarge(base_words)) => match cmp_in_place(words, base_words) {
                    Ordering::Less => (0, Repr::one()),
                    Ordering::Equal => (1, Repr::from_buffer(Buffer::from(words))),
                    Ordering::Greater => log_large(words, base_words),
                },
            }
        }

        pub fn log2_bounds(self) -> (f32, f32) {
            match self {
                RefSmall(dword) => dword.log2_bounds(),
                RefLarge(words) => log2_bounds_large(words)
            }
        }
    }

    fn log_dword(target: DoubleWord, base: DoubleWord) -> (usize, Repr) {
        debug_assert!(base > 1);

        // shortcuts
        match target {
            0 => panic_invalid_log_oprand(),
            1 => return (0, Repr::one()),
            i if i < base => return (0, Repr::one()),
            i if i == base => return (1, Repr::from_dword(base)),
            _ => {}
        }

        let log2_self = target.log2_bounds().0;
        let log2_base = base.log2_bounds().1;

        let mut est = (log2_self / log2_base) as u32; // float to int is underestimate
        let mut est_pow = base.pow(est);
        assert!(est_pow <= target);

        while let Some(next_pow) = est_pow.checked_mul(base) {
            let cmp = next_pow.cmp(&target);
            if cmp.is_le() {
                est_pow = next_pow;
                est += 1;
            }
            if cmp.is_ge() {
                break;
            }
        }
        (est as usize, Repr::from_dword(est_pow))
    }

    pub(crate) fn log_word_base(target: &[Word], base: Word) -> (usize, Repr) {
        let log2_self = log2_bounds_large(target).0;
        let (wexp, wbase) = if base == 10 {
            // specialize for base 10, which is cached in radix_info
            (radix::RADIX10_INFO.digits_per_word, radix::RADIX10_INFO.range_per_word)
        } else {
            max_exp_in_word(base)
        };
        let log2_wbase = wbase.log2_bounds().1;

        let mut est = (log2_self * wexp as f32 / log2_wbase) as usize; // est >= 1
        let mut est_pow = if est == 1 {
            Repr::from_word(base)
        } else {
            pow::repr::pow_word_base(base, est)
        }
        .into_buffer();
        assert!(cmp_in_place(&est_pow, target).is_le());

        // first proceed by multiplying wbase, which should happen very rarely
        while est_pow.len() < target.len() {
            if est_pow.len() == target.len() - 1 {
                let target_hi = highest_dword(target);
                let next_hi = (extend_word(*est_pow.last().unwrap()) + 1) * extend_word(wbase); // overestimate
                if next_hi > target_hi {
                    break;
                }
            }
            let carry = mul::mul_word_in_place(&mut est_pow, wbase);
            est_pow.push_resizing(carry);
            est += wexp;
        }

        // then proceed by multiplying base, which can require a few steps
        loop {
            match cmp_in_place(&est_pow, target) {
                Ordering::Less => {
                    let carry = mul::mul_word_in_place(&mut est_pow, base);
                    est_pow.push_resizing(carry);
                    est += 1;
                }
                Ordering::Equal => break,
                Ordering::Greater => {
                    // recover the over estimate
                    debug_assert_zero!(div::div_by_word_in_place(&mut est_pow, base));
                    est -= 1;
                    break;
                }
            }
        }

        (est, Repr::from_buffer(est_pow))
    }

    fn log_large(target: &[Word], base: &[Word]) -> (usize, Repr) {
        debug_assert!(cmp_in_place(target, base).is_ge()); // this ensures est >= 1

        // first estimates the result
        let log2_self = log2_bounds_large(target).0;
        let log2_base = log2_bounds_large(base).1;
        let mut est = (log2_self / log2_base) as usize; // float to int is underestimate
        est = est.max(1); // sometimes est can be zero due to estimation error
        let mut est_pow = if est == 1 {
            Repr::from_buffer(Buffer::from(base))
        } else if base.len() == 2 {
            let base_dword = highest_dword(base);
            pow::repr::pow_dword_base(base_dword, est)
        } else {
            pow::repr::pow_large_base(base, est)
        };
        assert!(cmp_in_place(est_pow.as_slice(), target).is_le());

        // then fix the error by trials
        loop {
            let next_pow = mul_ops::repr::mul_large(est_pow.as_slice(), base);
            let cmp = cmp_in_place(next_pow.as_slice(), target);
            if cmp.is_le() {
                est_pow = next_pow;
                est += 1;
            }
            if cmp.is_ge() {
                break;
            }
        }
        (est, est_pow)
    }

    #[inline]
    fn log2_bounds_large(words: &[Word]) -> (f32, f32) {
        // notice that the bit length can be larger than 2^24, so the result
        // cannot be exact even if the input is a power of two
        let hi = highest_dword(words);
        let rem_bits = (words.len() - 2) * WORD_BITS_USIZE;
        let (hi_lb, hi_ub) = hi.log2_bounds();

        /// Adjustment required to ensure floor or ceil operation
        const ADJUST: f32 = 2. * f32::EPSILON;
        let est_lb = (hi_lb + rem_bits as f32) * (1. - ADJUST);
        let est_ub = (hi_ub + rem_bits as f32) * (1. + ADJUST);
        (est_lb, est_ub)
    }
}
