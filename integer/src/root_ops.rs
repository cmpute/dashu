use dashu_base::{Sign, RootRem};

use crate::{
    error::{panic_root_negative, panic_root_zeroth},
    ibig::IBig,
    ubig::UBig,
};

impl UBig {
    /// Calculate the square root of the integer
    #[inline]
    pub fn sqrt(&self) -> UBig {
        UBig(self.repr().sqrt())
    }

    /// Calculate the nth-root of the integer
    #[inline]
    pub fn nth_root(&self, n: usize) -> UBig {
        UBig(self.repr().nth_root(n))
    }
}

impl RootRem for UBig {
    type OutputSqrt = UBig;
    type OutputCbrt = UBig;

    #[inline]
    fn sqrt_rem(&self) -> (Self, Self) {
        let (s, r) = self.repr().sqrt_rem();
        (UBig(s), UBig(r))
    }
    #[inline]
    fn cbrt_rem(&self) -> (Self, Self) {
        let c = self.nth_root(3);
        let r = self - c.pow(3);
        (c, r)
    }
}

impl IBig {
    /// Calculate the square root of the integer
    #[inline]
    pub fn sqrt(&self) -> UBig {
        let (sign, mag) = self.as_sign_repr();
        if sign == Sign::Negative {
            panic_root_negative()
        }
        UBig(mag.sqrt())
    }

    /// Calculate the nth-root of the integer
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
    use dashu_base::{Root, RootRem};

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
                if shift > WORD_BITS_USIZE {
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

            // shortcut
            let bits = self.bit_len();
            if bits <= n {
                // the result must be 1
                return Repr::one();
            }

            // then use newton's method
            let nm1 = n - 1;
            let mut guess = UBig::ONE << (self.bit_len() / n); // underestimate
            let next = |x: &UBig| {
                let y = UBig(self / x.pow(nm1).into_repr());
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
}
