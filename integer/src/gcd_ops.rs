//! Operators for finding greatest common divisor.

use crate::{ibig::IBig, sign::Sign, ubig::UBig};
use dashu_base::ring::{ExtendedGcd, Gcd};

// TODO(v0.2): implement as trait

impl UBig {
    /// Compute the greatest common divisor between self and the other operand
    ///
    /// # Example
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(12u8).gcd(&UBig::from(18u8)), 6);
    /// ```
    ///
    /// Panics if two oprands are both zero.
    #[inline]
    pub fn gcd(&self, rhs: &UBig) -> UBig {
        UBig(self.repr().gcd(rhs.repr()))
    }

    /// Compute the greatest common divisor between self and the other operand, and return
    /// both the common divisor `g` and the Bézout coefficients.
    ///
    /// # Example
    /// ```
    /// # use dashu_int::UBig;
    /// let (g, x, y) = UBig::from(12u8).gcd_ext(&UBig::from(18u8));
    /// assert!(g == 6 && x == -1 && y == 1);
    /// ```
    ///
    /// Panics if two oprands are both zero.
    #[inline]
    pub fn gcd_ext(&self, rhs: &UBig) -> (UBig, IBig, IBig) {
        let (r, s, t) = self.clone().into_repr().gcd_ext(rhs.clone().into_repr());
        (UBig(r), IBig(s), IBig(t))
    }
}

mod repr {
    use super::*;
    use crate::{
        add,
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        cmp, div, gcd, memory,
        memory::MemoryAllocation,
        mul,
        primitive::{shrink_dword, PrimitiveSigned},
        repr::{
            Repr,
            TypedRepr::{self, *},
            TypedReprRef::{self, *},
        },
    };
    use core::cmp::Ordering;

    impl<'l, 'r> Gcd<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        fn gcd(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => Repr::from_dword(dword0.gcd(dword1)),
                (RefSmall(dword0), RefLarge(words1)) => gcd_large_dword(words1, dword0),
                (RefLarge(words0), RefSmall(dword1)) => gcd_large_dword(words0, dword1),
                (RefLarge(words0), RefLarge(words1)) => gcd_large(words0.into(), words1.into()),
            }
        }
    }

    /// Perform gcd on a large number with a `Word`.
    #[inline]
    fn gcd_large_dword(buffer: &[Word], rhs: DoubleWord) -> Repr {
        if rhs == 0 {
            Repr::from_buffer(buffer.into())
        } else if let Some(word) = shrink_dword(rhs) {
            // reduce the large number by single word rhs
            let rem = div::rem_by_word(buffer, word);
            if rem == 0 {
                Repr::from_word(word)
            } else {
                Repr::from_word(rem.gcd(word))
            }
        } else {
            // reduce the large number by double word rhs
            let rem = div::rem_by_dword(buffer, rhs);
            if rem == 0 {
                Repr::from_dword(rhs)
            } else {
                Repr::from_dword(rem.gcd(rhs))
            }
        }
    }

    /// Perform gcd on two large numbers.
    #[inline]
    fn gcd_large(mut lhs: Buffer, mut rhs: Buffer) -> Repr {
        // make sure lhs > rhs
        match cmp::cmp_in_place(&lhs, &rhs) {
            Ordering::Greater => {}
            Ordering::Equal => return Repr::from_buffer(lhs),
            Ordering::Less => core::mem::swap(&mut lhs, &mut rhs),
        };

        let mut allocation =
            MemoryAllocation::new(gcd::memory_requirement_exact(lhs.len(), rhs.len()));

        let (len, swapped) = gcd::gcd_in_place(&mut lhs, &mut rhs, &mut allocation.memory());
        if swapped {
            rhs.truncate(len);
            Repr::from_buffer(rhs)
        } else {
            lhs.truncate(len);
            Repr::from_buffer(lhs)
        }
    }

    impl ExtendedGcd<TypedRepr> for TypedRepr {
        type OutputCoeff = Repr;
        type OutputGcd = Repr;

        fn gcd_ext(self, rhs: TypedRepr) -> (Repr, Repr, Repr) {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => {
                    let (g, s, t) = dword0.gcd_ext(dword1);
                    let (s_sign, s_mag) = s.to_sign_magnitude();
                    let (t_sign, t_mag) = t.to_sign_magnitude();
                    (
                        Repr::from_dword(g),
                        Repr::from_dword(s_mag).with_sign(s_sign),
                        Repr::from_dword(t_mag).with_sign(t_sign),
                    )
                }
                (Large(buffer0), Small(dword1)) => gcd_ext_large_dword(buffer0, dword1),
                (Small(dword0), Large(buffer1)) => {
                    let (g, s, t) = gcd_ext_large_dword(buffer1, dword0);
                    (g, t, s)
                }
                (Large(buffer0), Large(buffer1)) => gcd_ext_large(buffer0, buffer1),
            }
        }
    }

    /// Perform extended gcd on a large number with a `Word`.
    #[inline]
    fn gcd_ext_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> (Repr, Repr, Repr) {
        if rhs == 0 {
            (Repr::from_buffer(buffer), Repr::one(), Repr::zero())
        } else if let Some(word) = shrink_dword(rhs) {
            // reduce the large number by single word rhs
            let (g, a, b_sign) = gcd::gcd_ext_word(&mut buffer, word);
            let (a_sign, a_mag) = a.to_sign_magnitude();
            (
                Repr::from_word(g),
                Repr::from_word(a_mag).with_sign(a_sign),
                Repr::from_buffer(buffer).with_sign(b_sign),
            )
        } else {
            let (g, a, b_sign) = gcd::gcd_ext_dword(&mut buffer, rhs);
            let (a_sign, a_mag) = a.to_sign_magnitude();
            (
                Repr::from_dword(g),
                Repr::from_dword(a_mag).with_sign(a_sign),
                Repr::from_buffer(buffer).with_sign(b_sign),
            )
        }
    }

    /// Perform extended gcd on two large numbers.
    #[inline]
    fn gcd_ext_large(mut lhs: Buffer, mut rhs: Buffer) -> (Repr, Repr, Repr) {
        // make sure lhs > rhs
        let swapped = match cmp::cmp_in_place(&lhs, &rhs) {
            Ordering::Greater => false,
            Ordering::Equal => return (Repr::from_buffer(lhs), Repr::one(), Repr::zero()),
            Ordering::Less => {
                core::mem::swap(&mut lhs, &mut rhs);
                true
            }
        };
        let (lhs_len, rhs_len) = (lhs.len(), rhs.len());

        // allocate memory
        let clone_mem = memory::array_layout::<Word>(lhs_len + rhs_len);
        let gcd_mem = gcd::memory_requirement_ext_exact(lhs_len, rhs_len);
        let post_mem = memory::add_layout(
            // temporary space to store residue
            memory::array_layout::<Word>(lhs_len + rhs_len),
            memory::max_layout(
                // memory required for post processing: one multiplication + one division
                mul::memory_requirement_exact(lhs_len + rhs_len, rhs_len),
                div::memory_requirement_exact(lhs_len + rhs_len + 1, rhs_len),
            ),
        );
        let mut allocation = MemoryAllocation::new(memory::add_layout(
            clone_mem,
            memory::max_layout(gcd_mem, post_mem),
        ));
        let mut memory = allocation.memory();

        // copy oprands for post processing
        let (lhs_clone, mut memory) = memory.allocate_slice_copy(&lhs);
        let (rhs_clone, mut memory) = memory.allocate_slice_copy(&rhs);

        // actual computation
        let (g_len, b_len, b_sign) = gcd::gcd_ext_in_place(&mut lhs, &mut rhs, &mut memory);

        // the result from the internal function is g = gcd(lhs, rhs), b s.t g = b*rhs mod lhs
        // post processing: a = (g - rhs * b) / lhs
        rhs.truncate(g_len);
        let g = rhs;
        lhs.truncate(b_len);
        let b = lhs;

        // residue = g - rhs * b
        let brhs_len = rhs_clone.len() + b.len();
        let (residue, mut memory) = memory.allocate_slice_fill(brhs_len + 1, 0);
        mul::multiply(&mut residue[..brhs_len], rhs_clone, &b, &mut memory);
        match b_sign {
            Sign::Negative => {
                *residue.last_mut().unwrap() = add::add_in_place(residue, &g) as Word;
            }
            Sign::Positive => {
                let overflow = add::sub_in_place(residue, &g);
                debug_assert!(!overflow);
            }
        };

        // a = residue / lhs
        let (_, overflow) = div::div_rem_unnormalized_in_place(residue, lhs_clone, &mut memory);
        let mut a = Buffer::from(&residue[lhs_len..]);
        debug_assert_eq!(residue[0], 0); // this division is an exact division
        if overflow > 0 {
            a.push(overflow);
        }

        let g = Repr::from_buffer(g);
        let a = Repr::from_buffer(a).with_sign(-b_sign);
        let b = Repr::from_buffer(b).with_sign(b_sign);
        if swapped {
            (g, b, a)
        } else {
            (g, a, b)
        }
    }
}
