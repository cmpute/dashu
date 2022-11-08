//! Implementations of methods related to simplification.
//! 
//! Note that these methods are only implemented for [RBig] but not for [Relaxed][crate::Relaxed]
//! because the latter one is naturally not in the simplest form.

use crate::{
    rbig::RBig,
    repr::Repr, error::panic_divide_by_0,
};
use core::{mem, cmp::Ordering};
use dashu_base::{DivRem, UnsignedAbs, Approximation, AbsCmp};
use dashu_int::{IBig, UBig, Sign};

impl Repr {
    /// Find the simplest rational number in the open interval `(lower, upper)`.
    /// See [RBig::simplest_in()] and <https://stackoverflow.com/q/66980340/5960776>.
    pub fn simplest_in(mut lower: Self, mut upper: Self) -> Self {
        let sign = if lower.numerator.sign() != upper.numerator.sign() {
            // if lower < 0 < upper, then 0 is the simplest
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

/// Implementation of simplest_from_f32, simplest_from_f64
macro_rules! impl_simplest_from_float {
    ($f:ident) => {{
        if $f.is_infinite() || $f.is_nan() {
            return None;
        } else if $f == 0. {
            return Some(Self::ZERO);
        }

        // get the range (f - ulp/2, f + ulp/2)
        // if f is negative, then range will be flipped by simplest_in()
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

        // find the simplest float in the range
        let mut simplest = Self::simplest_in(left.clone(), right.clone());
        if $f.to_bits() & 1 == 0 {
            // consider boundry values when last bit is 0 (because ties to even)
            if left.is_simpler_than(&simplest) {
                simplest = left;
            }
            if right.is_simpler_than(&simplest) {
                simplest = right;
            }
        }
        Some(simplest)
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

    pub fn simplest_from_f32(f: f32) -> Option<Self> {
        impl_simplest_from_float!(f)
    }

    pub fn simplest_from_f64(f: f64) -> Option<Self> {
        impl_simplest_from_float!(f)
    }

    /// Find the simplest rational number in the open interval `(lower, upper)`.
    /// 
    /// `lower` and `upper` will be swapped if necessary. If `lower` and `upper` are
    /// the same number, then this number will be directly returned.
    #[inline]
    pub fn simplest_in(lower: Self, upper: Self) -> Self {
        Self(Repr::simplest_in(lower.0, upper.0).reduce())
    }

    /// Find the previous and next value in farey sequence (from 0 to 1) with `limit` as the order.
    /// 
    /// This function requires `-1 < x < 1` and `x.denominator` > `limit`
    fn farey_neighbors(x: &Self, limit: &UBig) -> (Self, Self) {
        debug_assert!(x.denominator() > limit);
        debug_assert!(!x.numerator().is_zero());
        debug_assert!(x.numerator().abs_cmp(x.denominator()).is_le());

        let (mut left, mut right) = match x.sign() {
            Sign::Positive => (Repr::zero(), Repr::one()),
            Sign::Negative => (Repr::neg_one(), Repr::zero()),
        };

        // the Farey neighbors can be found directly by adding the
        // numerator and denominator together, see <https://en.wikipedia.org/wiki/Farey_sequence#Farey_neighbours>.
        loop {
            let mut next = Repr {
                numerator: &left.numerator + &right.numerator,
                denominator: &left.denominator + &right.denominator,
            };

            // test if the denominator has exceeded the limit
            if &next.denominator > limit {
                next = next.reduce();
                if &next.denominator > limit {
                    return (Self(left), Self(right));
                }
            }

            // tighten the bounds
            if next > x.0 {
                right = next;
            } else {
                left = next;
            }
        }
    }

    // Find the closest rational number to this number, such that the denominators of
    // the result numbers is less than or equal to the limit.
    pub fn nearest(&self, limit: &UBig) -> Approximation<Self, Sign> {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        // directly return this number if it's already simple enough
        if self.denominator() <= limit {
            return Approximation::Exact(self.clone())
        }

        let (trunc, r) = self.clone().split_at_point();
        let (left, right) = Self::farey_neighbors(&r, limit);

        // find the closest one (compare r - left and right - r)
        let mut mid = (&left + &right).0;
        mid.denominator <<= 1;
        if r.0 > mid {
            return Approximation::Inexact(trunc + right, Sign::Positive);
        } else {
            return Approximation::Inexact(trunc + left, Sign::Negative);
        }
    }

    /// Find the closest rational number that is greater than this number and has a denominator less than `limit`.
    /// 
    /// It's equivalent to finding the next element in Farey sequence of order `limit`. 
    pub fn next_up(&self, limit: &UBig) -> Self {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        let (trunc, fract) = self.clone().split_at_point();
        let up = if self.denominator() <= limit {
            // If the denominator of the number is already small enough, increase the number a little
            // bit before finding the farey neighbors. Note that the distance between two adjacent
            // numbers in a farey sequence is at least limit^-2, so we just increase limit^-2
            let target = fract + Self(Repr{
                numerator: IBig::ONE,
                denominator: limit.square(),
            });
            Self::farey_neighbors(&target, limit).1
        } else {
            // otherwise we can directly find the next value by finding farey bounds
            Self::farey_neighbors(&fract, limit).1
        };
        return trunc + up;
    }

    /// Find the closest rational number that is less than this number and has a denominator less than `limit`.
    /// 
    /// It's equivalent to finding the previous element in Farey sequence of order `limit`.
    /// 
    /// This method requires the denominator of this number is less than the limit, otherwise
    /// please use [RBig::nearest]
    pub fn next_down(&self, limit: &UBig) -> Self {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        // similar to next_up()
        let (trunc, fract) = self.clone().split_at_point();
        let down = if self.denominator() <= limit {
            let target = fract - Self(Repr{
                numerator: IBig::ONE,
                denominator: limit.square(),
            });
            Self::farey_neighbors(&target, limit).0
        } else {
            Self::farey_neighbors(&fract, limit).0
        };
        return trunc + down;
    }
}
