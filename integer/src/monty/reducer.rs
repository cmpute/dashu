//! The [`Reducer`] bridge between the Montgomery ring and the generic
//! modular-arithmetic layer, paralleling the [`ConstDivisor`] reducer in
//! [modular](crate::modular). It supports odd moduli via Montgomery
//! multiplication — the fastest choice for primality testing style workloads
//! (repeated modular multiplications) in the 256–4096-bit range.

use num_modular::Reducer;

use super::repr::{to_exact_words, MontgomeryInner, MontgomeryLargeVal, MontgomeryReprData};
use super::{Montgomery, MontgomeryRepr};
use crate::{buffer::Buffer, repr::Repr, ubig::UBig};

impl MontgomeryRepr {
    /// Get the modulus of the ring.
    pub(crate) fn modulus(&self) -> UBig {
        match self.data() {
            MontgomeryReprData::Single(r) => UBig::from_word(r.0.modulus()),
            MontgomeryReprData::Double(r) => UBig::from_dword(r.0.modulus()),
            MontgomeryReprData::Large(r) => UBig(Repr::from_buffer(Buffer::from(&r.modulus[..]))),
        }
    }
}

/// Re-interpret a raw Montgomery-form value (stored in a plain `UBig`) as a
/// [`Montgomery`] value in the given ring.
fn monty_from_raw<'a>(raw: &UBig, ring: &'a MontgomeryRepr) -> Montgomery<'a> {
    match ring.data() {
        MontgomeryReprData::Single(r) => Montgomery::from_single(raw.to_word(), r),
        MontgomeryReprData::Double(r) => Montgomery::from_double(raw.to_dword(), r),
        MontgomeryReprData::Large(r) => {
            Montgomery::from_large(MontgomeryLargeVal(to_exact_words(raw, r.modulus.len())), r)
        }
    }
}

/// Convert a [`Montgomery`] value into its raw Montgomery-form representation.
fn monty_into_raw(m: Montgomery<'_>) -> UBig {
    match m.into_repr() {
        MontgomeryInner::Single(w, _) => UBig::from_word(w),
        MontgomeryInner::Double(d, _) => UBig::from_dword(d),
        MontgomeryInner::Large(v, _) => UBig::from_words(&v.0),
    }
}

impl Reducer<UBig> for MontgomeryRepr {
    /// # Panics
    ///
    /// Panics if the modulus is even or smaller than 2. Montgomery reduction
    /// is only defined for odd moduli greater than one.
    #[inline]
    fn new(m: &UBig) -> Self {
        MontgomeryRepr::new(m.clone())
    }

    #[inline]
    fn transform(&self, target: UBig) -> UBig {
        monty_into_raw(self.reduce(target))
    }

    #[inline]
    fn check(&self, target: &UBig) -> bool {
        // Montgomery-form values are kept in the canonical range [0, m)
        *target < self.modulus()
    }

    #[inline]
    fn modulus(&self) -> UBig {
        MontgomeryRepr::modulus(self)
    }

    #[inline]
    fn residue(&self, target: UBig) -> UBig {
        monty_from_raw(&target, self).residue()
    }

    #[inline]
    fn is_zero(&self, target: &UBig) -> bool {
        // zero is its own Montgomery form
        *target == UBig::ZERO
    }

    #[inline]
    fn add(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        monty_into_raw(monty_from_raw(lhs, self) + monty_from_raw(rhs, self))
    }

    #[inline]
    fn dbl(&self, target: UBig) -> UBig {
        monty_into_raw(monty_from_raw(&target, self).dbl())
    }

    fn sub(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        monty_into_raw(monty_from_raw(lhs, self) - monty_from_raw(rhs, self))
    }

    #[inline]
    fn neg(&self, target: UBig) -> UBig {
        monty_into_raw(-monty_from_raw(&target, self))
    }

    #[inline]
    fn mul(&self, lhs: &UBig, rhs: &UBig) -> UBig {
        monty_into_raw(monty_from_raw(lhs, self) * monty_from_raw(rhs, self))
    }

    fn inv(&self, target: UBig) -> Option<UBig> {
        monty_from_raw(&target, self).inv().map(monty_into_raw)
    }

    #[inline]
    fn sqr(&self, target: UBig) -> UBig {
        monty_into_raw(monty_from_raw(&target, self).sqr())
    }

    #[inline]
    fn pow(&self, base: UBig, exp: &UBig) -> UBig {
        monty_into_raw(monty_from_raw(&base, self).pow(exp))
    }
}

#[cfg(test)]
mod tests {
    use crate::{fast_div::ConstDivisor, ubig::UBig};
    use dashu_base::BitTest as _;

    use super::*;

    fn u(v: u8) -> UBig {
        UBig::from(v)
    }

    /// Big odd prime with word-sized, double-word-sized and large variants
    const M_ODD: u128 = 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffed;

    fn assert_reducer_semantics<R: Reducer<UBig> + Clone>(reducer: R, m: &UBig, odd_only: bool) {
        let a = UBig::from(12345u32) % m;
        let b = UBig::from(6789u32) % m;

        assert_eq!(reducer.modulus(), *m);

        // round trip through the reduced form
        assert_eq!(reducer.residue(reducer.transform(a.clone())), a);
        // reduced forms pass the check, plain values out of range do not
        assert!(reducer.check(&reducer.transform(a.clone())));
        assert!(!reducer.check(m));

        let ta = reducer.transform(a.clone());
        let tb = reducer.transform(b.clone());

        // arithmetic in reduced form agrees with plain arithmetic modulo m
        assert_eq!(reducer.residue(reducer.add(&ta, &tb)), (&a + &b) % m);
        assert_eq!(reducer.residue(reducer.sub(&ta, &tb)), (&a + m - &b) % m);
        assert_eq!(reducer.residue(reducer.mul(&ta, &tb)), (&a * &b) % m);
        assert_eq!(reducer.residue(reducer.sqr(ta.clone())), (&a * &a) % m);
        assert_eq!(reducer.residue(reducer.dbl(ta.clone())), (&a << 1) % m);
        assert_eq!(reducer.residue(reducer.neg(ta.clone())), (m - &a) % m);
        assert!(reducer.is_zero(&reducer.transform(UBig::ZERO)));

        // modular exponentiation (small and multi-word exponents), checked
        // against an independent square-and-multiply implementation
        let naive_powmod = |base: &UBig, exp: &UBig| -> UBig {
            let mut acc = UBig::ONE;
            let mut sq = base % m;
            for i in 0..exp.bit_len() {
                if exp.bit(i) {
                    acc = &acc * &sq % m;
                }
                sq = &sq * &sq % m;
            }
            acc
        };
        assert_eq!(reducer.residue(reducer.pow(ta.clone(), &UBig::from(3u8))), (&a * &a * &a) % m);
        let big_exp = UBig::from(2u8).pow(100) - UBig::ONE;
        assert_eq!(reducer.residue(reducer.pow(ta.clone(), &big_exp)), naive_powmod(&a, &big_exp));

        // modular inverse: a * inv(a) = 1 (requires gcd(a, m) == 1)
        if odd_only {
            let ta_inv = reducer
                .inv(ta.clone())
                .expect("inverse exists for prime modulus");
            assert_eq!(reducer.residue(reducer.mul(&ta, &ta_inv)), UBig::ONE);
        }
    }

    #[test]
    fn reducer_montgomery_single_word() {
        let m = u(101);
        let reducer = MontgomeryRepr::new(m.clone());
        assert_reducer_semantics(reducer, &m, true);
    }

    #[test]
    fn reducer_montgomery_double_word() {
        // modulus larger than a single word
        let m = UBig::from(0x7fff_ffff_ffff_ffedu128);
        let reducer = MontgomeryRepr::new(m.clone());
        assert_reducer_semantics(reducer, &m, true);
    }

    #[test]
    fn reducer_montgomery_large() {
        // 128-bit modulus exercises the multi-word REDC path
        let m = UBig::from(M_ODD);
        let reducer = MontgomeryRepr::new(m.clone());
        assert_reducer_semantics(reducer, &m, true);
    }

    #[test]
    fn reducer_const_divisor() {
        let m = UBig::from(M_ODD);
        let reducer = ConstDivisor::new(m.clone());
        assert_reducer_semantics(reducer.clone(), &m, true);

        // even modulus works for the division based reducer
        let m_even = UBig::from(1000003u32) << 4;
        let reducer = ConstDivisor::new(m_even.clone());
        assert_reducer_semantics(reducer, &m_even, false);
    }

    #[test]
    fn reducer_montgomery_rejects_even_modulus() {
        let result = std::panic::catch_unwind(|| MontgomeryRepr::new(UBig::from(100u32)));
        assert!(result.is_err());
    }
}
