//! Exact sums of products over an unbounded exponent range.
//!
//! The parts of a complex product, quotient or squared modulus are sums of products of the
//! operand parts. A single product can leave the `isize` exponent range while the sum (or the
//! quotient of two sums) is an ordinary number, and the addends can lie astronomically far
//! apart. The helpers here carry the exponent in an `i128` and never align two addends across
//! a gap that the result does not depend on, so they return
//!
//! - the exact sign of a sum ([`sign_of_sum`]),
//! - a sum correctly rounded to a precision ([`sum_of_products`]), and
//! - the correctly rounded quotient of two sums ([`ratio_of_sums`])
//!
//! at a cost bounded by the widths of the operands and the precision, whatever the exponents.

use crate::mul::Term;
use alloc::vec::Vec;
use dashu_base::Approximation::{self, Exact, Inexact};
use dashu_base::Sign::{self, *};
use dashu_float::round::{mode, Round, Rounding};
use dashu_float::{Context as FloatCtxt, FBig, FpError, FpResult, Repr};
use dashu_int::{IBig, UBig, Word};

/// Guard digits of the approximate quotient in [`ratio_of_sums`] (see there for what the
/// derivation needs).
const RATIO_GUARD: usize = 8;

/// A finite nonzero number `repr · B^offset`. The exponent of `repr` is small (it is that of a
/// normalized significand, or a distance bounded by the operand widths), the offset is not.
#[derive(Clone)]
pub(crate) struct Wide<const B: Word> {
    repr: Repr<B>,
    offset: i128,
}

impl<const B: Word> Wide<B> {
    /// `significand · B^exponent`, or `None` if it is zero.
    fn new(significand: IBig, exponent: i128) -> Option<Self> {
        if significand.is_zero() {
            None
        } else {
            Some(Self {
                repr: Repr::new(significand, 0),
                offset: exponent,
            })
        }
    }

    /// The product `sign · a · b` of two finite numbers, or `None` if it is zero.
    pub(crate) fn product(a: &Repr<B>, b: &Repr<B>, sign: Sign) -> Option<Self> {
        let significand = a.significand() * b.significand();
        Self::new(significand * sign, a.exponent() as i128 + b.exponent() as i128)
    }

    /// `self · factor · B^shift` for a nonzero `factor`.
    fn scale(&self, factor: &IBig, shift: i128) -> Self {
        Self {
            repr: Repr::new(self.repr.significand() * factor, self.repr.exponent()),
            offset: self.offset + shift,
        }
    }

    fn sign(&self) -> Sign {
        self.repr.sign()
    }

    /// The exponent of the lowest digit position.
    fn exponent(&self) -> i128 {
        self.offset + self.repr.exponent() as i128
    }

    /// An upper bound on the position just above the top digit: `|self| < B^top_ub`.
    fn top_ub(&self) -> i128 {
        self.exponent() + self.repr.digits_ub() as i128
    }

    /// A lower bound on the magnitude: `|self| >= B^top_lb`.
    fn top_lb(&self) -> i128 {
        self.exponent() + self.repr.digits_lb() as i128
    }

    /// The exact sum, or `None` if it is zero.
    ///
    /// The two numbers are aligned, so this is only called when the gap between their lowest
    /// digit positions is bounded by their widths (see the callers).
    fn add(&self, other: &Self) -> Option<Self> {
        let base = self.exponent().min(other.exponent());
        let lift = |w: &Self| {
            let exponent = isize::try_from(w.exponent() - base)
                .expect("the exponent gap is bounded by the operand widths");
            Repr::<B>::new(w.repr.significand().clone(), exponent)
        };
        let exact = FloatCtxt::<mode::Zero>::new(0);
        let sum = exact.unwrap_fp(exact.add(&lift(self), &lift(other)));
        if sum.repr().significand().is_zero() {
            None
        } else {
            Some(Self {
                repr: sum.into_repr(),
                offset: base,
            })
        }
    }
}

/// The nonzero products of `terms` (which must be finite).
pub(crate) fn products<const B: Word>(terms: &[Term<'_, B>]) -> Vec<Wide<B>> {
    terms
        .iter()
        .filter_map(|&(a, b, sign)| Wide::product(a, b, sign))
        .collect()
}

/// `ceil(log2(n))` for `n >= 1`.
fn ceil_log2(n: usize) -> i128 {
    (usize::BITS - (n - 1).leading_zeros()) as i128
}

fn by_descending_top<const B: Word>(terms: &mut [Wide<B>]) {
    terms.sort_by_key(|t| core::cmp::Reverse(t.top_ub()));
}

/// The sign of the exact sum of `terms`, or `None` if the sum is zero.
///
/// The addends are accumulated exactly from the highest one down, and the accumulation stops
/// as soon as the partial sum exceeds everything that is left. Until then the partial sum is
/// not far above the next addend, so each alignment is bounded by the widths of the addends.
pub(crate) fn sign_of_sum<const B: Word>(mut terms: Vec<Wide<B>>) -> Option<Sign> {
    by_descending_top(&mut terms);
    let mut acc: Option<Wide<B>> = None;
    for (i, t) in terms.iter().enumerate() {
        if let Some(partial) = &acc {
            // each remaining addend is below `B^top_ub(t)`, so their total is below
            // `2^c · B^top_ub(t) <= B^(top_ub(t) + c)`
            let c = ceil_log2(terms.len() - i);
            if partial.top_lb() >= t.top_ub() + c {
                return Some(partial.sign());
            }
        }
        acc = match acc {
            None => Some(t.clone()),
            Some(partial) => partial.add(t),
        };
    }
    acc.map(|sum| sum.sign())
}

/// The exact sum of `terms` in a form that rounds like it to `precision` (nonzero) digits or
/// fewer, under any mode: `None` if the sum is zero.
///
/// The addends are accumulated exactly from the highest one down. Once the remaining ones lie
/// entirely below the rounding window of the partial sum, only the sign of their total can
/// still affect the rounding, so they are replaced by a unit just below that window.
fn sticky_sum<const B: Word>(mut terms: Vec<Wide<B>>, precision: usize) -> Option<Wide<B>> {
    by_descending_top(&mut terms);
    let mut acc: Option<Wide<B>> = None;
    for (i, t) in terms.iter().enumerate() {
        let partial = match &acc {
            None => {
                acc = Some(t.clone());
                continue;
            }
            Some(partial) => partial,
        };
        // A position `k` such that replacing a nonzero perturbation of magnitude below
        // `2·B^k` by `±B^k` (same sign) cannot change how `partial + perturbation` rounds:
        // `partial` and every rounding boundary near it are multiples of `g/2` for
        // `g = B^min(exponent, top − 1 − precision)`, and `k` leaves three digits of margin
        // below `g` so that `2·B^k < g/2` in every base.
        let window = partial.top_lb() - precision as i128 - 1;
        let k = partial.exponent().min(window) - 3;
        // the remaining addends total less than `2^c · B^top_ub(t)`
        let c = ceil_log2(terms.len() - i);
        if t.top_ub() + (c - 1).max(0) <= k {
            if let Some(sign) = sign_of_sum(terms[i..].to_vec()) {
                let unit = Wide {
                    repr: Repr::new(IBig::ONE * sign, 0),
                    offset: k,
                };
                acc = partial.add(&unit);
            }
            break;
        }
        acc = partial.add(t);
    }
    acc
}

/// Attach the exponent `offset` to a finite nonzero value, reporting a result outside the
/// exponent range as [`FpError::Overflow`] / [`FpError::Underflow`].
fn with_offset<R: Round, const B: Word>(
    value: Repr<B>,
    offset: i128,
    ctx: &FloatCtxt<R>,
) -> Result<FBig<R, B>, FpError> {
    let sign = value.sign();
    let (significand, exponent) = value.into_parts();
    // fold the trailing zeros in first, so that a value is out of range only when it really is
    let (significand, folded) = Repr::<B>::new(significand, 0).into_parts();
    let exponent = offset + exponent as i128 + folded as i128;
    if exponent >= isize::MAX as i128 {
        Err(FpError::Overflow(sign))
    } else if exponent <= isize::MIN as i128 {
        Err(FpError::Underflow(sign))
    } else {
        Ok(FBig::from_repr(Repr::new(significand, exponent as isize), *ctx))
    }
}

/// The sign of `Σ signᵢ·aᵢ·bᵢ` for finite operands, with a lower bound `e` on its magnitude
/// (`|Σ| >= B^e`); `None` if the sum is zero.
pub(crate) fn sign_and_magnitude<const B: Word>(terms: &[Term<'_, B>]) -> Option<(Sign, i128)> {
    // The sticky form at one digit has the sign of the sum, and differs from it by less than
    // three units four digits below its lowest one — far less than a factor `B`.
    let sum = sticky_sum(products(terms), 1)?;
    Some((sum.sign(), sum.top_lb() - 1))
}

/// Round `v + δ` to the (limited) precision of `ctx`, for a perturbation `δ` of the sign
/// `sticky` that is too small to be visible next to the finite nonzero `v`: with
/// `pad = max(0, precision + 1 − digits(v))`, it must satisfy `|δ| <= B^(v.exponent − pad − 2)`.
///
/// Every rounding boundary near `v` (a representable value or a midpoint between two) is a
/// multiple of half a unit in the last place of the `precision + 1`-digit grid `v` is padded
/// to, so none lies strictly between `v` and `v + δ`, and `v + δ` rounds like `v` with one
/// extra unit two digits further down. This is how a function whose value is a known closed
/// form plus a negligible tail is rounded directly — a Ziv loop would need a working
/// precision proportional to `log(1/|δ|)` to see the tail. The result is always inexact.
pub(crate) fn round_sticky<R: Round, const B: Word>(
    ctx: &FloatCtxt<R>,
    v: &Repr<B>,
    sticky: Sign,
) -> FpResult<FBig<R, B>> {
    let p = ctx.precision();
    debug_assert!(p != 0 && v.is_finite() && !v.significand().is_zero());
    let pad = (p + 1).saturating_sub(v.digits()) + 2;
    let padded = v.significand() * IBig::from(UBig::from(B).pow(pad)) + IBig::ONE * sticky;
    // rounded in a frame whose exponent is zero: the padded exponent can lie below the range
    let rounded =
        FBig::<R, B>::from_repr(Repr::new(padded, 0), FloatCtxt::new(0)).with_precision(p);
    let offset = v.exponent() as i128 - pad as i128;
    Ok(match rounded {
        Exact(r) => Exact(with_offset(r.into_repr(), offset, ctx)?),
        Inexact(r, rnd) => Inexact(with_offset(r.into_repr(), offset, ctx)?, rnd),
    })
}

/// The correctly rounded value of `Σ signᵢ·aᵢ·bᵢ` under `ctx` (of limited precision) for finite
/// operands, where a product may leave the exponent range: only a *result* outside the range
/// is reported as [`FpError::Overflow`] / [`FpError::Underflow`].
pub(crate) fn sum_of_products<R: Round, const B: Word>(
    ctx: &FloatCtxt<R>,
    terms: &[Term<'_, B>],
) -> FpResult<FBig<R, B>> {
    let p = ctx.precision();
    debug_assert!(p != 0);
    match sticky_sum(products(terms), p) {
        // exact cancellation of nonzero addends
        None => {
            let zero = Repr::zero_with_sign(if R::IS_ROUND_TOWARD_NEGATIVE {
                Negative
            } else {
                Positive
            });
            Ok(Exact(FBig::from_repr(zero, *ctx)))
        }
        Some(sum) => {
            let rounded = FBig::<R, B>::from_repr(sum.repr, FloatCtxt::new(0)).with_precision(p);
            Ok(match rounded {
                Exact(v) => Exact(with_offset(v.into_repr(), sum.offset, ctx)?),
                Inexact(v, rnd) => Inexact(with_offset(v.into_repr(), sum.offset, ctx)?, rnd),
            })
        }
    }
}

/// The correctly rounded quotient `(Σ num) / (Σ den)` under `ctx` (of limited precision), for
/// finite operands and a positive denominator. The value, its inexactness flag and (for the
/// directed modes) its direction are those of the exact quotient, whatever the exponent
/// spread of the addends; a quotient outside the exponent range is reported as
/// [`FpError::Overflow`] / [`FpError::Underflow`].
///
/// With `N` and `D` the two sums, an approximation of `|N/D|` to a few guard digits pins the
/// truncated `precision`-digit significand `m` down to two neighbours, and exact sign tests
/// (`|N|` against `m·u·D` and against `(m + ½)·u·D`, for the unit `u` of the last digit)
/// decide between them and place the remainder relative to one half — everything the
/// rounding mode needs.
pub(crate) fn ratio_of_sums<R: Round, const B: Word>(
    ctx: &FloatCtxt<R>,
    num: &[Term<'_, B>],
    den: &[Term<'_, B>],
) -> FpResult<FBig<R, B>> {
    let p = ctx.precision();
    debug_assert!(p != 0);
    let num_terms = products(num);
    let den_terms = products(den);
    let sign = match sign_of_sum(num_terms.clone()) {
        // an exactly zero numerator: the zero carries the sign of the sum of products
        None => return crate::mul::sum_of_products(ctx, num),
        Some(sign) => sign,
    };

    // `q̂`, an approximation of `|N/D|` at `w` digits: each of the two sums and their quotient
    // is truncated at `w` digits, so with `ε = B^(1−w)` the quotient `|N/D|` lies in
    // `]q̂·(1 − 2ε), q̂·(1 + 3ε)[`.
    let w = p + RATIO_GUARD;
    let trunc = FloatCtxt::<mode::Zero>::new(w);
    let exact = FloatCtxt::<mode::Zero>::new(0);
    let round = |sum: Wide<B>| {
        let value = FBig::<mode::Zero, B>::from_repr(sum.repr, exact).with_precision(w);
        (value.value(), sum.offset)
    };
    let (n_hat, n_offset) = round(sticky_sum(num_terms.clone(), w).expect("nonzero numerator"));
    let (d_hat, d_offset) = round(sticky_sum(den_terms.clone(), w).expect("nonzero denominator"));
    let q_hat = trunc
        .unwrap_fp(trunc.div(n_hat.repr(), d_hat.repr()))
        .into_repr();
    let q_hat = if q_hat.sign() == Negative {
        -q_hat
    } else {
        q_hat
    };
    let offset = n_offset - d_offset;

    // the enclosure `[lo, hi]`, with `δ = B^(top − w + 1) > ε·q̂` for the position `top` just
    // above the top digit of `q̂`
    let top = q_hat.exponent() + q_hat.digits() as isize;
    let delta = |times: u8| Repr::<B>::new(IBig::from(times), top - w as isize + 1);
    let lo = exact.unwrap_fp(exact.sub(&q_hat, &delta(2)));
    let hi = exact.unwrap_fp(exact.add(&q_hat, &delta(3)));

    // The enclosure is narrower than one unit of the last of `precision` digits, so its two
    // ends truncate to the same `precision`-digit number or to two consecutive ones; in the
    // latter case one exact comparison with the upper one decides.
    let g_lo = lo.with_precision(p).value();
    let g_hi = hi.with_precision(p).value();
    let against = |m: &IBig, half: bool, unit: i128| {
        // the sign of `|N| − m·B^unit·D`, or of `2|N| − (2m + 1)·B^unit·D` with `half`
        let (n_factor, d_factor) = if half {
            (IBig::from(2u8), -((m << 1) + IBig::ONE))
        } else {
            (IBig::ONE, -m)
        };
        let n_factor = n_factor * sign;
        let mut terms: Vec<Wide<B>> = num_terms.iter().map(|t| t.scale(&n_factor, 0)).collect();
        terms.extend(den_terms.iter().map(|t| t.scale(&d_factor, unit)));
        sign_of_sum(terms)
    };
    // a `precision`-digit number as its full significand and the exponent of its last digit
    let split = |g: &FBig<mode::Zero, B>| {
        let pad = p - g.repr().digits();
        let m = g.repr().significand() * IBig::from(UBig::from(B).pow(pad));
        (m, g.repr().exponent() as i128 - pad as i128 + offset)
    };
    let (m_hi, unit_hi) = split(&g_hi);
    let mut floor_test = None;
    let (m, unit) = if g_lo == g_hi {
        (m_hi, unit_hi)
    } else {
        match against(&m_hi, false, unit_hi) {
            Some(Negative) => split(&g_lo),
            decided => {
                floor_test = Some(decided);
                (m_hi, unit_hi)
            }
        }
    };

    // `|N/D| = (m + f)·B^unit` with `0 <= f < 1`: place `f` relative to 0 and ½
    let floor_test = floor_test.unwrap_or_else(|| against(&m, false, unit));
    let signed_m = m.clone() * sign;
    let rounding = if floor_test.is_none() {
        None // `f = 0`: the quotient is `m·B^unit` exactly
    } else {
        // a stand-in for the remainder with the same sign and the same position relative to ½
        let quarters: i8 = match against(&m, true, unit) {
            Some(Negative) => 1,
            None => 2,
            Some(Positive) => 3,
        };
        Some(R::round_ratio(&signed_m, IBig::from(quarters) * sign, &IBig::from(4u8)))
    };
    let significand = match rounding {
        None | Some(Rounding::NoOp) => signed_m,
        Some(Rounding::AddOne) => signed_m + IBig::ONE,
        Some(Rounding::SubOne) => signed_m - IBig::ONE,
    };
    let value = with_offset(Repr::new(significand, 0), unit, ctx)?;
    Ok(match rounding {
        None => Approximation::Exact(value),
        Some(rounding) => Approximation::Inexact(value, rounding),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    type F = FBig<mode::HalfEven, 2>;

    fn r(sig: i64, exp: isize) -> Repr<2> {
        Repr::new(sig.into(), exp)
    }

    const FAR: isize = 1 << 62;

    #[test]
    fn sign_across_the_exponent_range() {
        let (one, big, tiny) = (r(1, 0), r(1, FAR + 10), r(1, -FAR - 10));
        // big² − big² + tiny² − 1: the leading products (beyond the range) cancel exactly
        let terms = [
            (&big, &big, Positive),
            (&tiny, &tiny, Positive),
            (&big, &big, Negative),
            (&one, &one, Negative),
        ];
        assert_eq!(sign_of_sum(products(&terms)), Some(Negative));
        // … and with nothing but the product below the range left
        assert_eq!(sign_of_sum(products(&terms[..3])), Some(Positive));
        assert_eq!(sign_of_sum(products(&[terms[0], terms[2]])), None);
        assert_eq!(sign_of_sum(products(&[(&r(0, 0), &big, Positive)])), None);
        // three addends of one sign against a fourth just above each of them
        let (three, eight) = (r(3, 0), r(8, 0));
        let terms = [
            (&three, &one, Positive),
            (&three, &one, Positive),
            (&three, &one, Positive),
            (&eight, &one, Negative),
        ];
        assert_eq!(sign_of_sum(products(&terms)), Some(Positive));
    }

    #[test]
    fn wide_sum_matches_the_in_range_sum() {
        // the same sums through the in-range path of `mul::sum_of_products`
        let values = [
            r(1, 0),
            r(-3, 5),
            r(5, -70),
            r(7, 200),
            r(-1, -200),
            r(0, 0),
        ];
        for p in [1usize, 2, 20, 50, 100, 500] {
            let ctx = FloatCtxt::<mode::HalfEven>::new(p);
            let down = FloatCtxt::<mode::Down>::new(p);
            for a in &values {
                for b in &values {
                    for c in &values {
                        let terms = [(a, b, Positive), (b, c, Negative), (c, c, Positive)];
                        if terms
                            .iter()
                            .all(|(x, y, _)| x.significand().is_zero() || y.significand().is_zero())
                        {
                            continue; // the signed-zero table of the in-range path
                        }
                        assert_eq!(
                            sum_of_products(&ctx, &terms),
                            crate::mul::sum_of_products(&ctx, &terms)
                        );
                        assert_eq!(
                            sum_of_products(&down, &terms),
                            crate::mul::sum_of_products(&down, &terms)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn wide_ratio_matches_the_exact_division() {
        let values = [
            r(1, 0),
            r(-3, 5),
            r(5, -70),
            r(7, 200),
            r(-1, -200),
            r(0, 0),
        ];
        let exact = FloatCtxt::<mode::HalfEven>::new(0);
        for p in [1usize, 2, 20, 50, 100, 500] {
            for x in &values {
                for y in &values {
                    for u in &values {
                        for v in &values[..5] {
                            let num = [(x, u, Positive), (y, v, Positive)];
                            let den = [(u, u, Positive), (v, v, Positive)];
                            let n = crate::mul::sum_of_products(&exact, &num).unwrap().value();
                            let d = crate::mul::sum_of_products(&exact, &den).unwrap().value();
                            macro_rules! check {
                                ($mode:ty) => {
                                    let ctx = FloatCtxt::<$mode>::new(p);
                                    let got = ratio_of_sums(&ctx, &num, &den);
                                    let want = ctx.div(n.repr(), d.repr());
                                    assert_eq!(
                                        got.map(|v| v.map(|f| f.into_repr())),
                                        want.map(|v| v.map(|f| f.into_repr())),
                                        "({x:?}·{u:?} + {y:?}·{v:?}) / ({u:?}² + {v:?}²) at {p}"
                                    );
                                };
                            }
                            check!(mode::HalfEven);
                            check!(mode::HalfAway);
                            check!(mode::Down);
                            check!(mode::Up);
                            check!(mode::Zero);
                            check!(mode::Away);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sticky_rounding() {
        let one = r(1, 0);
        let value = |r: FpResult<FBig<_, 2>>| r.unwrap().value().into_repr();
        for p in [1usize, 20, 53] {
            let down = FloatCtxt::<mode::Down>::new(p);
            let up = FloatCtxt::<mode::Up>::new(p);
            let even = FloatCtxt::<mode::HalfEven>::new(p);
            // just below 1: the largest number below 1 under Down, 1 otherwise
            let below = Repr::<2>::new((IBig::ONE << p) - IBig::ONE, -(p as isize));
            assert_eq!(value(round_sticky(&down, &one, Negative).map(|v| v.map(retag))), below);
            assert_eq!(value(round_sticky(&up, &one, Negative).map(|v| v.map(retag))), one);
            assert_eq!(value(round_sticky(&even, &one, Negative)), one);
            assert!(matches!(round_sticky(&even, &one, Positive), Ok(Inexact(..))));
            // a midpoint is decided by the sticky part, not by the tie rule
            let mid = Repr::<2>::new((IBig::ONE << p) + IBig::ONE, -(p as isize));
            let above = Repr::<2>::new((IBig::ONE << (p - 1)) + IBig::ONE, 1 - p as isize);
            if p > 1 {
                assert_eq!(value(round_sticky(&even, &mid, Positive)), above);
                assert_eq!(value(round_sticky(&even, &mid, Negative)), one);
            }
            // at the bottom of the exponent range
            let tiny = r(-1, isize::MIN + 2 + p as isize);
            let rounded = value(round_sticky(&up, &tiny, Positive).map(|v| v.map(retag)));
            assert_eq!(rounded.exponent(), isize::MIN + 2);
        }
        // 2·x − y·y with its magnitude bound
        let far: isize = 1 << 40;
        let (two, x, y) = (r(2, 0), r(1, -100), r(1, -far));
        let terms = [(&two, &x, Positive), (&y, &y, Negative)];
        // 2^-99 less a negligible square: the bound is within a few digits of it
        let (sign, bound) = sign_and_magnitude(&terms).unwrap();
        assert!(sign == Positive && (-103..=-99).contains(&bound), "{bound}");
        let y = r(1, -49);
        let terms = [(&two, &x, Positive), (&y, &y, Negative)];
        assert_eq!(sign_and_magnitude(&terms).map(|(s, _)| s), Some(Negative));
        let (x, y) = (r(1, -101), r(1, -50));
        assert_eq!(sign_and_magnitude(&[(&two, &x, Positive), (&y, &y, Negative)]), None);
    }

    /// The same value under the test's default mode.
    fn retag<R: Round>(v: FBig<R, 2>) -> F {
        F::from_repr(v.into_repr(), FloatCtxt::new(0))
    }

    #[test]
    fn products_beyond_the_range() {
        let ctx = FloatCtxt::<mode::HalfEven>::new(53);
        let (one, big, tiny) = (r(1, 0), r(1, FAR + 10), r(1, -FAR - 10));
        // 1 − tiny² rounds to 1, 1 + big·tiny is 2
        let sum = sum_of_products(&ctx, &[(&one, &one, Positive), (&tiny, &tiny, Negative)]);
        assert_eq!(sum, Ok(Inexact(F::ONE, Rounding::AddOne)));
        let sum = sum_of_products(&ctx, &[(&one, &one, Positive), (&big, &tiny, Positive)]);
        assert_eq!(sum, Ok(Exact(F::from(2))));
        // results beyond the range are still errors
        let sum = sum_of_products(&ctx, &[(&one, &one, Positive), (&big, &big, Negative)]);
        assert_eq!(sum, Err(FpError::Overflow(Negative)));
        let sum = sum_of_products(&ctx, &[(&tiny, &tiny, Negative)]);
        assert_eq!(sum, Err(FpError::Underflow(Negative)));
        // big² / (big² + 1) rounds to 1; 1 / (big² + tiny²) is below the range
        let den = [(&big, &big, Positive), (&one, &one, Positive)];
        let q = ratio_of_sums(&ctx, &[(&big, &big, Positive)], &den);
        assert_eq!(q, Ok(Inexact(F::ONE, Rounding::AddOne)));
        let den = [(&big, &big, Positive), (&tiny, &tiny, Positive)];
        let q = ratio_of_sums(&ctx, &[(&one, &one, Negative)], &den);
        assert_eq!(q, Err(FpError::Underflow(Negative)));
    }
}
