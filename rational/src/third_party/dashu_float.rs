use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use dashu_base::{AbsOrd, Approximation, ConversionError, DivRem, Gcd};
use dashu_float::{
    round::{ErrorBounds, Round, Rounded},
    Context, FBig, Repr as FBigRepr,
};
use dashu_int::{IBig, UBig, Word};

impl<R: Round, const B: Word> TryFrom<Repr> for FBig<R, B> {
    type Error = ConversionError;

    /// Convert the rational number to a [`FBig`], succeeding only when the value is
    /// exactly representable in base `B` — that is, when every prime factor of the
    /// reduced denominator also divides `B`. For a correctly-rounded (possibly
    /// inexact) conversion, use [`RBig::to_float`] instead.
    fn try_from(v: Repr) -> Result<Self, Self::Error> {
        let Repr {
            numerator,
            denominator,
        } = v;

        // reduce to lowest terms, working on the magnitude
        let (sign, mut num_mag) = numerator.into_parts();
        let g = (&num_mag).gcd(&denominator);
        num_mag /= &g;
        let den = denominator / &g;

        // The value is exactly representable in base B iff every prime factor of the
        // reduced denominator also divides B. Repeatedly divide out gcd(den, B); the
        // iteration count `k` is the number of base-B digits to clear, and equals the
        // exponent (each step lowers every prime power p^a by v_p(B)).
        let base = UBig::from_word(B);
        let mut k = 0usize;
        let mut d = den.clone();
        loop {
            let g = (&d).gcd(&base);
            if g.is_one() {
                break;
            }
            d /= &g;
            k += 1;
        }
        if !d.is_one() {
            return Err(ConversionError::LossOfPrecision);
        }

        // Exact: value = num/den = (num * B^k / den) * B^(-k).
        let signif_mag = if k == 0 {
            num_mag
        } else {
            (num_mag * base.pow(k)) / &den
        };
        let signif = IBig::from_parts(sign, signif_mag);
        let repr = FBigRepr::new(signif, -(k as isize));
        let precision = repr.digits().max(1);
        Ok(FBig::from_repr(repr, Context::new(precision)))
    }
}

impl<const B: Word> TryFrom<FBigRepr<B>> for Repr {
    type Error = ConversionError;
    fn try_from(value: FBigRepr<B>) -> Result<Self, Self::Error> {
        if value.is_infinite() {
            Err(ConversionError::OutOfBounds)
        } else {
            let (signif, exp) = value.into_parts();
            let (numerator, denominator) = if exp >= 0 {
                (signif * UBig::from_word(B).pow(exp as usize), UBig::ONE)
            } else {
                (signif, UBig::from_word(B).pow((-exp) as usize))
            };
            Ok(Repr {
                numerator,
                denominator,
            })
        }
    }
}

impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for Repr {
    type Error = ConversionError;
    #[inline]
    fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
        value.into_repr().try_into()
    }
}

macro_rules! forward_conversion_to_repr {
    ($t:ident, $reduce:ident) => {
        impl<R: Round, const B: Word> TryFrom<$t> for FBig<R, B> {
            type Error = ConversionError;
            #[inline]
            fn try_from(v: $t) -> Result<Self, Self::Error> {
                v.0.try_into()
            }
        }

        impl<const B: Word> TryFrom<FBigRepr<B>> for $t {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: FBigRepr<B>) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| $t(repr.$reduce()))
            }
        }

        impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for $t {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| $t(repr.$reduce()))
            }
        }
    };
}
forward_conversion_to_repr!(RBig, reduce);
forward_conversion_to_repr!(Relaxed, reduce2);

impl Repr {
    // There are some cases where the result is exactly representable by a FBig
    // without loss of significance (it's an integer or its denominator is a power of B).
    // However, it's better to explicitly prohibit it because it's still failing
    // in other cases and a method that panics occasionally is not good.
    fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        assert!(precision > 0);

        if self.numerator.is_zero() {
            return FBig::ZERO.with_precision(precision);
        }

        // Scale the quotient by a power of the base so that its integer part has either
        // `precision` or `precision + 1` digits: with a = ilog(numerator) and
        // b = ilog(denominator), the value lies in the open range (B^(a-b-1), B^(a-b+1)).
        let base = UBig::from_word(B);
        let num_log = self.numerator.ilog(&base) as isize;
        let den_log = self.denominator.ilog(&base) as isize;
        let mut shift = precision as isize - (num_log - den_log);
        let (num, mut den) = if shift >= 0 {
            let num = if B == 2 {
                &self.numerator << shift as usize
            } else {
                &self.numerator * base.pow(shift as usize)
            };
            (num, self.denominator.clone())
        } else {
            (self.numerator.clone(), &self.denominator * base.pow(shift.unsigned_abs()))
        };
        let (mut q, mut r) = num.div_rem(&den);

        // The rounding must happen exactly once, at the last of the `precision` digits.
        // If the quotient has one digit more, move that digit into the remainder first
        // (rounding the longer quotient and then rounding again to the precision would
        // round twice, e.g. 2.49 -> 2.5 -> 3).
        if q.abs_cmp(&base.pow(precision)).is_ge() {
            let (q_high, q_low) = q.div_rem(&base);
            r += q_low * &den;
            den *= &base;
            q = q_high;
            shift -= 1;
        }
        debug_assert!(q.ilog(&base) + 1 == precision);

        let rounded = if r.is_zero() {
            Approximation::Exact(q)
        } else {
            let adjust = R::round_ratio(&q, r, den.as_ibig());
            Approximation::Inexact(q + adjust, adjust)
        };

        // the significand has at most `precision` digits now (or it's B^precision after a
        // carry), so the conversion below is exact
        let context = Context::<R>::new(precision);
        rounded
            .and_then(|n| context.convert_int(n))
            .map(|f| f >> shift)
    }
}

impl RBig {
    /// Convert the rational number to a [FBig] with guaranteed correct rounding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_ratio::RBig;
    /// use dashu_float::{DBig, round::Rounding::*};
    ///
    /// assert_eq!(RBig::ONE.to_float(1), Exact(DBig::ONE));
    /// assert_eq!(RBig::from(1000).to_float(4), Exact(DBig::from(1000)));
    /// assert_eq!(RBig::from_parts(1000.into(), 6u8.into()).to_float(4),
    ///     Inexact(DBig::from_parts(1667.into(), -1), AddOne));
    /// ```
    #[inline]
    pub fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        self.0.to_float(precision)
    }

    /// Find the simplest rational number in the rounding interval of the [FBig] number.
    ///
    /// The rounding interval is the set of the numbers that are rounded to `f` at the
    /// precision of `f` under its rounding mode, so that converting the result back with
    /// [RBig::to_float] gives `f` again. A number with unlimited precision is converted
    /// exactly. This method returns [None] when the floating point value is infinite.
    ///
    /// See [RBig::simplest_in] for the definition of `simplicity`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_ratio::RBig;
    /// use core::str::FromStr;
    /// use dashu_float::DBig;
    ///
    /// let f = DBig::from_str("4.00")? / DBig::from_str("3.00")?;
    /// let r = RBig::from_str_radix("4/3", 10)?;
    /// assert_eq!(RBig::simplest_from_float(&f), Some(r));
    /// assert_eq!(RBig::simplest_from_float(&DBig::INFINITY), None);
    ///
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn simplest_from_float<R: ErrorBounds, const B: Word>(f: &FBig<R, B>) -> Option<Self> {
        let repr = f.repr();
        if repr.is_infinite() {
            return None;
        } else if repr.significand().is_zero() {
            return Some(Self::ZERO);
        }

        let exact = Self::try_from(repr.clone()).unwrap();
        if f.precision() == 0 {
            // nothing else is rounded to a number with unlimited precision
            return Some(exact);
        }

        // The search is done on the magnitudes, the sign is restored at last.
        let sign = exact.sign();
        let magnitude = Self(exact.0.clone().abs());

        // The adjacent float away from zero is one unit in the last place apart. So is the
        // adjacent one towards zero, unless the magnitude is a power of the base: the
        // floats below it are B times denser.
        let digits = repr.digits();
        let precision = f.precision().max(digits); // a guard digit might be present
        let base = UBig::from_word(B);
        let ulp =
            Self::from(base.clone()).pow(repr.exponent() + digits as isize - precision as isize);
        let is_power = repr.significand().abs_cmp(&base.pow(digits - 1)).is_eq();
        let (outer_gap, inner_gap) = if is_power {
            (&ulp, &ulp / Self::from(base))
        } else {
            (&ulp, ulp.clone())
        };

        // The rounding interval is located by rounding probes, which makes the result
        // consistent with `to_float` under any rounding mode by construction. On each side
        // of f the interval can reach the adjacent float (which is excluded, the rounding
        // is directed), the midpoint to it (included or not depending on the tie-breaking
        // rule), or nowhere (the rounding is directed the other way and f is the endpoint).
        let rounds_to_f = |x: &Self| {
            let x = Self(sign * x.0.clone());
            let rounded = x.to_float::<R, B>(precision).value();
            Self::try_from(rounded.into_repr()).unwrap() == exact
        };
        let endpoint = |gap: &Self, outwards: bool| {
            let at = |quarters: u8| {
                let offset = gap * Self::from_parts(quarters.into(), 4u8.into());
                if outwards {
                    &magnitude + offset
                } else {
                    &magnitude - offset
                }
            };
            if rounds_to_f(&at(3)) {
                (at(4), false)
            } else if rounds_to_f(&at(1)) {
                let midpoint = at(2);
                let inclusive = rounds_to_f(&midpoint);
                (midpoint, inclusive)
            } else {
                (magnitude.clone(), true)
            }
        };
        let (inner, incl_inner) = endpoint(&inner_gap, false);
        let (outer, incl_outer) = endpoint(outer_gap, true);

        // select the simplest in this range
        let mut simplest = Self::simplest_in(inner.clone(), outer.clone());
        if incl_inner && inner.is_simpler_than(&simplest) {
            simplest = inner;
        }
        if incl_outer && outer.is_simpler_than(&simplest) {
            simplest = outer;
        }
        Some(Self(sign * simplest.0))
    }
}

impl Relaxed {
    /// Convert the rational number to a [FBig] with guaranteed correct rounding.
    ///
    /// See [RBig::to_float] for details.
    #[inline]
    pub fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        self.0.to_float(precision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RBig;
    use core::str::FromStr;
    use dashu_float::{DBig, FBig};

    #[test]
    fn test_rational_to_float_exact() {
        // values with a finite base-10 expansion convert exactly
        let cases: &[(RBig, &str)] = &[
            (RBig::from_parts(1.into(), 2u8.into()), "0.5"),
            (RBig::from_parts(1.into(), 4u8.into()), "0.25"),
            (RBig::from_parts(3.into(), 4u8.into()), "0.75"),
            (RBig::from_parts(1.into(), 8u8.into()), "0.125"),
            (RBig::from(100), "100"),
        ];
        for (r, expected) in cases {
            let f: DBig = r.clone().try_into().unwrap();
            assert_eq!(f, DBig::from_str(expected).unwrap(), "exact convert {r:?}");
        }

        // 1/8 also has a finite base-2 expansion (0.001b)
        let f: FBig = RBig::from_parts(1.into(), 8u8.into()).try_into().unwrap();
        assert_eq!(f, "0.001".parse::<FBig>().unwrap());
    }

    #[test]
    fn test_rational_to_float_inexact() {
        // 1/3 has no finite base-10 expansion
        let r = RBig::from_parts(1.into(), 3u8.into());
        let res: Result<DBig, _> = r.try_into();
        assert!(matches!(res, Err(ConversionError::LossOfPrecision)));

        // 7/12 = 7/(4*3): the factor 3 is absent from base 10
        let r = RBig::from_parts(7.into(), 12u8.into());
        let res: Result<DBig, _> = r.try_into();
        assert!(matches!(res, Err(ConversionError::LossOfPrecision)));

        // 1/10 has no finite base-2 expansion
        let r = RBig::from_parts(1.into(), 10u8.into());
        let res: Result<FBig, _> = r.try_into();
        assert!(matches!(res, Err(ConversionError::LossOfPrecision)));
    }
}
