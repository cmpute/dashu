//! Implementations of methods related to simplification.
//!
//! Note that these methods are only implemented for [RBig] but not for [Relaxed][crate::Relaxed]
//! because the latter one is naturally not in the simplest form.

use crate::{error::panic_divide_by_0, rbig::RBig, repr::Repr};
use core::{cmp::Ordering, mem};
use dashu_base::{AbsOrd, Approximation, DivRem, FloatEncoding, UnsignedAbs};
use dashu_int::{IBig, Sign, UBig};

impl Repr {
    /// Find the simplest rational number in the open interval `(lower, upper)`.
    /// See [RBig::simplest_in()] and <https://stackoverflow.com/q/66980340/5960776>.
    pub fn simplest_in(mut lower: Self, mut upper: Self) -> Self {
        // Zero carries a positive sign, but as an endpoint it is not inside the open
        // interval: the interval then lies on the side of the other endpoint.
        let sign = match (lower.numerator.is_zero(), upper.numerator.is_zero()) {
            (true, true) => return Self::zero(),
            (true, false) => upper.numerator.sign(),
            (false, true) => lower.numerator.sign(),
            (false, false) => {
                if lower.numerator.sign() != upper.numerator.sign() {
                    // if lower < 0 < upper, then 0 is the simplest
                    return Self::zero();
                }
                lower.numerator.sign()
            }
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
    ($f:ident, $t:ty) => {{
        if $f.is_infinite() || $f.is_nan() {
            return None;
        } else if $f == 0. {
            return Some(Self::ZERO);
        }

        // |f| = man * 2^exp, the search is done on the magnitude and the sign restored at last
        let (man, exp) = $f.decode().unwrap();
        let sign = Sign::from(man < 0);
        let man = man.unsigned_abs();

        // The rounding interval ends at the midpoints between f and the adjacent floats.
        // The next float away from zero is at a distance of 2^exp. So is the next one
        // towards zero, unless |f| is a power of two (and not the least normal number,
        // below which the subnormal numbers keep the same spacing): then it is 2^(exp - 1)
        // away. With all the numbers scaled by 4 * 2^-exp to make them integers:
        let lowest_bit = 1 << (<$t>::MANTISSA_DIGITS - 1);
        let narrow = man == lowest_bit && exp as i32 > <$t>::MIN_EXP - <$t>::MANTISSA_DIGITS as i32;
        let scaled = UBig::from(man) << 2;
        let lower = &scaled - if narrow { UBig::ONE } else { UBig::from(2u8) };
        let upper = scaled + UBig::from(2u8);

        let exp = exp as isize - 2;
        let build = |n: UBig| {
            let repr = if exp >= 0 {
                Repr {
                    numerator: IBig::from(n) << exp as usize,
                    denominator: UBig::ONE,
                }
            } else {
                Repr {
                    numerator: IBig::from(n),
                    denominator: UBig::ONE << exp.unsigned_abs(),
                }
            };
            Self(repr.reduce2())
        };
        let (lower, upper) = (build(lower), build(upper));

        // find the simplest rational number in the range
        let mut simplest = Self::simplest_in(lower.clone(), upper.clone());
        if man & 1 == 0 {
            // The midpoints round to f only when its last bit is 0 (because ties to even).
            // Note that the adjacent float beyond a power of two has an odd mantissa.
            if lower.is_simpler_than(&simplest) {
                simplest = lower;
            }
            if upper.is_simpler_than(&simplest) {
                simplest = upper;
            }
        }
        Some(Self(sign * simplest.0))
    }};
}

impl RBig {
    /// Determine if this rational number is simpler than the other number.
    ///
    /// The numbers are ordered by their denominators first, then by the magnitudes of their
    /// numerators, and at last a positive number is simpler than its negation. See
    /// [RBig::simplest_in] for the definition of `simplicity`.
    ///
    /// This method only make sense for canonicalized ratios.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let half = RBig::from_parts(1.into(), 2u8.into());
    /// let three_halves = RBig::from_parts(3.into(), 2u8.into());
    /// assert!(RBig::ONE.is_simpler_than(&three_halves)); // smaller denominator
    /// assert!(half.is_simpler_than(&three_halves)); // smaller numerator
    /// assert!(RBig::ONE.is_simpler_than(&RBig::NEG_ONE)); // positive
    /// assert!(!half.is_simpler_than(&half));
    /// ```
    #[inline]
    pub fn is_simpler_than(&self, other: &Self) -> bool {
        // first compare denominator
        match self.denominator().cmp(other.denominator()) {
            Ordering::Less => return true,
            Ordering::Greater => return false,
            Ordering::Equal => {}
        }
        // then compare numerator
        match self.numerator().abs_cmp(other.numerator()) {
            Ordering::Less => return true,
            Ordering::Greater => return false,
            Ordering::Equal => {}
        }
        // then compare sign
        self.sign() == Sign::Positive && other.sign() == Sign::Negative
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
    /// is `[1. - f32::EPSILON / 4, 1. + f32::EPSILON / 2]` (the floats below one are twice as
    /// dense as those above it). That is, the error of result value will be at most 1/2 ULP.
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
        impl_simplest_from_float!(f, f32)
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
    /// is `[1. - f64::EPSILON / 4, 1. + f64::EPSILON / 2]` (the floats below one are twice as
    /// dense as those above it). That is, the error of result value will be at most 1/2 ULP.
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
        impl_simplest_from_float!(f, f64)
    }

    /// Find the simplest rational number in the open interval `(lower, upper)`.
    ///
    /// A rational `n₁/d₁` is simpler than another rational number `n₂/d₂` if:
    /// * `d₁ < d₂` (compare denominator)
    /// * or `d₁ = d₂` and `|n₁| < |n₂|` (then compare the magnitude of numerator)
    /// * or `d₁ = d₂`, `|n₁| = |n₂|` and `n₂ < 0 < n₁` (then compare the sign)
    ///
    /// `lower` and `upper` will be swapped if necessary. If `lower` and `upper` are
    /// the same number, then this number will be directly returned. The endpoints are
    /// otherwise not part of the interval, even when one of them is zero.
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

    /// Find the bounds of `x` in the Farey sequence of order `limit` extended to all the
    /// rational numbers (the fractions with a denominator of at most `limit`): the greatest
    /// element that is less than or equal to `x`, and the least element that is greater
    /// than `x`.
    fn farey_bounds(x: &Self, limit: &UBig) -> (Self, Self) {
        debug_assert!(!limit.is_zero());
        let (n, m) = (x.numerator(), x.denominator());

        // Start from the integers around x, then walk down the Stern-Brocot tree: the
        // mediant of the bounds replaces the left bound if it's not greater than x, and the
        // right bound otherwise, until its denominator exceeds the limit. With x = n/m and
        // the bounds a/b <= x < c/d, the walk is driven by
        //   p = n*b - a*m >= 0 (the distance to the left bound, times m*b) and
        //   q = c*m - n*d >  0 (the distance to the right bound, times m*d).
        // A run of k steps on the same side maps (p, q) to (p - k*q, q) or (p, q - k*p),
        // so each run is taken at once. This is the Euclidean algorithm on (p, q) and the
        // number of runs is logarithmic, whereas the number of single steps can be as
        // large as the limit (e.g. for x close to an integer).
        let floor = x.0.floor();
        let mut p = (n - &floor * m).unsigned_abs();
        let mut q = m - &p;
        let (mut a, mut b) = (floor.clone(), UBig::ONE);
        let (mut c, mut d) = (floor + IBig::ONE, UBig::ONE);

        loop {
            // move the left bound while the mediant is not greater than x
            let k_left = ((limit - &b) / &d).min(&p / &q);
            if !k_left.is_zero() {
                a += &c * &k_left;
                b += &d * &k_left;
                p -= &q * k_left.clone();
            }

            // move the right bound while the mediant is greater than x; once the left
            // bound is x itself (p = 0) only the denominator limits the run
            let mut k_right = (limit - &d) / &b;
            if !p.is_zero() {
                k_right = k_right.min((&q - UBig::ONE) / &p);
            }
            if !k_right.is_zero() {
                c += &a * &k_right;
                d += &b * &k_right;
                q -= &p * k_right.clone();
            }

            if k_left.is_zero() && k_right.is_zero() {
                break;
            }
        }

        // adjacent fractions in the tree are always in the lowest terms
        let left = Repr {
            numerator: a,
            denominator: b,
        };
        let right = Repr {
            numerator: c,
            denominator: d,
        };
        (Self(left), Self(right))
    }

    /// Find the closest rational number to this number with a limit of the denominator.
    ///
    /// If the denominator of this number is larger than the limit, then it returns the closest one
    /// between `self.next_up()` and `self.next_down()` to `self` (the lesser one if they are equally
    /// close). If the denominator of this number is already less than or equal to the limit, then
    /// `Exact(self)` will be returned.
    ///
    /// The error `|self - self.nearest()|` will be at most `1/(2*limit)` (this bound is reached when
    /// `self` is right in the middle of two fractions with the denominator `limit`). The sign
    /// of `self.nearest() - self` will be returned if the result is not [Exact][Approximation::Exact],
    /// that is [Sign::Positive] if the result is greater than `self`.
    ///
    /// # Panics
    ///
    /// Panics if `limit` is zero.
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

        // find the closest one (compare self - left and right - self)
        let (left, right) = Self::farey_bounds(self, limit);
        let mut mid = (&left + &right).0;
        mid.denominator <<= 1;
        if self.0 > mid {
            Approximation::Inexact(right, Sign::Positive)
        } else {
            Approximation::Inexact(left, Sign::Negative)
        }
    }

    /// Find the closest rational number that is greater than this number and has a denominator
    /// of at most `limit`.
    ///
    /// It's equivalent to finding the next element in Farey sequence of order `limit`. The error
    /// `|self - self.next_up()|` will be at most `1/limit` (this bound is reached when `self` is an
    /// integer).
    ///
    /// # Panics
    ///
    /// Panics if `limit` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a: RBig = 3.141592653.try_into().unwrap();
    /// assert_eq!(a.next_up(&10u8.into()), RBig::from_parts(22.into(), 7u8.into()));
    /// assert_eq!(RBig::ONE.next_up(&10u8.into()), RBig::from_parts(11.into(), 10u8.into()));
    /// ```
    pub fn next_up(&self, limit: &UBig) -> Self {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        Self::farey_bounds(self, limit).1
    }

    /// Find the closest rational number that is less than this number and has a denominator
    /// of at most `limit`.
    ///
    /// It's equivalent to finding the previous element in Farey sequence of order `limit`. The error
    /// `|self - self.next_down()|` will be at most `1/limit` (this bound is reached when `self` is an
    /// integer).
    ///
    /// # Panics
    ///
    /// Panics if `limit` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a: RBig = 3.141592653.try_into().unwrap();
    /// assert_eq!(a.next_down(&10u8.into()), RBig::from_parts(25.into(), 8u8.into()));
    /// assert_eq!(RBig::ONE.next_down(&10u8.into()), RBig::from_parts(9.into(), 10u8.into()));
    /// ```
    pub fn next_down(&self, limit: &UBig) -> Self {
        if limit.is_zero() {
            panic_divide_by_0()
        }

        // the mirror image of next_up()
        -Self::farey_bounds(&-self, limit).1
    }
}
