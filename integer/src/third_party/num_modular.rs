//! Implement num-modular traits.
//!
//! This module implements [ModularSymbols] (the Jacobi/Legendre/Kronecker
//! symbols) for [UBig]. Together with the `Reducer` implementations for the
//! rings of the [modular](crate::modular) and [monty](crate::monty) modules,
//! it makes `UBig` usable as the base integer type for the carried-modulus
//! integers of the `num-modular` ecosystem (e.g. `num_prime::mint::Mint`).

use num_modular::ModularSymbols;

use crate::{fast_div::ConstDivisor, ubig::UBig};
use dashu_base::BitTest;

impl ModularSymbols<&UBig> for UBig {
    #[inline]
    fn checked_legendre(&self, n: &UBig) -> Option<i8> {
        let ring = ConstDivisor::new(n.clone());
        let r = ring
            .reduce(self.clone())
            .pow(&((n - UBig::ONE) >> 1))
            .residue();
        if r.is_zero() {
            Some(0)
        } else if r == UBig::ONE {
            Some(1)
        } else if r + UBig::ONE == *n {
            Some(-1)
        } else {
            None
        }
    }

    fn checked_jacobi(&self, n: &UBig) -> Option<i8> {
        // the Jacobi symbol is only defined for positive odd integers
        if !n.bit(0) {
            return None;
        }
        if self.is_zero() {
            return Some(if n == &UBig::ONE { 1 } else { 0 });
        }
        if self == &UBig::ONE {
            return Some(1);
        }

        let mut a = self % n;
        let mut n = n.clone();
        let mut t: i8 = 1;
        while !a.is_zero() {
            // strip factors of two from a, tracking their effect on the symbol:
            // (2|n) = -1 iff n mod 8 in {3,5}, i.e. bit 0 set and bit 1 != bit 2
            let s = a.trailing_zeros().unwrap_or(0);
            if s > 0 {
                if s & 1 == 1 && n.bit(1) != n.bit(2) {
                    t = -t;
                }
                a = &a >> s;
            }

            // (a|n) = (n|a) * (-1)^((a-1)/2 * (n-1)/2), the extra sign is -1
            // only if both a and n are 3 mod 4
            core::mem::swap(&mut a, &mut n);
            if a.bit(0) && a.bit(1) && n.bit(0) && n.bit(1) {
                t = -t;
            }
            a %= &n;
        }
        Some(if n == UBig::ONE { t } else { 0 })
    }

    fn kronecker(&self, n: &UBig) -> i8 {
        if n.is_zero() {
            return if self == &UBig::ONE { 1 } else { 0 };
        }
        if n == &UBig::ONE {
            return 1;
        }
        if self.is_zero() {
            return 0;
        }
        if *n == UBig::from(2u8) {
            // (a|2) = 1 if a mod 8 in {1,7}, -1 if in {3,5}, 0 if a is even
            return if !self.bit(0) {
                0
            } else if self.bit(1) == self.bit(2) {
                1
            } else {
                -1
            };
        }

        let s = n.trailing_zeros().unwrap_or(0);
        if s == 0 {
            // n is odd, the Kronecker symbol equals the Jacobi symbol
            return self.checked_jacobi(n).unwrap_or(0);
        }
        let n_odd = n >> s;
        let t1 = self.kronecker(&UBig::from(2u8));
        if t1 == 0 {
            return 0;
        }
        let t2 = self.checked_jacobi(&n_odd).unwrap_or(0);
        if s & 1 == 0 {
            t2
        } else {
            t1 * t2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(v: u8) -> UBig {
        UBig::from(v)
    }

    #[test]
    fn jacobi_symbol() {
        // hand-computed cases, cross-checked by Euler's criterion
        assert_eq!(u(2).checked_jacobi(&u(5)), Some(-1));
        assert_eq!(u(2).checked_jacobi(&u(7)), Some(1));
        assert_eq!(u(2).checked_jacobi(&u(9)), Some(1)); // (2|3)^2
        assert_eq!(u(3).checked_jacobi(&u(5)), Some(-1));
        assert_eq!(u(4).checked_jacobi(&u(5)), Some(1));
        assert_eq!(u(5).checked_jacobi(&u(5)), Some(0));
        assert_eq!(u(6).checked_jacobi(&u(7)), Some(-1)); // 6 = -1 (mod 7)
        assert_eq!(u(5).checked_jacobi(&u(7)), Some(-1));
        assert_eq!(u(2).checked_jacobi(&u(15)), Some(1)); // (2|3)(2|5)
        assert_eq!(u(14).checked_jacobi(&u(15)), Some(-1));
        assert_eq!(u(0).checked_jacobi(&u(9)), Some(0));
        assert_eq!(u(0).checked_jacobi(&u(1)), Some(1));
        assert_eq!(u(1).checked_jacobi(&u(9)), Some(1));

        // not defined for even modulus
        assert_eq!(u(3).checked_jacobi(&u(8)), None);

        // reciprocity: (a|n)(n|a) = -1 when both are 3 mod 4, +1 otherwise
        assert_eq!(u(5).checked_jacobi(&u(21)), u(21).checked_jacobi(&u(5)));
        assert_eq!(u(11).checked_jacobi(&u(13)), u(13).checked_jacobi(&u(11)));
        assert_eq!(u(13).checked_jacobi(&u(17)), u(17).checked_jacobi(&u(13)));
        assert_eq!(u(7).checked_jacobi(&u(15)), Some(-1)); // (7|3)(7|5)
        assert_eq!(u(15).checked_jacobi(&u(7)), Some(1));

        // multiplicativity in the first argument
        assert_eq!(
            u(6).checked_jacobi(&u(35)),
            Some(u(2).checked_jacobi(&u(35)).unwrap() * u(3).checked_jacobi(&u(35)).unwrap())
        );
    }

    #[test]
    fn legendre_symbol() {
        // 7 is prime
        assert_eq!(u(2).checked_legendre(&u(7)), Some(1));
        assert_eq!(u(3).checked_legendre(&u(7)), Some(-1));
        assert_eq!(u(7).checked_legendre(&u(7)), Some(0));
        // quadratic residues mod 11: 1^2..5^2 = 1,4,9,5,3
        for a in [1u8, 3, 4, 5, 9] {
            assert_eq!(u(a).checked_legendre(&u(11)), Some(1), "a={a}");
        }
        for a in [2u8, 6, 7, 8, 10] {
            assert_eq!(u(a).checked_legendre(&u(11)), Some(-1), "a={a}");
        }
    }

    #[test]
    fn kronecker_symbol() {
        assert_eq!(u(1).kronecker(&u(0)), 1);
        assert_eq!(u(5).kronecker(&u(0)), 0);
        assert_eq!(u(5).kronecker(&u(1)), 1);
        assert_eq!(u(3).kronecker(&u(2)), -1);
        assert_eq!(u(7).kronecker(&u(2)), 1);
        assert_eq!(u(1).kronecker(&u(2)), 1);
        assert_eq!(u(5).kronecker(&u(2)), -1);
        assert_eq!(u(4).kronecker(&u(2)), 0); // even self
        assert_eq!(u(3).kronecker(&u(8)), -1); // (3|2)^3
        assert_eq!(u(5).kronecker(&u(8)), -1); // (5|2)^3
        assert_eq!(u(4).kronecker(&u(8)), 0);

        // consistency with the Jacobi symbol for odd n
        assert_eq!(u(6).kronecker(&u(35)), u(6).checked_jacobi(&u(35)).unwrap());
    }
}
