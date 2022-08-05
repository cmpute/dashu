//! Exponentiation.

use core::ops::{Shl, Shr};

use crate::{ibig::IBig, sign::Sign::*, ubig::UBig};

impl UBig {
    /// Raises self to the power of `exp`.
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(3u8).pow(3), 27);
    /// ```
    #[inline]
    pub fn pow(&self, exp: usize) -> UBig {
        // remove factor 2 before actual powering
        let shift = self.trailing_zeros().unwrap_or(0);
        let result = if shift != 0 {
            self.repr()
                .shr(shift)
                .as_typed()
                .pow(exp)
                .into_typed()
                .shl(exp * shift)
        } else {
            self.repr().pow(exp)
        };
        UBig(result)
    }
}

impl IBig {
    /// Raises self to the power of `exp`.
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(-3).pow(3), -27);
    /// ```
    #[inline]
    pub fn pow(&self, exp: usize) -> IBig {
        let (sign, mag) = self.as_sign_repr();
        let sign = if sign == Negative && exp % 2 == 1 {
            Negative
        } else {
            Positive
        };

        // remove factor 2 before actual powering
        let shift = mag.trailing_zeros().unwrap_or(0);
        let result = if shift != 0 {
            mag.shr(shift)
                .as_typed()
                .pow(exp)
                .into_typed()
                .shl(exp * shift)
        } else {
            mag.pow(exp)
        };
        IBig(result.with_sign(sign))
    }
}

pub(crate) mod repr {
    use dashu_base::DivRem;

    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        math::{self, bit_len, max_exp_in_word},
        memory::{self, MemoryAllocation},
        mul, mul_ops,
        primitive::{extend_word, shrink_dword, split_dword},
        repr::{
            Repr,
            TypedReprRef::{self, *},
        },
        sqr,
    };

    impl TypedReprRef<'_> {
        pub fn pow(&self, exp: usize) -> Repr {
            // shortcuts
            match exp {
                0 => return Repr::one(),
                1 => {
                    return match *self {
                        Self::RefSmall(dw) => Repr::from_dword(dw),
                        Self::RefLarge(words) => Repr::from_buffer(Buffer::from(words)),
                    }
                }
                2 => return self.square(),
                _ => {}
            };

            match self {
                RefSmall(dword) => {
                    if let Some(word) = shrink_dword(*dword) {
                        pow_word_base(word, exp)
                    } else {
                        pow_dword_base(*dword, exp)
                    }
                }
                RefLarge(words) => pow_large_base(words, exp),
            }
        }
    }

    pub(crate) fn pow_word_base(base: Word, exp: usize) -> Repr {
        debug_assert!(exp > 1);
        match base {
            0 => return Repr::zero(),
            1 => return Repr::one(),
            2 => return Repr::zero().into_typed().set_bit(exp),
            b if b.is_power_of_two() => {
                return Repr::zero()
                    .into_typed()
                    .set_bit(exp * base.trailing_zeros() as usize)
            }
            _ => {}
        }

        // lift the base to a full word and some shortcuts
        let (wexp, wbase) = max_exp_in_word(base);
        if exp < wexp {
            return Repr::from_word(base.pow(exp as u32));
        } else if exp < 2 * wexp {
            let pow = base.pow((exp - wexp) as u32);
            return Repr::from_dword(extend_word(wbase) * extend_word(pow));
        }

        // by now wexp / exp >= 2, result = wbase ^ (wexp / exp) * base ^ (wexp % exp)
        let (exp, exp_rem) = exp.div_rem(wexp);
        let mut res = Buffer::allocate(exp + 1); // result is at most exp + 1 words
        let mut allocation = MemoryAllocation::new(
            memory::add_layout(
                memory::array_layout::<Word>(exp / 2 + 1), // store res before squaring
                sqr::memory_requirement_exact(exp / 2 + 1),
            ), // memory for squaring
        );
        let mut memory = allocation.memory();

        // res = wbase * wbase
        let mut p = bit_len(exp) - 2;
        let (lo, hi) = split_dword(extend_word(wbase) * extend_word(wbase));
        res.push(lo);
        res.push(hi);

        loop {
            if exp & (1 << p) != 0 {
                let carry = mul::mul_word_in_place(&mut res, wbase);
                res.push_resizing(carry); // actually never resize
            }
            if p == 0 {
                break;
            }
            p -= 1;

            // res = square(res)
            let (tmp, mut memory) = memory.allocate_slice_copy(&res);
            res.fill(0);
            res.push_zeros(res.len());
            sqr::square(&mut res, tmp, &mut memory);
        }

        // carry out the remaining multiplications
        let pow_rem = base.pow(exp_rem as u32);
        let carry = mul::mul_word_in_place(&mut res, pow_rem);
        res.push_resizing(carry);
        Repr::from_buffer(res)
    }

    pub(crate) fn pow_dword_base(base: DoubleWord, exp: usize) -> Repr {
        debug_assert!(exp > 1);
        debug_assert!(base > Word::MAX as DoubleWord);

        let mut res = Buffer::allocate(2 * exp); // result is at most 2 * exp words
        let mut allocation = MemoryAllocation::new(
            memory::add_layout(
                memory::array_layout::<Word>(exp), // store res before squaring
                sqr::memory_requirement_exact(exp),
            ), // memory for squaring
        );
        let mut memory = allocation.memory();

        // res = base * base
        let mut p = bit_len(exp) - 2;
        let (lo, hi) = math::mul_add_carry_dword(base, base, 0);
        let (n0, n1) = split_dword(lo);
        res.push(n0);
        res.push(n1);
        let (n2, n3) = split_dword(hi);
        res.push(n2);
        res.push(n3);

        loop {
            if exp & (1 << p) != 0 {
                let carry = mul::mul_dword_in_place(&mut res, base);
                if carry > 0 {
                    let (c0, c1) = split_dword(carry);
                    res.push(c0);
                    res.push_resizing(c1); // actually never resize
                }
            }
            if p == 0 {
                break;
            }
            p -= 1;

            // res = square(res)
            let (tmp, mut memory) = memory.allocate_slice_copy(&res);
            res.fill(0);
            res.push_zeros(res.len());
            sqr::square(&mut res, tmp, &mut memory);
        }

        Repr::from_buffer(res)
    }

    pub(crate) fn pow_large_base(base: &[Word], exp: usize) -> Repr {
        debug_assert!(exp > 1);
        let mut p = bit_len(exp) - 2;
        let mut res = mul_ops::repr::square_large(base);
        loop {
            if exp & (1 << p) != 0 {
                res = mul_ops::repr::mul_large(res.as_slice(), base);
            }
            if p == 0 {
                break;
            }
            p -= 1;
            res = mul_ops::repr::square_large(res.as_slice());
        }
        res
    }
}
