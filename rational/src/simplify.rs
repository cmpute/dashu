//! Implementations of methods related to simplification.
//!
//! Note that these methods are only implemented for [RBig] but not for [Relaxed][crate::Relaxed]
//! because the latter one is naturally not in the simplest form.

use crate::{error::panic_divide_by_0, rbig::RBig, repr::Repr};
use core::{cmp::Ordering, mem};
use dashu_base::{AbsOrd, Approximation, DivRem, UnsignedAbs};
use dashu_int::{IBig, Sign, UBig};

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
        let left = Self(
            Repr {
                numerator: &est.numerator + IBig::ONE,
                denominator: est.denominator.clone(),
            }
            .reduce(),
        );
        let right = Self(
            Repr {
                numerator: est.numerator - IBig::ONE,
                denominator: est.denominator,
            }
            .reduce(),
        );

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
    /// Determine if this rational number is simpler than the other number.
    ///
    /// This method only make sense for canonicalized ratios.
    #[inline]
    fn is_simpler_than(&self, other: &Self) -> bool {
        (self.denominator() < other.denominator()) // first compare denominator
            && self.numerator().abs_cmp(other.numerator()).is_le() // then compare numerator
            && self.sign() > other.sign() // then compare sign
    }

    /// Find the simplest rational number in the rounding interval of the [f32] number.
    ///
    /// This method returns None when the floating point value is not representable by a rational number,
    /// such as infinities or nans.
    ///
    /// See [RBig::simplest_in] for the definition of `simplicity`.
    ///
    /// The rounding interval of a [f32] value is an interval such that all numbers in this
    /// range will rounded to this [f32] value. For example the rounding interval for `1f32`
    /// is `[1. - f32::EPSILON / 2, 1. + f32::EPSILON / 2]`. That is, the error of result value will
    /// be less than 1/2 ULP.
    ///
    /// This method can be used to recover the original fraction represented as a division of [f32].
    /// See the examples below.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// assert_eq!(
    ///     RBig::simplest_from_f32(1e-1).unwrap(),
    ///     RBig::from_parts(1.into(), 10u8.into())
    /// );
    /// assert_eq!(
    ///     RBig::simplest_from_f32(22./7.).unwrap(),
    ///     RBig::from_parts(22.into(), 7u8.into())
    /// );
    /// ```
    pub fn simplest_from_f32(f: f32) -> Option<Self> {
        impl_simplest_from_float!(f)
    }

    /// Find the simplest rational number in the rounding interval of the [f64] number.
    ///
    /// This method returns None when the floating point value is not representable by a rational number,
    /// such as infinities or nans.
    ///
    /// See [RBig::simplest_in] for the definition of `simplicity`.
    ///
    /// The rounding interval of a [f64] value is an interval such that all numbers in this
    /// range will rounded to this [f64] value. For example the rounding interval for `1f64`
    /// is `[1. - f64::EPSILON / 2, 1. + f64::EPSILON / 2]`. That is, the error of result value will
    /// be less than 1/2 ULP.
    ///
    /// This method can be used to recover the original fraction represented as a division of [f64].
    /// See the examples below.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// assert_eq!(
    ///     RBig::simplest_from_f64(1e-1).unwrap(),
    ///     RBig::from_parts(1.into(), 10u8.into())
    /// );
    /// assert_eq!(
    ///     RBig::simplest_from_f64(22./7.).unwrap(),
    ///     RBig::from_parts(22.into(), 7u8.into())
    /// );
    pub fn simplest_from_f64(f: f64) -> Option<Self> {
        impl_simplest_from_float!(f)
    }

    /// Find the simplest rational number in the open interval `(lower, upper)`.
    ///
    /// A rational `n₁/d₁` is simpler than another rational number `n₂/d₂` if:
    /// * `d₁ < d₂` (compare denominator)
    /// * or `|n₁| < |n₂|` (then compare the magnitude of numerator)
    /// * or `n₂ < 0 < n₁` (then compare the sign)
    ///
    /// `lower` and `upper` will be swapped if necessary. If `lower` and `upper` are
    /// the same number, then this number will be directly returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a = RBig::from_parts(1234.into(), 5678u16.into());
    /// let b = RBig::from_parts(1235.into(), 5679u16.into());
    /// let s = RBig::simplest_in(a, b);
    /// // 1234/5678 < 5/23 < 1235/5679
    /// assert_eq!(s, RBig::from_parts(5.into(), 23u8.into()));
    /// ```
    ///
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

    /// Find the closest rational number to this number with a limit of the denominator.
    ///
    /// If the denominator of this number is larger than the limit, then it returns the closest one
    /// between `self.next_up()` and `self.next_down()` to `self`. If the denominator of this number
    /// is already less than or equal to the limit, then `Exact(self)` will be returned.
    ///
    /// The error `|self - self.nearest()|` will be less than `1/(2*limit)`, and the sign of
    /// the error `self - self.nearest()` will be returned if the result is not [Exact][Approximation::Exact].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::{Approximation::*, Sign};
    /// # use dashu_ratio::RBig;
    /// let a: RBig = 3.141592653.try_into().unwrap();
    /// assert_eq!(a.nearest(&10u8.into()), Inexact(
    ///     RBig::from_parts(22.into(), 7u8.into()),
    ///     Sign::Positive // 22/7 > 3.141592653
    /// ));
    /// ```
    pub fn nearest(&self, limit: &UBig) -> Approximation<Self, Sign> {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        // directly return this number if it's already simple enough
        if self.denominator() <= limit {
            return Approximation::Exact(self.clone());
        }

        let (trunc, r) = self.clone().split_at_point();
        let (left, right) = Self::farey_neighbors(&r, limit);

        // find the closest one (compare r - left and right - r)
        let mut mid = (&left + &right).0;
        mid.denominator <<= 1;
        if r.0 > mid {
            Approximation::Inexact(trunc + right, Sign::Positive)
        } else {
            Approximation::Inexact(trunc + left, Sign::Negative)
        }
    }

    /// Find the closest rational number that is greater than this number and has a denominator less than `limit`.
    ///
    /// It's equivalent to finding the next element in Farey sequence of order `limit`. The error
    /// `|self - self.next_up()|` will be less than `1/limit`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a: RBig = 3.141592653.try_into().unwrap();
    /// assert_eq!(a.next_up(&10u8.into()), RBig::from_parts(22.into(), 7u8.into()));
    /// ```
    pub fn next_up(&self, limit: &UBig) -> Self {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        let (trunc, fract) = self.clone().split_at_point();
        let up = if self.denominator() <= limit {
            // If the denominator of the number is already small enough, increase the number a little
            // bit before finding the farey neighbors. Note that the distance between two adjacent
            // numbers in a farey sequence is at least limit^-2, so we just increase limit^-2
            let target = fract
                + Self(Repr {
                    numerator: IBig::ONE,
                    denominator: limit.sqr(),
                });
            Self::farey_neighbors(&target, limit).1
        } else {
            // otherwise we can directly find the next value by finding farey bounds
            Self::farey_neighbors(&fract, limit).1
        };
        trunc + up
    }

    /// Find the closest rational number that is less than this number and has a denominator less than `limit`.
    ///
    /// It's equivalent to finding the previous element in Farey sequence of order `limit`. The error
    /// `|self - self.next_down()|` will be less than `1/limit`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a: RBig = 3.141592653.try_into().unwrap();
    /// assert_eq!(a.next_down(&10u8.into()), RBig::from_parts(25.into(), 8u8.into()));
    /// ```
    pub fn next_down(&self, limit: &UBig) -> Self {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        // similar to next_up()
        let (trunc, fract) = self.clone().split_at_point();
        let down = if self.denominator() <= limit {
            let target = fract
                - Self(Repr {
                    numerator: IBig::ONE,
                    denominator: limit.sqr(),
                });
            Self::farey_neighbors(&target, limit).0
        } else {
            Self::farey_neighbors(&fract, limit).0
        };
        trunc + down
    }
}
