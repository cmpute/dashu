use core::cmp::Ordering;

use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use dashu_base::{Approximation, BitTest, ConversionError, DivRem, Gcd};
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

        let base = UBig::from_word(B);
        // (`ilog` is the truncated logarithm, i.e. the number of digits minus one)
        let num_digits = self.numerator.ilog(&base) + 1;
        let den_digits = self.denominator.ilog(&base) + 1;
        let sign = self.numerator.sign();
        // 0 unless B is a power of two
        let bits_per_digit = if B.is_power_of_two() {
            B.trailing_zeros() as usize
        } else {
            0
        };

        // Scale the value to (hi + lo / B^k) / denominator · B^exponent, where `hi` spans exactly
        // `precision + den_digits` digits. The truncated quotient q = hi / denominator then has
        // `precision` or `precision + 1` digits, so at most one digit is left to drop when
        // rounding. A narrow numerator is padded with zero digits (then k = 0 and lo = 0); the
        // excess low digits of a wide one are split off into `lo`, which is kept as sticky digits
        // so that no information is rounded away before the final (single) rounding step.
        let (hi, lo, k, mut exponent) = if num_digits > precision + den_digits {
            let k = num_digits - precision - den_digits;
            let (_, num_mag) = self.numerator.clone().into_parts();
            let (hi, lo) = if bits_per_digit > 0 {
                let (lo, hi) = num_mag.split_bits(k * bits_per_digit);
                (hi, lo)
            } else {
                num_mag.div_rem(&base.pow(k))
            };
            (hi, lo, k, k as isize)
        } else {
            let shift = precision + den_digits - num_digits;
            let hi = if bits_per_digit > 0 {
                &self.numerator << (shift * bits_per_digit)
            } else {
                &self.numerator * base.pow(shift)
            };
            (hi.into_parts().1, UBig::ZERO, 0, -(shift as isize))
        };
        let (q, rem) = hi.div_rem(&self.denominator);

        // The value is (q + N / D) · B^exponent with N = rem·B^k + lo and D = denominator·B^k
        // (0 <= N < D). If the quotient is one digit too wide, drop that digit together with the
        // fraction in a single step: with q = truncated·B + digit,
        //     q + N / D = truncated + (digit·D + N) / (B·D).
        // (Rounding the digit on its own first would double-round the value.)
        // (`ilog` also builds the power B^log, which a power-of-two base doesn't need to pay for:
        // its digit count follows from the bit length)
        let q_digits = if B.is_power_of_two() {
            (q.bit_len() + bits_per_digit - 1) / bits_per_digit
        } else {
            q.ilog(&base) + 1
        };
        debug_assert!(q_digits == precision || q_digits == precision + 1);
        let dropped = q_digits > precision;
        let (truncated, digit): (UBig, Word) = if dropped {
            let (truncated, digit) = if bits_per_digit > 0 {
                // the digit is the low bits of the quotient: mask it off and shift it out in place
                let mut truncated = q;
                let digit =
                    truncated.as_words()[0] & (Word::MAX >> (Word::BITS as usize - bits_per_digit));
                truncated >>= bits_per_digit;
                (truncated, UBig::from_word(digit))
            } else {
                q.div_rem(&base)
            };
            exponent += 1;
            (truncated, digit.try_into().expect("digit is below the base"))
        } else {
            (q, 0)
        };

        // The part below the truncated significand is digit·D + N over unit·D, where `unit` is
        // the weight of the last kept digit (B if a digit was dropped, 1 otherwise). Compared
        // with half of that unit:
        //     2·(digit·D + N)  vs  unit·D      <=>      2·N  vs  factor·D,   factor = unit − 2·digit
        // so the outcome is the sign of 2·N − factor·D. With N = rem·B^k + lo and
        // D = denominator·B^k:
        //     2·N − factor·D = B^k·(2·rem − factor·denominator) + 2·lo
        // Since rem < denominator, a positive even factor leaves a deficit of at least two units
        // in the bracket, beyond the reach of 2·lo < 2·B^k: the value is below the half. (In an
        // even base the factor is always even or zero.) Otherwise the bracket decides: exactly
        // zero is a tie, which a nonzero `lo` tips upwards, and one unit short can still be made
        // up by the doubled sticky digits.
        // The comparison lives in the closure so that the directed modes, which never ask for it,
        // don't pay for it.
        let half_test = || {
            if dropped {
                if digit == 0 {
                    // below one unit, hence below half of it
                    return Ordering::Less;
                }
                if digit > B - digit {
                    // past the half by itself, regardless of the remainder
                    return Ordering::Greater;
                }
            }
            let factor = if dropped { B - digit * 2 } else { 1 };
            if factor == 0 {
                // at the half: an exact tie unless a remainder or sticky digits are left
                return if rem.is_zero() && lo.is_zero() {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                };
            }
            if factor % 2 == 0 {
                return Ordering::Less;
            }
            let scaled = if factor == 1 {
                None
            } else {
                Some(factor * &self.denominator)
            };
            let scaled_den = scaled.as_ref().unwrap_or(&self.denominator);
            let twice_rem = &rem << 1;
            match twice_rem.cmp(scaled_den) {
                Ordering::Greater => Ordering::Greater,
                // an exact tie unless the sticky digits are nonzero
                Ordering::Equal => {
                    if lo.is_zero() {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                }
                Ordering::Less => {
                    if &twice_rem + UBig::ONE == *scaled_den {
                        // one unit short: only the sticky digits can turn it over
                        (&lo << 1).cmp(&base.pow(k))
                    } else {
                        Ordering::Less
                    }
                }
            }
        };

        let context = Context::<R>::new(precision);
        if rem.is_zero() && lo.is_zero() && digit == 0 {
            // nothing is left below the significand: the value is represented exactly
            let repr = FBigRepr::new(IBig::from_parts(sign, truncated), exponent);
            Approximation::Exact(FBig::from_repr(repr, context))
        } else {
            let integer = IBig::from_parts(sign, truncated);
            let adjust = R::round_low_part(&integer, sign, half_test);
            let repr = FBigRepr::new(integer + adjust, exponent);
            Approximation::Inexact(FBig::from_repr(repr, context), adjust)
        }
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
        if f.repr().is_infinite() {
            return None;
        } else if f.repr().is_pos_zero() {
            return Some(Self::ZERO);
        }

        // calculate lower and upper bound
        let (l, r, incl_l, incl_r) = R::error_bounds(f);
        let lb = f - l.with_precision(f.precision() + 1).unwrap();
        let rb = f + r.with_precision(f.precision() + 1).unwrap();

        // select the simplest in this range
        let left = Self::try_from(lb).unwrap();
        let right = Self::try_from(rb).unwrap();
        let mut simplest = Self::simplest_in(left.clone(), right.clone());
        if incl_l && left.is_simpler_than(&simplest) {
            simplest = left;
        }
        if incl_r && right.is_simpler_than(&simplest) {
            simplest = right;
        }
        Some(simplest)
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
    use dashu_base::Approximation::{Exact, Inexact};
    use dashu_float::{
        round::{
            mode::{Away, Down, HalfAway, HalfEven, Up, Zero},
            Rounding,
            Rounding::{AddOne, NoOp, SubOne},
        },
        DBig, FBig,
    };

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

    /// An unreduced rational, so that the digit counts the conversion sees are the given ones.
    fn repr(numerator: i64, denominator: u64) -> Repr {
        Repr {
            numerator: IBig::from(numerator),
            denominator: UBig::from(denominator),
        }
    }

    /// A float with the given value, flagged exact (`None`) or inexact with the adjustment
    /// that was applied to the truncated significand.
    fn rounded<R: Round, const B: Word>(
        value: i64,
        adjust: Option<Rounding>,
    ) -> Rounded<FBig<R, B>> {
        match adjust {
            None => Exact(FBig::from(value)),
            Some(adjust) => Inexact(FBig::from(value), adjust),
        }
    }

    // The reported regressions (6/11, 493801/4, 31/3, …) are public-API cases and live in
    // `tests/dashu_float.rs`; the tests below cover what only this module can steer.

    #[test]
    fn test_rational_to_float_rounding_decision() {
        // One input per outcome of comparing the dropped part with half a unit of the last kept
        // digit, all at precision 1 in base 10. The numerator's top `precision + denominator`
        // digits are divided by the denominator; a digit is dropped from the quotient when it
        // is 10 or more, and the digits below the divided part are the sticky digits.
        // (input, value under HalfEven, value under HalfAway)
        let cases = [
            // padded numerator, the quotient fits the precision
            ((12, 8), (2, Some(AddOne)), (2, Some(AddOne))), // 1.5: an exact tie, odd digit
            ((20, 8), (2, Some(NoOp)), (3, Some(AddOne))),   // 2.5: an exact tie, even digit
            ((11, 8), (1, Some(NoOp)), (1, Some(NoOp))),     // 1.375
            ((13, 8), (2, Some(AddOne)), (2, Some(AddOne))), // 1.625
            ((16, 8), (2, None), (2, None)),                 // 2: exact
            ((12, 5), (2, Some(NoOp)), (2, Some(NoOp))),     // 2.4: a fifth below the tie
            ((13, 5), (3, Some(AddOne)), (3, Some(AddOne))), // 2.6: a fifth above
            ((10, 5), (2, None), (2, None)),                 // 2: exact
            // padded numerator, one digit of the quotient is dropped
            ((19, 1), (20, Some(AddOne)), (20, Some(AddOne))), // 19: digit 9 is past the half on its own
            ((11, 1), (10, Some(NoOp)), (10, Some(NoOp))),     // 11: digit 1 is far below the half
            ((15, 1), (20, Some(AddOne)), (20, Some(AddOne))), // 15: an exact tie, odd
            ((25, 1), (20, Some(NoOp)), (30, Some(AddOne))),   // 25: an exact tie, even
            ((29, 2), (10, Some(NoOp)), (10, Some(NoOp))),     // 14.5: digit 4, remainder one half
            ((31, 2), (20, Some(AddOne)), (20, Some(AddOne))), // 15.5: digit 5, a remainder is left
            ((251, 10), (30, Some(AddOne)), (30, Some(AddOne))), // 25.1: digit 5, a remainder is left
            ((30, 2), (20, Some(AddOne)), (20, Some(AddOne))),   // 15: digit 5, an exact tie
            ((303, 10), (30, Some(NoOp)), (30, Some(NoOp))),     // 30.3: the dropped digit is 0
            ((20, 2), (10, None), (10, None)),                   // 10: exact
            ((99, 1), (100, Some(AddOne)), (100, Some(AddOne))), // 99: rounding up carries a digit
            // wide numerator: the digits cut off are the sticky part
            ((130, 2), (60, Some(NoOp)), (70, Some(AddOne))), // 65: an exact tie, even
            ((131, 2), (70, Some(AddOne)), (70, Some(AddOne))), // 65.5: sticky digits tip the tie
            ((125, 2), (60, Some(NoOp)), (60, Some(NoOp))),   // 62.5: no remainder, two below
            ((250, 1), (200, Some(NoOp)), (300, Some(AddOne))), // 250: digit 5, an exact tie
            ((257, 1), (300, Some(AddOne)), (300, Some(AddOne))), // 257: digit 5, sticky digits
            ((247, 1), (200, Some(NoOp)), (200, Some(NoOp))), // 247: digit 4, out of reach
            ((275, 1), (300, Some(AddOne)), (300, Some(AddOne))), // 275: digit 7, past the half
            ((225, 1), (200, Some(NoOp)), (200, Some(NoOp))), // 225: digit 2
            ((125, 5), (20, Some(NoOp)), (30, Some(AddOne))), // 25: a fifth below the tie, closed by the sticky digits
            ((124, 5), (20, Some(NoOp)), (20, Some(NoOp))), // 24.8: a fifth below, not quite closed
            ((126, 5), (30, Some(AddOne)), (30, Some(AddOne))), // 25.2: a fifth below, crossed
            ((119, 5), (20, Some(NoOp)), (20, Some(NoOp))), // 23.8: far below, large sticky digits
            ((2503, 10), (300, Some(AddOne)), (300, Some(AddOne))), // 250.3: digit 5, sticky digits
            ((3005, 10), (300, Some(NoOp)), (300, Some(NoOp))), // 300.5: the dropped digit is 0
        ];
        for ((num, den), even, away) in cases {
            let r = repr(num, den);
            assert_eq!(
                r.to_float::<HalfEven, 10>(1),
                rounded::<HalfEven, 10>(even.0, even.1),
                "{num}/{den} under HalfEven"
            );
            assert_eq!(
                r.to_float::<HalfAway, 10>(1),
                rounded::<HalfAway, 10>(away.0, away.1),
                "{num}/{den} under HalfAway"
            );
        }
    }

    #[test]
    fn test_rational_to_float_rounding_modes() {
        // 65.5 at 1 digit: the sticky digit settles the tie, so the result rounds away from
        // zero under every mode that looks at the size of the dropped part
        let r = RBig::from_parts(131.into(), 2u8.into());
        let n = RBig::from_parts((-131).into(), 2u8.into());
        assert_eq!(r.to_float::<HalfEven, 10>(1), rounded::<HalfEven, 10>(70, Some(AddOne)));
        assert_eq!(r.to_float::<HalfAway, 10>(1), rounded::<HalfAway, 10>(70, Some(AddOne)));
        assert_eq!(r.to_float::<Zero, 10>(1), rounded::<Zero, 10>(60, Some(NoOp)));
        assert_eq!(r.to_float::<Away, 10>(1), rounded::<Away, 10>(70, Some(AddOne)));
        assert_eq!(r.to_float::<Up, 10>(1), rounded::<Up, 10>(70, Some(AddOne)));
        assert_eq!(r.to_float::<Down, 10>(1), rounded::<Down, 10>(60, Some(NoOp)));

        // the same value negated: the directed modes follow the sign
        assert_eq!(n.to_float::<Zero, 10>(1), rounded::<Zero, 10>(-60, Some(NoOp)));
        assert_eq!(n.to_float::<Away, 10>(1), rounded::<Away, 10>(-70, Some(SubOne)));
        assert_eq!(n.to_float::<Up, 10>(1), rounded::<Up, 10>(-60, Some(NoOp)));
        assert_eq!(n.to_float::<Down, 10>(1), rounded::<Down, 10>(-70, Some(SubOne)));
    }

    #[test]
    fn test_rational_to_float_odd_base() {
        // In base 3 at precision 1, the numerator 11k₃ is split into 11₃ = 4 and the sticky
        // digit k. The quotient 4 drops the digit 1, which is one third below the half, and
        // leaves no remainder: the sticky digit alone decides.
        let nine = || FBig::<HalfEven, 3>::from(9u8);
        assert_eq!(repr(12, 1).to_float::<HalfEven, 3>(1), Inexact(nine(), NoOp)); // 110₃
        assert_eq!(repr(13, 1).to_float::<HalfEven, 3>(1), Inexact(nine(), NoOp)); // 111₃
        assert_eq!(
            repr(14, 1).to_float::<HalfEven, 3>(1), // 112₃
            Inexact(FBig::<HalfEven, 3>::from(18u8), AddOne)
        );
    }
}
