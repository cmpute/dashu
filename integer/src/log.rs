//! Logarithm

use crate::{ubig::UBig, ibig::IBig};

impl UBig {
    /// Calculate the (truncated) logarithm of the [UBig]
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
    /// assert_eq!(UBig::from(81u8).log(&base), 4);
    /// assert_eq!(UBig::from(1000u16).log(&base), 6);
    /// ```
    #[inline]
    pub fn log(&self, base: &UBig) -> usize {
        self.repr().log(base.repr()).0
    }
}

impl IBig {
    /// Calculate the (truncated) logarithm of the absolute value of [IBig]
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
    /// assert_eq!(IBig::from(-81).log(&base), 4);
    /// assert_eq!(IBig::from(-1000).log(&base), 6);
    /// ```
    #[inline]
    pub fn log(&self, base: &UBig) -> usize {
        self.as_sign_repr().1.log(base.repr()).0
    }
}

pub(crate) mod repr {
    use core::cmp::Ordering;

    use crate::{
        error::panic_invalid_log_oprand,
        repr::{TypedReprRef::{self, *}, Repr},
        primitive::{shrink_dword, WORD_BITS_USIZE, highest_dword, split_dword, extend_word},
        math::{log2_dword_fp8, max_exp_in_word, ceil_log2_word_fp8, ceil_log2_dword_fp8},
        arch::word::{Word, DoubleWord},
        cmp::cmp_in_place, buffer::Buffer,
        pow, mul_ops, mul, div, helper_macros::debug_assert_zero, radix};

    impl TypedReprRef<'_> {
        /// Floor logarithm, returns (log(self), base^log(self))
        pub fn log(self, base: TypedReprRef<'_>) -> (usize, Repr) {
            // shortcuts
            if let RefSmall(dw) = base {
                match dw {
                    0 | 1 => panic_invalid_log_oprand(),
                    2 => return (self.bit_len() - 1, Repr::zero().into_typed().set_bit(self.bit_len())),
                    b if b.is_power_of_two() => {
                        let base_bits = b.trailing_zeros() as usize;
                        let exp = (self.bit_len() - 1) / base_bits;
                        return (exp, Repr::zero()
                            .into_typed()
                            .set_bit(exp * base_bits))
                    },
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
                },
                (RefLarge(words), RefLarge(base_words)) => {
                    match cmp_in_place(words, base_words) {
                        Ordering::Less => (0, Repr::one()),
                        Ordering::Equal => (1, Repr::from_buffer(Buffer::from(words)) ),
                        Ordering::Greater => log_large(words, base_words)
                    }
                }
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

        let log2_self = log2_dword_fp8(target);
        let log2_base = ceil_log2_dword_fp8(base);

        let mut est = log2_self / log2_base;
        let mut est_pow = base.pow(est);

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
        let log2_self = log2_large_fp8(target);
        let (wexp, wbase) = if base == 10 {
            // specialize for base 10, which is cached in radix_info
            (radix::RADIX10_INFO.digits_per_word, radix::RADIX10_INFO.range_per_word)
        } else {
            max_exp_in_word(base)
        };
        let log2_wbase = ceil_log2_word_fp8(wbase) as usize;

        let mut est = log2_self * wexp / log2_wbase; // est >= 1
        let mut est_pow = if est == 1 {
            Repr::from_word(base)
        } else {
            pow::repr::pow_word_base(base, est)
        }.into_buffer();

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
                },
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
        let mut est = if target.len() < usize::MAX / 8 {
            let log2_self = log2_large_fp8(target);
            let log2_base = ceil_log2_large_fp8(base);
            log2_self / log2_base
        } else {
            // the target is too large, use very coarse estimation
            // to prevent overflow in log2_large_fp8
            let log2_self = target.len() * WORD_BITS_USIZE - target.last().unwrap().leading_zeros() as usize;
            let log2_base = (base.len() + 1) * WORD_BITS_USIZE; // ceiling log
            log2_self / log2_base
        }.max(1); // est >= 1
        let mut est_pow = if est == 1 {
            Repr::from_buffer(Buffer::from(base))
        } else if base.len() == 2 {
            let base_dword = highest_dword(base);
            pow::repr::pow_dword_base(base_dword, est)
        } else {
            pow::repr::pow_large_base(base, est)
        };

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
    fn log2_large_fp8(words: &[Word]) -> usize {
        let log2_hi = log2_dword_fp8(highest_dword(words)) as usize;
        log2_hi + (words.len() - 2) * WORD_BITS_USIZE
    }

    #[inline]
    fn ceil_log2_large_fp8(words: &[Word]) -> usize {
        let hi = highest_dword(words);
        let log2_hi = if hi.is_power_of_two() {
            (hi.trailing_zeros() as usize * 256) + 1
        } else {
            ceil_log2_dword_fp8(hi) as usize
        };
        log2_hi + (words.len() - 2) * WORD_BITS_USIZE
    }
}
