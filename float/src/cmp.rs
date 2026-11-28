use core::cmp::Ordering;

use dashu_base::{AbsOrd, EstimatedLog2, Sign};
use dashu_int::{IBig, UBig};

// the kernels below live behind the num-order gate, and so do their imports
#[cfg(feature = "num-order")]
use crate::error::FpResult;
#[cfg(feature = "num-order")]
use crate::repr::Context;
#[cfg(feature = "num-order")]
use crate::round::mode::{Down, Up};
#[cfg(feature = "num-order")]
use crate::utils::{common_root, ilog_exact};
#[cfg(feature = "num-order")]
use dashu_base::{BitTest, UnsignedAbs};

use crate::{
    fbig::FBig,
    repr::{Repr, Word},
    round::Round,
    utils::{shl_digits, shl_digits_in_place},
};

impl<R1: Round, R2: Round, const B: Word> PartialEq<FBig<R2, B>> for FBig<R1, B> {
    #[inline]
    fn eq(&self, other: &FBig<R2, B>) -> bool {
        match (self.repr.is_infinite(), other.repr.is_infinite()) {
            // +inf == +inf, -inf == -inf
            (true, true) => !((self.repr.exponent >= 0) ^ (other.repr.exponent >= 0)),

            // the representation is normalized so direct comparing is okay,
            // and the context doesn't count in comparison
            (false, false) => self.repr == other.repr,

            // inf != any exact numbers
            (_, _) => false,
        }
    }
}
impl<R: Round, const B: Word> Eq for FBig<R, B> {}

/// Compare two [`Repr`]s, optionally at the precision of the [`FBig`] contexts they came from.
///
/// `precision = None` compares the **exact** values: neither side is rounded, whatever the
/// significands' widths. The `Some((lhs_prec, rhs_prec))` form is the `FBig` comparison's
/// magnitude shortcut, and it only applies when *both* precisions are nonzero — an
/// unlimited-precision (0) operand makes the comparison exact again, which is what comparing
/// against `FBig::ONE`/`ZERO` gets.
///
/// The distinction is load-bearing at the inverse transcendentals' domain checks and `±1`
/// endpoint rows: `±1` is a *pole* there (`asin`/`acos`/`acosh`/`atanh` have a vertical tangent,
/// `ln_1p` has one at `−1`), so an argument that merely *rounds* onto the endpoint is not the
/// endpoint value — its true result sits `√(2δ)` away, many ulps at a fixed work precision
/// (`asin(1 − 10⁻¹⁵⁵)` at 100 digits is `π/2 − 1.4·10⁻⁷⁸`) — and anything that *snaps* to the
/// endpoint certifies a result the true one is nowhere near. Those sites pass `None`. Away from
/// the poles (the `±1/2` rows, where the derivative is bounded) a rounded comparison is safe
/// instead: a sub-ulp argument perturbation moves the result by at most an ulp, and `1/2` is not
/// even representable in an odd base.
pub(crate) fn repr_cmp_same_base<const B: Word, const ABS: bool>(
    lhs: &Repr<B>,
    rhs: &Repr<B>,
    precision: Option<(usize, usize)>,
) -> Ordering {
    // case 1: compare with inf
    match (lhs.is_infinite(), rhs.is_infinite()) {
        (true, true) => {
            return if ABS {
                Ordering::Equal
            } else {
                lhs.exponent.cmp(&rhs.exponent)
            }
        }
        (false, true) => {
            return match ABS || rhs.exponent >= 0 {
                true => Ordering::Less,
                false => Ordering::Greater,
            }
        }
        (true, false) => {
            return match ABS || lhs.exponent >= 0 {
                true => Ordering::Greater,
                false => Ordering::Less,
            }
        }
        _ => {}
    };

    // case 2: compare sign
    let sign = if ABS {
        Sign::Positive
    } else {
        match (lhs.significand.sign(), rhs.significand.sign()) {
            (Sign::Positive, Sign::Positive) => Sign::Positive,
            (Sign::Positive, Sign::Negative) => return Ordering::Greater,
            (Sign::Negative, Sign::Positive) => return Ordering::Less,
            (Sign::Negative, Sign::Negative) => Sign::Negative,
        }
    };

    // case 3: compare with 0 (both +0 and -0 are zero)
    match (lhs.significand.is_zero(), rhs.significand.is_zero()) {
        (true, true) => return Ordering::Equal,
        (true, false) => {
            // rhs must be positive, otherwise case 2 will return
            return Ordering::Less;
        }
        (false, true) => {
            // lhs must be positive, otherwise case 2 will return
            return Ordering::Greater;
        }
        _ => {}
    }

    // case 4: compare exponent and precision
    let (lhs_exp, rhs_exp) = (lhs.exponent, rhs.exponent);
    if let Some((lhs_prec, rhs_prec)) = precision {
        // only compare when both number are not having arbitrary precision
        if lhs_prec != 0 && rhs_prec != 0 {
            // Saturating: an exponent near `isize::MAX` (e.g. the result of `powi(2, n)` for a
            // near-max `n`, which is representable so the range guard doesn't short-circuit it)
            // makes `exp + precision` overflow. These are magnitude shortcuts only — saturating
            // just forgoes the shortcut (falling through to the exact case 6), it never mis-orders.
            if lhs_exp > rhs_exp.saturating_add(rhs_prec as isize) {
                return sign * Ordering::Greater;
            }
            if rhs_exp > lhs_exp.saturating_add(lhs_prec as isize) {
                return sign * Ordering::Less;
            }
        }
    }

    // case 5: compare exponent and digits
    let (lhs_digits, rhs_digits) = (lhs.digits_ub(), rhs.digits_ub());
    if lhs_exp > rhs_exp.saturating_add(rhs_digits as isize) {
        return sign * Ordering::Greater;
    }
    if rhs_exp > lhs_exp.saturating_add(lhs_digits as isize) {
        return sign * Ordering::Less;
    }

    // case 6: compare exact values by shifting
    let (lhs_signif, rhs_signif) = (&lhs.significand, &rhs.significand);
    if ABS {
        match lhs_exp.cmp(&rhs_exp) {
            Ordering::Equal => lhs_signif.abs_cmp(rhs_signif),
            Ordering::Greater => {
                shl_digits::<B>(lhs_signif, (lhs_exp - rhs_exp) as usize).abs_cmp(rhs_signif)
            }
            Ordering::Less => {
                lhs_signif.abs_cmp(&shl_digits::<B>(rhs_signif, (rhs_exp - lhs_exp) as usize))
            }
        }
    } else {
        match lhs_exp.cmp(&rhs_exp) {
            Ordering::Equal => lhs_signif.cmp(rhs_signif),
            Ordering::Greater => {
                shl_digits::<B>(lhs_signif, (lhs_exp - rhs_exp) as usize).cmp(rhs_signif)
            }
            Ordering::Less => {
                lhs_signif.cmp(&shl_digits::<B>(rhs_signif, (rhs_exp - lhs_exp) as usize))
            }
        }
    }
}

impl<const B: Word> PartialOrd for Repr<B> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const B: Word> Ord for Repr<B> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        repr_cmp_same_base::<B, false>(self, other, None)
    }
}

impl<R1: Round, R2: Round, const B: Word> PartialOrd<FBig<R2, B>> for FBig<R1, B> {
    #[inline]
    fn partial_cmp(&self, other: &FBig<R2, B>) -> Option<Ordering> {
        Some(repr_cmp_same_base::<B, false>(
            &self.repr,
            &other.repr,
            Some((self.context.precision, other.context.precision)),
        ))
    }
}

impl<R: Round, const B: Word> Ord for FBig<R, B> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        repr_cmp_same_base::<B, false>(
            &self.repr,
            &other.repr,
            Some((self.context.precision, other.context.precision)),
        )
    }
}

impl<R: Round, const B: Word> AbsOrd for FBig<R, B> {
    #[inline]
    fn abs_cmp(&self, other: &Self) -> Ordering {
        repr_cmp_same_base::<B, true>(
            &self.repr,
            &other.repr,
            Some((self.context.precision, other.context.precision)),
        )
    }
}

pub(crate) fn repr_cmp_ubig<const B: Word, const ABS: bool>(lhs: &Repr<B>, rhs: &UBig) -> Ordering {
    // case 1: compare with inf
    if lhs.is_infinite() {
        return if lhs.exponent > 0 || ABS {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }

    // case 2: compare sign
    if !ABS && lhs.significand.sign() == Sign::Negative {
        return Ordering::Less;
    }

    // case 3: compare log2 estimations
    let (lhs_lo, lhs_hi) = lhs.log2_bounds();
    let (rhs_lo, rhs_hi) = rhs.log2_bounds();
    if lhs_lo > rhs_hi {
        return Ordering::Greater;
    }
    if lhs_hi < rhs_lo {
        return Ordering::Less;
    }

    // case 4: compare the exact values
    let mut rhs: IBig = rhs.clone().into();
    if lhs.exponent < 0 {
        shl_digits_in_place::<B>(&mut rhs, (-lhs.exponent) as usize);
        lhs.significand.cmp(&rhs)
    } else {
        shl_digits::<B>(&lhs.significand, lhs.exponent as usize).cmp(&rhs)
    }
}

pub(crate) fn repr_cmp_ibig<const B: Word, const ABS: bool>(lhs: &Repr<B>, rhs: &IBig) -> Ordering {
    // case 1: compare with inf
    if lhs.is_infinite() {
        return if lhs.exponent > 0 || ABS {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }

    // case 2: compare sign
    let sign = if ABS {
        Sign::Positive
    } else {
        match (lhs.significand.sign(), rhs.sign()) {
            (Sign::Positive, Sign::Positive) => Sign::Positive,
            (Sign::Positive, Sign::Negative) => return Ordering::Greater,
            (Sign::Negative, Sign::Positive) => return Ordering::Less,
            (Sign::Negative, Sign::Negative) => Sign::Negative,
        }
    };

    // case 3: compare log2 estimations
    let (lhs_lo, lhs_hi) = lhs.log2_bounds();
    let (rhs_lo, rhs_hi) = rhs.log2_bounds();
    if lhs_lo > rhs_hi {
        return sign * Ordering::Greater;
    }
    if lhs_hi < rhs_lo {
        return sign * Ordering::Less;
    }

    // case 4: compare the exact values
    if lhs.exponent < 0 {
        lhs.significand
            .cmp(&shl_digits::<B>(rhs, (-lhs.exponent) as usize))
    } else {
        shl_digits::<B>(&lhs.significand, lhs.exponent as usize).cmp(rhs)
    }
}

macro_rules! impl_abs_ord_with_method {
    ($T:ty, $method:ident) => {
        impl<const B: Word> AbsOrd<$T> for Repr<B> {
            #[inline]
            fn abs_cmp(&self, other: &$T) -> Ordering {
                $method::<B, true>(self, other)
            }
        }
        impl<const B: Word> AbsOrd<Repr<B>> for $T {
            #[inline]
            fn abs_cmp(&self, other: &Repr<B>) -> Ordering {
                $method::<B, true>(other, self).reverse()
            }
        }
        impl<R: Round, const B: Word> AbsOrd<$T> for FBig<R, B> {
            #[inline]
            fn abs_cmp(&self, other: &$T) -> Ordering {
                $method::<B, true>(&self.repr, other)
            }
        }
        impl<R: Round, const B: Word> AbsOrd<FBig<R, B>> for $T {
            #[inline]
            fn abs_cmp(&self, other: &FBig<R, B>) -> Ordering {
                $method::<B, true>(&other.repr, self).reverse()
            }
        }
    };
}
impl_abs_ord_with_method!(UBig, repr_cmp_ubig);
impl_abs_ord_with_method!(IBig, repr_cmp_ibig);

#[cfg(feature = "num-order")]
/// Up to this many bits of `B1^|e1|` and `B2^|e2|` together, two numbers in different bases are
/// compared by scaling both to integers, which is the fastest way while the powers are small.
const EXACT_POWER_BITS_THRESHOLD: u128 = 4096;

#[cfg(feature = "num-order")]
/// Compare the magnitudes of two finite nonzero numbers in different bases by scaling them to
/// integers. The cost is that of the powers `B1^|e1|` and `B2^|e2|`.
pub(crate) fn abs_cmp_scaled<const B1: Word, const B2: Word>(
    lhs: &Repr<B1>,
    rhs: &Repr<B2>,
) -> Ordering {
    let mut left: IBig = lhs.significand.clone().unsigned_abs().into();
    let mut right: IBig = rhs.significand.clone().unsigned_abs().into();
    if lhs.exponent < 0 {
        shl_digits_in_place::<B1>(&mut right, lhs.exponent.unsigned_abs());
    } else {
        shl_digits_in_place::<B1>(&mut left, lhs.exponent as usize);
    }
    if rhs.exponent < 0 {
        shl_digits_in_place::<B2>(&mut left, rhs.exponent.unsigned_abs());
    } else {
        shl_digits_in_place::<B2>(&mut right, rhs.exponent as usize);
    }
    left.cmp(&right)
}

#[cfg(feature = "num-order")]
/// Try to order the magnitudes of two finite nonzero numbers in different bases from
/// enclosures of their logarithms at a working precision of `precision` bits:
///
/// `ln|s1| + e1·ln(B1)` against `ln|s2| + e2·ln(B2)`
///
/// Every operation is rounded outwards (the logarithm itself is correctly rounded in the
/// directed modes), so the sign of the difference is certain when the enclosure excludes
/// zero. `None` means that it does not: the numbers are equal, or closer than the precision
/// tells apart.
pub(crate) fn abs_cmp_by_log<const B1: Word, const B2: Word>(
    lhs: &Repr<B1>,
    rhs: &Repr<B2>,
    precision: usize,
) -> Option<Ordering> {
    let (down, up) = (Context::<Down>::new(precision), Context::<Up>::new(precision));

    // Unwrap a directed float operation whose result cannot fail here: the operands are
    // finite, the arguments of the logarithms are positive, and the magnitudes are those of
    // an `isize` times a logarithm. (This precondition is local to this function;
    // `Context::unwrap_fp` is the shared unwrapper with the different contract of saturating
    // a possible error.)
    fn unwrap_directed<R: Round>(result: FpResult<FBig<R, 2>>) -> Repr<2> {
        match result {
            Ok(rounded) => rounded.value().into_repr(),
            Err(_) => unreachable!(),
        }
    }

    // the enclosure of ln|significand| + exponent · ln(base)
    let log = |significand: &IBig, exponent: isize, base: Word| -> (Repr<2>, Repr<2>) {
        let magnitude = Repr::<2>::new(significand.clone().unsigned_abs().into(), 0);
        let base = Repr::<2>::new(base.into(), 0);
        let exponent = Repr::<2>::new(exponent.into(), 0);
        let (base_lo, base_hi) =
            (unwrap_directed(down.ln(&base, None)), unwrap_directed(up.ln(&base, None)));
        // ln(base) > 0, so a negative exponent swaps the ends
        let (scaled_lo, scaled_hi) = if exponent.sign() == Sign::Positive {
            (
                unwrap_directed(down.mul(&exponent, &base_lo)),
                unwrap_directed(up.mul(&exponent, &base_hi)),
            )
        } else {
            (
                unwrap_directed(down.mul(&exponent, &base_hi)),
                unwrap_directed(up.mul(&exponent, &base_lo)),
            )
        };
        (
            unwrap_directed(down.add(&unwrap_directed(down.ln(&magnitude, None)), &scaled_lo)),
            unwrap_directed(up.add(&unwrap_directed(up.ln(&magnitude, None)), &scaled_hi)),
        )
    };
    let (lhs_lo, lhs_hi) = log(&lhs.significand, lhs.exponent, B1);
    let (rhs_lo, rhs_hi) = log(&rhs.significand, rhs.exponent, B2);

    let diff_lo = unwrap_directed(down.sub(&lhs_lo, &rhs_hi));
    if !diff_lo.significand.is_zero() && diff_lo.sign() == Sign::Positive {
        return Some(Ordering::Greater);
    }
    let diff_hi = unwrap_directed(up.sub(&lhs_hi, &rhs_lo));
    if !diff_hi.significand.is_zero() && diff_hi.sign() == Sign::Negative {
        return Some(Ordering::Less);
    }
    None
}

#[cfg(feature = "num-order")]
/// Compare the magnitudes of two finite nonzero numbers in different bases, in a time that
/// does not grow with their exponents (other than through the size of the exponents
/// themselves).
pub(crate) fn abs_cmp_cross_base<const B1: Word, const B2: Word>(
    lhs: &Repr<B1>,
    rhs: &Repr<B2>,
) -> Ordering {
    // With small exponents, scaling to integers is the fastest.
    let power_bits = |exponent: isize, base: Word| {
        exponent.unsigned_abs() as u128 * (Word::BITS - base.leading_zeros()) as u128
    };
    let power_bits = power_bits(lhs.exponent, B1) + power_bits(rhs.exponent, B2);
    if power_bits <= EXACT_POWER_BITS_THRESHOLD {
        return abs_cmp_scaled(lhs, rhs);
    }
    let (lhs_bits, rhs_bits) = (lhs.significand.bit_len(), rhs.significand.bit_len());

    // Bases that are powers of one integer `root` (2 and 8, 4 and 8): both numbers are an
    // integer times a power of the root, and only the difference of those powers matters.
    let root = common_root(B1, B2);
    if root != 0 {
        let (i, j) = (ilog_exact(B1, root) as i128, ilog_exact(B2, root) as i128);
        // |lhs| / |rhs| = |s1| / |s2| · root^diff
        let diff = i * lhs.exponent as i128 - j * rhs.exponent as i128;
        // root^|diff| >= 2^|diff|, and a significand of n bits lies in [2^(n-1), 2^n)
        if diff > rhs_bits as i128 {
            return Ordering::Greater;
        }
        if -diff > lhs_bits as i128 {
            return Ordering::Less;
        }
        let power = UBig::from_word(root).pow(diff.unsigned_abs() as usize);
        let (left, right) =
            (lhs.significand.clone().unsigned_abs(), rhs.significand.clone().unsigned_abs());
        return if diff >= 0 {
            (left * power).cmp(&right)
        } else {
            left.cmp(&(right * power))
        };
    }

    // Otherwise the bases are multiplicatively independent, and the logarithms are compared
    // at a growing precision. That settles every pair of unequal numbers eventually, but not
    // two equal ones, so it is cut off where the numbers can still be equal.
    //
    // If |s1|·B1^e1 = |s2|·B2^e2, then e1·v_p(B1) - e2·v_p(B2) = v_p(s2) - v_p(s1) for every
    // prime p, where the right side is at most m = max(bits(s1), bits(s2)) in magnitude. The
    // bases being independent, there are two primes p and q for which the two equations are
    // independent, and solving them bounds |e1| by m·(v_p(B2) + v_q(B2)) <= m·log2(B2), and
    // |e2| likewise. So with an exponent beyond m·(bits of a word) the numbers differ, and the
    // loop ends. Below that the powers are bounded in terms of the significands, and the
    // comparison falls back to scaling to integers once the working precision is a sizable
    // fraction of those integers (a logarithm costs many multiplications of its precision).
    let max_bits = lhs_bits.max(rhs_bits);
    let max_exponent = lhs.exponent.unsigned_abs().max(rhs.exponent.unsigned_abs());
    let can_be_equal = max_exponent as u128 <= max_bits as u128 * Word::BITS as u128;
    let precision_limit = (power_bits + max_bits as u128) / 64;

    let mut precision = 96;
    loop {
        if let Some(order) = abs_cmp_by_log(lhs, rhs, precision) {
            return order;
        }
        precision = precision.saturating_mul(2);
        if can_be_equal && precision as u128 > precision_limit {
            return abs_cmp_scaled(lhs, rhs);
        }
    }
}
