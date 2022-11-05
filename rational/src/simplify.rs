//! Implementations of methods related to simplification.
//! 
//! Note that these methods are only implemented for [RBig] but not for [Relaxed]
//! because the latter one is naturally not in the simpliest form.

use crate::{
    rbig::RBig,
    repr::Repr,
};
use core::{mem, cmp::Ordering};
use dashu_base::{DivRem, UnsignedAbs};
use dashu_int::IBig;

impl Repr {
    /// Find the simpliest rational number in the open interval `(lower, upper)`.
    /// See [RBig::simpliest_in()] and <https://stackoverflow.com/q/66980340/5960776>.
    pub fn simpliest_in(mut lower: Self, mut upper: Self) -> Self {
        let sign = if lower.numerator.sign() != upper.numerator.sign() {
            // if lower < 0 < upper, then 0 is the simpliest
            return Self::zero();
        } else {
            lower.numerator.sign()
        };
        lower = lower.abs();
        upper = upper.abs();

        match lower.cmp(&upper) {
            // swap so that lower is less than upper
            Ordering::Greater => mem::swap(&mut lower, &mut upper),
            Ordering::Equal => return sign * lower,
            Ordering::Less => {}
        }

        let Repr {
            numerator: mut num_l,
            denominator: den_l,
        } = lower;
        let Repr {
            numerator: mut num_r,
            denominator: den_r,
        } = upper;

        // negative values might exist during the calculation
        let (mut den_l, mut den_r) = (IBig::from(den_l), IBig::from(den_r));

        // use continued fraction expansion to find this float
        let (mut n0, mut d0) = (IBig::ONE, IBig::ZERO);
        let (mut n1, mut d1) = (IBig::ZERO, IBig::ONE);
        let (num, den) = loop {
            let (q, r1) = num_l.div_rem(&den_l);

            n1 += &q * &n0;
            mem::swap(&mut n0, &mut n1);
            d1 += &q * &d0;
            mem::swap(&mut d0, &mut d1);

            let r2 = mem::take(&mut num_r) - q * &den_r;
            num_l = mem::replace(&mut den_r, r1);
            num_r = mem::replace(&mut den_l, r2);

            if num_l < den_l {
                break (n0 + n1, d0 + d1);
            }
        };

        debug_assert!(num.sign() == den.sign());
        Repr {
            numerator: num.unsigned_abs() * sign,
            denominator: den.unsigned_abs(),
        }
    }
}

/// Implementation of simpliest_from_f32, simpliest_from_f64
macro_rules! impl_simpliest_from_float {
    ($f:ident) => {{
        if $f.is_infinite() || $f.is_nan() {
            return None;
        } else if $f == 0. {
            return Some(Self::ZERO);
        }

        // get the range (f - ulp/2, f + ulp/2)
        // if f is negative, then range will be flipped by simpliest_in()
        let mut est = Repr::try_from($f).unwrap();
        est.numerator <<= 1;
        est.denominator <<= 1;
        let left = Self(Repr {
            numerator: &est.numerator + IBig::ONE,
            denominator: est.denominator.clone()
        }.reduce());
        let right = Self(Repr {
            numerator: est.numerator - IBig::ONE,
            denominator: est.denominator
        }.reduce());

        // find the simpliest float in the range
        let mut simpliest = Self::simpliest_in(left.clone(), right.clone());
        if $f.to_bits() & 1 == 0 {
            // consider boundry values when last bit is 0 (because ties to even)
            if left.is_simpler_than(&simpliest) {
                simpliest = left;
            }
            if right.is_simpler_than(&simpliest) {
                simpliest = right;
            }
        }
        Some(simpliest)
    }};
}

impl RBig {
    /// Determine if this rational number is simpler than the other number
    ///
    /// This method only make sense for canonicalized ratios
    #[inline]
    fn is_simpler_than(&self, other: &Self) -> bool {
        (self.denominator() < other.denominator()) // first compare denominator
            && self.numerator().abs_cmp(other.numerator()).is_le() // then compare numerator
            && self.sign() > other.sign() // then compare sign
    }

    pub fn simpliest_from_f32(f: f32) -> Option<Self> {
        impl_simpliest_from_float!(f)
    }

    pub fn simpliest_from_f64(f: f64) -> Option<Self> {
        impl_simpliest_from_float!(f)
    }

    // TODO: support approx_fbig

    /// Find the simpliest rational number in the open interval `(lower, upper)`.
    /// 
    /// `lower` and `upper` will be swapped if necessary. If `lower` and `upper` are
    /// the same number, then this number will be directly returned.
    #[inline]
    pub fn simpliest_in(lower: Self, upper: Self) -> Self {
        Self(Repr::simpliest_in(lower.0, upper.0).reduce())
    }

    // Find the closest rational number to this number, such that the denominators of
    // the result numbers is less than the limit
    fn nearest(&self) -> (Self, Self) {
        unimplemented!()
    }
}
