//! Logarithm

use crate::{ibig::IBig, ubig::UBig};

impl UBig {
    /// Calculate the (truncated) logarithm of the [UBig]
    ///
    /// This function could takes a long time when the integer is very large.
    /// In applications where an exact result is not necessary,
    /// [log2f_bounds][UBig::log2f_bounds] could be used.
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

    /// Estimate the bounds of the binary logarithm.
    ///
    /// The result is `(lower bound, upper bound)` such that lower bound ≤ log2(self) ≤ upper bound.
    /// The precision of the bounds is at least 8 bits (relative error < 2^-8).
    /// 
    /// With `std` disabled, the precision is about 13 bits. With `std` enabled, the precision
    /// will be full 23 bits.
    ///
    /// # Panics
    ///
    /// Panics if the number is 0
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let lb3 = 1.584962500721156f32;
    /// let (lb3_lb, lb3_ub) = UBig::from(3u8).log2_bounds();
    /// assert!(lb3_lb <= lb3 && lb3 <= lb3_ub);
    /// assert!((lb3 - lb3_lb) / lb3 < 1. / 256.);
    /// assert!((lb3_ub - lb3) / lb3 <= 1. / 256.);
    /// ```
    #[inline]
    pub fn log2_bounds(&self) -> (f32, f32) {
        let repr = self.repr();
        (repr.log2_lb(), repr.log2_ub())
    }
}

impl IBig {
    /// Calculate the (truncated) logarithm of the magnitude of [IBig]
    ///
    /// This function could takes a long time when the integer is very large.
    /// In applications where an exact result is not necessary,
    /// [log2f_bounds][IBig::log2f_bounds] could be used.
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

    /// Estimate the bounds of the binary logarithm on the magnitude.
    ///
    /// See the documentation of [UBig::log2_bounds] for the precision behavior.
    ///
    /// # Panics
    ///
    /// Panics if the number is 0
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// let lb3 = 1.584962500721156f32;
    /// let (lb3_lb, lb3_ub) = IBig::from(-3).log2_bounds();
    /// assert!(lb3_lb <= lb3 && lb3 <= lb3_ub);
    /// assert!((lb3 - lb3_lb) / lb3 < 1. / 256.);
    /// assert!((lb3_ub - lb3) / lb3 <= 1. / 256.);
    /// ```
    #[inline]
    pub fn log2_bounds(&self) -> (f32, f32) {
        let repr = self.as_sign_repr().1;
        (repr.log2_lb(), repr.log2_ub())
    }
}

pub(crate) mod repr {
    use core::cmp::Ordering;

    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        cmp::cmp_in_place,
        div,
        error::panic_invalid_log_oprand,
        helper_macros::debug_assert_zero,
        math::max_exp_in_word,
        mul, mul_ops, pow,
        primitive::{
            extend_word, highest_dword, shrink_dword, split_dword, WORD_BITS_USIZE,
        },
        radix,
        repr::{
            Repr,
            TypedReprRef::{self, *},
        },
    };

    #[cfg(not(feature = "std"))]
    use crate::math::{ceil_log2_dword_fp8, ceil_log2_word_fp8, log2_dword_fp8, log2_word_fp8, max_exp_in_dword};

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

        /// Lower bound of log2(self)
        pub fn log2_lb(self) -> f32 {
            match self {
                RefSmall(dword) => {
                    if dword == 0 {
                        panic_invalid_log_oprand()
                    }
                    if dword.is_power_of_two() {
                        // this int to float conversion is lossless
                        return dword.trailing_zeros() as f32;
                    }
                    #[cfg(not(feature = "std"))]
                    if dword < 1 << (Word::BITS / 2) {
                        // log2_word_fp8 is not accurate when the base is too small
                        // we first raise the base to a pow under a DoubleWord
                        let (exp, pow) = max_exp_in_dword(dword as Word);
                        let shift = crate::primitive::WORD_BITS - pow.leading_zeros();
                        let est = log2_word_fp8((pow >> shift) as Word) + shift * 256;
                        return est as f32 / (exp as f32 * 256.);
                    }
                    log2_dword(dword)
                }
                RefLarge(words) => log2_large(words),
            }
        }

        /// Upper bound of log2(self)
        pub fn log2_ub(self) -> f32 {
            match self {
                RefSmall(dword) => {
                    if dword == 0 {
                        panic_invalid_log_oprand()
                    }
                    if dword.is_power_of_two() {
                        // this int to float conversion is lossless
                        return dword.trailing_zeros() as f32;
                    }
                    #[cfg(not(feature = "std"))]
                    if dword < 1 << (Word::BITS / 2) {
                        // ceil_log2_word_fp8 is not accurate when the base is too small
                        // we first raise the base to a pow under a DoubleWord
                        let (exp, pow) = max_exp_in_dword(dword as Word);
                        let shift = crate::primitive::WORD_BITS - pow.leading_zeros();
                        let est = ceil_log2_word_fp8((pow >> shift) as Word) + shift * 256;
                        return est as f32 / (exp as f32 * 256.);
                    }
                    ceil_log2_dword(dword)
                }
                RefLarge(words) => ceil_log2_large(words),
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

        let log2_self = log2_dword(target);
        let log2_base = ceil_log2_dword(base);

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
        let log2_self = log2_large(target);
        let (wexp, wbase) = if base == 10 {
            // specialize for base 10, which is cached in radix_info
            (radix::RADIX10_INFO.digits_per_word, radix::RADIX10_INFO.range_per_word)
        } else {
            max_exp_in_word(base)
        };
        let log2_wbase = ceil_log2_word(wbase);

        let mut est = (log2_self * wexp as f32 / log2_wbase) as usize; // est >= 1
        let mut est_pow = if est == 1 {
            Repr::from_word(base)
        } else {
            pow::repr::pow_word_base(base, est)
        }
        .into_buffer();
        assert!(cmp_in_place(&est_pow, target).is_le());

        // first proceed by multiplying wbase, which happens very rarely
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
        let log2_self = log2_large(target);
        let log2_base = ceil_log2_large(base);
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
    #[cfg(not(feature = "std"))]
    fn log2_dword(dword: DoubleWord) -> f32 {
        log2_dword_fp8(dword) as f32 / 256.0
    }

    #[inline]
    #[cfg(not(feature = "std"))]
    fn ceil_log2_word(word: Word) -> f32 {
        ceil_log2_word_fp8(word) as f32 / 256.0
    }

    #[inline]
    #[cfg(not(feature = "std"))]
    fn ceil_log2_dword(dword: DoubleWord) -> f32 {
        ceil_log2_dword_fp8(dword) as f32 / 256.0
    }

    /// Adjustment required to ensure floor or ceil operation
    const LOG2_ADJUST: f32 = 2. * f32::EPSILON;

    #[cfg(feature = "std")]
    macro_rules! log2_using_f32 {
        ($n:ident, $ceil:literal) => {{
            if $n.is_power_of_two() {
                $n.trailing_zeros() as f32
            } else {
                const ADJUST: f32 = if $ceil {
                    (1. + LOG2_ADJUST)
                } else {
                    (1. - LOG2_ADJUST)
                };

                let nbits = crate::math::bit_len($n);
                if nbits > 24 {
                    // 24bit integer converted to f32 is lossless
                    let shifted = if $ceil {
                        ($n >> (nbits - 24)) + 1
                    } else {
                        $n >> (nbits - 24)
                    };
                    let est = if shifted.is_power_of_two() {
                        shifted.trailing_zeros() as f32 
                    } else {
                        (shifted as f32).log2() * ADJUST
                    };
                    est + (nbits - 24) as f32
                } else {
                    ($n as f32).log2() * ADJUST
                }
            }
        }};
    }

    #[inline]
    #[cfg(feature = "std")]
    fn log2_dword(dword: DoubleWord) -> f32 {
        log2_using_f32!(dword, false)
    }

    #[inline]
    #[cfg(feature = "std")]
    fn ceil_log2_word(word: Word) -> f32 {
        log2_using_f32!(word, true)
    }
    
    #[inline]
    #[cfg(feature = "std")]
    fn ceil_log2_dword(dword: DoubleWord) -> f32 {
        log2_using_f32!(dword, true)
    }

    #[inline]
    fn log2_large(words: &[Word]) -> f32 {
        // notice that the bit length can be larger than 2^24, so the result
        // cannot be exact even if the input is a power of two
        let hi = highest_dword(words);
        let rem_bits = (words.len() - 2) * WORD_BITS_USIZE;
        let est = if hi.is_power_of_two() {
            (hi.trailing_zeros() as usize + rem_bits) as f32
        } else {
            log2_dword(hi) + rem_bits as f32
        };
        est * (1. - LOG2_ADJUST) // ensure underesitmation
    }

    #[inline]
    fn ceil_log2_large(words: &[Word]) -> f32 {
        // notice that the bit length can be larger than 2^24, so the result
        // cannot be exact even if the input is a power of two
        let hi = highest_dword(words);
        let rem_bits = (words.len() - 2) * WORD_BITS_USIZE;
        let est = if hi.is_power_of_two() {
            (hi.trailing_zeros() as usize + rem_bits) as f32
        } else {
            ceil_log2_dword(hi) + rem_bits as f32
        };
        est * (1. + LOG2_ADJUST) // ensure overestimation
    }
}
