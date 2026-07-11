use dashu_base::{CubicRoot, CubicRootRem, Sign, SquareRoot, SquareRootRem};

use crate::{
    error::{panic_root_negative, panic_root_zeroth},
    ibig::IBig,
    ubig::UBig,
};

impl UBig {
    /// Calculate the nth-root of the integer rounding towards zero.
    ///
    /// The result `r` is tight: `r`<sup>`n`</sup> ≤ `self` < `(r + 1)`<sup>`n`</sup>.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(4u8).nth_root(2), UBig::from(2u8));
    /// assert_eq!(UBig::from(4u8).nth_root(3), UBig::from(1u8));
    /// assert_eq!(UBig::from(1024u16).nth_root(5), UBig::from(4u8));
    /// ```
    ///
    /// # Panics
    ///
    /// If `n` is zero
    #[inline]
    pub fn nth_root(&self, n: usize) -> UBig {
        UBig(self.repr().nth_root(n))
    }
}

impl SquareRoot for UBig {
    type Output = UBig;
    #[inline]
    fn sqrt(&self) -> Self::Output {
        UBig(self.repr().sqrt())
    }
}

impl SquareRootRem for UBig {
    type Output = UBig;
    #[inline]
    fn sqrt_rem(&self) -> (Self, Self) {
        let (s, r) = self.repr().sqrt_rem();
        (UBig(s), UBig(r))
    }
}

impl CubicRoot for UBig {
    type Output = UBig;
    #[inline]
    fn cbrt(&self) -> Self::Output {
        self.nth_root(3)
    }
}

impl CubicRootRem for UBig {
    type Output = UBig;
    #[inline]
    fn cbrt_rem(&self) -> (Self::Output, Self) {
        let c = self.nth_root(3);
        let r = self - c.pow(3);
        (c, r)
    }
}

impl IBig {
    /// Calculate the nth-root of the integer rounding towards zero
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(4).nth_root(2), IBig::from(2));
    /// assert_eq!(IBig::from(-4).nth_root(3), IBig::from(-1));
    /// assert_eq!(IBig::from(-1024).nth_root(5), IBig::from(-4));
    /// ```
    ///
    /// # Panics
    ///
    /// If `n` is zero, or if `n` is even when the integer is negative.
    #[inline]
    pub fn nth_root(&self, n: usize) -> IBig {
        if n == 0 {
            panic_root_zeroth()
        }

        let (sign, mag) = self.as_sign_repr();
        if sign == Sign::Negative && n % 2 == 0 {
            panic_root_negative()
        }

        IBig(mag.nth_root(n).with_sign(sign))
    }
}

impl SquareRoot for IBig {
    type Output = UBig;
    #[inline]
    fn sqrt(&self) -> UBig {
        let (sign, mag) = self.as_sign_repr();
        if sign == Sign::Negative {
            panic_root_negative()
        }
        UBig(mag.sqrt())
    }
}

impl CubicRoot for IBig {
    type Output = IBig;
    #[inline]
    fn cbrt(&self) -> IBig {
        let (sign, mag) = self.as_sign_repr();
        if sign == Sign::Negative {
            panic_root_negative()
        }
        IBig(mag.nth_root(3).with_sign(sign))
    }
}

mod repr {
    use super::*;
    use crate::{
        add,
        arch::word::Word,
        buffer::Buffer,
        memory::MemoryAllocation,
        mul,
        primitive::{extend_word, shrink_dword, WORD_BITS, WORD_BITS_USIZE},
        repr::{
            Repr,
            TypedReprRef::{self, *},
        },
        root, shift, shift_ops,
    };
    use dashu_base::{SquareRoot, SquareRootRem};

    impl<'a> TypedReprRef<'a> {
        #[inline]
        pub fn sqrt(self) -> Repr {
            match self {
                RefSmall(dw) => {
                    if let Some(w) = shrink_dword(dw) {
                        Repr::from_word(w.sqrt() as Word)
                    } else {
                        Repr::from_word(dw.sqrt())
                    }
                }
                RefLarge(words) => sqrt_rem_large(words, true).0,
            }
        }

        #[inline]
        pub fn sqrt_rem(self) -> (Repr, Repr) {
            match self {
                RefSmall(dw) => {
                    if let Some(w) = shrink_dword(dw) {
                        let (s, r) = w.sqrt_rem();
                        (Repr::from_word(s as Word), Repr::from_word(r))
                    } else {
                        let (s, r) = dw.sqrt_rem();
                        (Repr::from_word(s), Repr::from_dword(r))
                    }
                }
                RefLarge(words) => sqrt_rem_large(words, false),
            }
        }
    }

    fn sqrt_rem_large(words: &[Word], root_only: bool) -> (Repr, Repr) {
        // first shift the words so that there are even words and
        // the top word is normalized. Note: shift <= 2 * WORD_BITS - 2
        let shift = WORD_BITS_USIZE * (words.len() & 1)
            + (words.last().unwrap().leading_zeros() & !1) as usize;
        let n = (words.len() + 1) / 2;
        let mut buffer = shift_ops::repr::shl_large_ref(words, shift).into_buffer();
        let mut out = Buffer::allocate(n);
        out.push_zeros(n);

        let mut allocation = MemoryAllocation::new(root::memory_requirement_sqrt_rem(n));
        let r_top = root::sqrt_rem(&mut out, &mut buffer, &mut allocation.memory());

        // afterwards, s = out[..], r = buffer[..n] + r_top << n*WORD_BITS
        // then recover the result if shift != 0
        if shift != 0 {
            // to get the final result, let s0 = s mod 2^(shift/2), then
            // 2^shift*n = (s-s0)^2 + 2s*s0 - s0^2 + r, so final r = (r + 2s*s0 - s0^2) / 2^shift
            if !root_only {
                let s0 = out[0] & ((1 << (shift / 2)) - 1);
                let c1 = mul::add_mul_word_in_place(&mut buffer[..n], 2 * s0, &out);
                let c2 =
                    add::sub_dword_in_place(&mut buffer[..n], extend_word(s0) * extend_word(s0));
                buffer[n] = r_top as Word + c1 - c2 as Word;
            }

            // s >>= shift/2, r >>= shift
            let _ = shift::shr_in_place(&mut out, shift as u32 / 2);
            if !root_only {
                // Use `>=` (not `>`) so the boundary case `shift == WORD_BITS_USIZE`
                // also drops a whole word. Otherwise the `shr_in_place(shift %
                // WORD_BITS) == shr_in_place(0)` below is a no-op and the
                // division by 2^shift is silently skipped. Hit on 16-bit
                // Word for inputs like `2^400 - 1` (`words.len() = 25` odd,
                // top word fully populated, so `shift = WORD_BITS + 0`).
                if shift >= WORD_BITS_USIZE {
                    shift::shr_in_place_one_word(&mut buffer);
                    buffer.truncate(n);
                } else {
                    buffer.truncate(n + 1);
                }
                let _ = shift::shr_in_place(&mut buffer, shift as u32 % WORD_BITS);
            }
        } else if !root_only {
            buffer[n] = r_top as Word;
            buffer.truncate(n + 1);
        }

        (Repr::from_buffer(out), Repr::from_buffer(buffer))
    }

    impl<'a> TypedReprRef<'a> {
        pub fn nth_root(self, n: usize) -> Repr {
            match n {
                0 => panic_root_zeroth(),
                1 => return Repr::from_ref(self),
                2 => return self.sqrt(),
                _ => {}
            }

            let bits = self.bit_len();
            if bits == 0 {
                return Repr::zero(); // the nth root of 0 is 0, not 1
            }
            if bits <= n {
                return Repr::one();
            }

            // Peel off powers of small primes (2, 3, 5, 7) to shrink the
            // exponent.  Computing x^{n-1} in Newton is prohibitively
            // expensive for large n, but reducing the exponent step-wise
            // keeps the intermediate values small.
            let (repr, remaining) = reduce_by_small_factors(self, n);
            if remaining == 1 {
                return repr;
            }
            let typed = repr.as_typed();
            if remaining == 2 {
                return typed.sqrt();
            }
            newton_nth_root(typed, remaining)
        }
    }

    /// Strip powers of 2, 3, 5, and 7 from `n`, applying the corresponding
    /// root operation at each step.  Returns `(intermediate_root, remaining_n)`.
    fn reduce_by_small_factors(num: TypedReprRef<'_>, mut n: usize) -> (Repr, usize) {
        // Factor 2 (specialized sqrt)
        let twos = n.trailing_zeros() as usize;
        n >>= twos;
        let mut repr = if twos == 0 {
            Repr::from_ref(num)
        } else {
            let mut r = num.sqrt();
            for _ in 1..twos {
                r = r.as_typed().sqrt();
            }
            r
        };

        // Factor 3
        while n % 3 == 0 {
            repr = newton_nth_root(repr.as_typed(), 3);
            n /= 3;
        }
        // Factor 5
        while n % 5 == 0 {
            repr = newton_nth_root(repr.as_typed(), 5);
            n /= 5;
        }
        // Factor 7
        while n % 7 == 0 {
            repr = newton_nth_root(repr.as_typed(), 7);
            n /= 7;
        }

        (repr, n)
    }

    /// Newton's method for the integer nth root of a non-composite n.
    fn newton_nth_root(num: TypedReprRef<'_>, n: usize) -> Repr {
        debug_assert!(n > 2);
        let nm1 = n - 1;
        let mut guess = UBig::ONE << (num.bit_len() / n); // underestimate
        let next = |x: &UBig| {
            let y = UBig(num / x.pow(nm1).into_repr());
            (y + x * nm1) / n
        };

        let mut fixpoint = next(&guess);
        // first go up then go down, to ensure an underestimate
        while fixpoint > guess {
            guess = fixpoint;
            fixpoint = next(&guess);
        }
        while fixpoint < guess {
            guess = fixpoint;
            fixpoint = next(&guess);
        }
        guess.0
    }
}
