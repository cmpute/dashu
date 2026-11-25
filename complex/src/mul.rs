//! Complex squaring, multiplication and fused multiply–add.
//!
//! Every result part is a sum of two or three products of the operand parts. Each part is formed
//! **exactly** and rounded a single time ([`sum_of_products`]), so the result is correctly rounded
//! per component — including under catastrophic cancellation — and the inexactness flags and
//! directed-rounding directions are those of the exact value.

use crate::cbig::CBig;
use crate::repr::{combine_results, CfpResult, Context};
use alloc::vec::Vec;
use core::ops::{Mul, MulAssign};
use dashu_base::AbsOrd;
use dashu_base::Approximation::Exact;
use dashu_base::Sign::{self, *};
use dashu_float::round::Round;
use dashu_float::{Context as FloatCtxt, FBig, FpError, FpResult, Repr};
use dashu_int::{IBig, Word};

/// One addend `sign · a · b` of a [`sum_of_products`].
pub(crate) type Term<'a, const B: Word> = (&'a Repr<B>, &'a Repr<B>, Sign);

/// The correctly rounded value of `Σ signᵢ·aᵢ·bᵢ` under `ctx`: the products and their sum are
/// formed exactly and rounded once, so the value, its inexactness flag and (for directed modes)
/// its direction are those of the exact result. At unlimited precision the result is exact.
///
/// Zero results follow IEEE 754 §6.3: a sum of like-signed zero products keeps that sign, and an
/// exact cancellation (or a sum of opposite-signed zeros) is `+0`, or `-0` when rounding toward
/// negative infinity.
///
/// An addend lying entirely below the rounding window of the partial sum of the larger ones only
/// matters through its sign, so it is replaced by a unit just below that window instead of being
/// aligned — the exact sum never materializes the exponent gap between the addends.
///
/// Returns [`FpError::InfiniteInput`] for an infinite operand, and [`FpError::Overflow`] /
/// [`FpError::Underflow`] when the result leaves the exponent range. A product that leaves the
/// range on its own does not: the sum is then formed over a wider exponent range (at unlimited
/// precision, where nothing may be dropped, the product's error is returned).
pub(crate) fn sum_of_products<R: Round, const B: Word>(
    ctx: &FloatCtxt<R>,
    terms: &[Term<'_, B>],
) -> FpResult<FBig<R, B>> {
    if terms
        .iter()
        .any(|(a, b, _)| a.is_infinite() || b.is_infinite())
    {
        return Err(FpError::InfiniteInput);
    }

    let exact = FloatCtxt::<R>::new(0);
    let zero = |negative: bool| {
        let repr = if negative {
            Repr::neg_zero()
        } else {
            Repr::zero()
        };
        Exact(FBig::from_repr(repr, *ctx))
    };

    // the exact nonzero products, and the signs of the zero ones
    let (mut pos_zero, mut neg_zero) = (false, false);
    let mut prods: Vec<FBig<R, B>> = Vec::with_capacity(terms.len());
    for &(a, b, sign) in terms {
        if a.significand().is_zero() || b.significand().is_zero() {
            match sign * a.sign() * b.sign() {
                Positive => pos_zero = true,
                Negative => neg_zero = true,
            }
            continue;
        }
        let prod = match exact.mul(a, b) {
            Ok(prod) => prod.value(),
            // A product outside the exponent range does not put the sum there: it can cancel
            // against another one, or vanish below the rounding window of the rest.
            Err(FpError::Overflow(_) | FpError::Underflow(_)) if ctx.precision() != 0 => {
                return crate::wide::sum_of_products(ctx, terms);
            }
            Err(e) => return Err(e),
        };
        prods.push(if sign == Negative { -prod } else { prod });
    }
    if prods.is_empty() {
        return Ok(zero(neg_zero && (!pos_zero || R::IS_ROUND_TOWARD_NEGATIVE)));
    }

    // accumulate from the largest magnitude down
    prods.sort_unstable_by(|l, r| r.abs_cmp(l));
    let p = ctx.precision();
    let mut acc: Option<FBig<R, B>> = None;
    for (i, t) in prods.iter().enumerate() {
        let partial = match &acc {
            None => {
                acc = Some(t.clone());
                continue;
            }
            Some(partial) => partial,
        };
        if p != 0 {
            // `t` and everything after it (they are no larger) sit below the rounding window
            // of the partial sum: only the sign of their total can still affect the rounding.
            let below = match (sticky_position(partial.repr(), p), top_ub(t.repr())) {
                (Some(k), Some(top)) if top <= k => Some(k),
                _ => None,
            };
            if let (Some(k), Some(tail)) = (below, tail_sign(&prods[i..])) {
                if let Some(sign) = tail {
                    let unit = Repr::new(IBig::ONE * sign, k);
                    acc = Some(exact.add(partial.repr(), &unit)?.value());
                }
                break;
            }
        }
        let sum = exact.add(partial.repr(), t.repr())?.value();
        acc = if sum.repr().significand().is_zero() {
            None
        } else {
            Some(sum)
        };
    }

    match acc {
        // exact cancellation of nonzero addends
        None => Ok(zero(R::IS_ROUND_TOWARD_NEGATIVE)),
        Some(sum) => {
            let rounded = sum.with_precision(p);
            if rounded.value_ref().repr().is_infinite() {
                // the rounding carried the exponent out of range
                return Err(FpError::Overflow(rounded.value_ref().repr().sign()));
            }
            Ok(rounded)
        }
    }
}

/// An upper bound on the position just above the top digit of a nonzero `v`
/// (`|v| < B^top`), or `None` if it does not fit in an `isize`.
fn top_ub<const B: Word>(v: &Repr<B>) -> Option<isize> {
    v.exponent()
        .checked_add(isize::try_from(v.digits_ub()).ok()?)
}

/// A position `k` such that replacing a nonzero perturbation of magnitude below `2·B^k` by
/// `±B^k` (same sign) cannot change how the nonzero `v + perturbation` rounds to `precision`
/// digits, or `None` if it does not fit in an `isize`.
///
/// `v` and every rounding boundary near it (a multiple of half an ulp, with the ulp at least
/// `B^(top − 1 − precision)` even when the perturbation pulls `v` below a power of the base) are
/// multiples of `g/2` for `g = B^min(exponent, top − 1 − precision)`. A perturbation below `g/2`
/// in magnitude therefore keeps the sum strictly between `v` and the adjacent boundary, where
/// only its sign matters. `k` leaves three digits of margin below `g`, so that `2·B^k < g/2` in
/// every base.
fn sticky_position<const B: Word>(v: &Repr<B>, precision: usize) -> Option<isize> {
    let top_lb = v
        .exponent()
        .checked_add(isize::try_from(v.digits_lb()).ok()?)?;
    let window = top_lb.checked_sub(isize::try_from(precision).ok()?.checked_add(1)?)?;
    v.exponent().min(window).checked_sub(3)
}

/// The sign of the exact sum of the (nonzero) `tail` addends: `Some(None)` if they cancel
/// exactly, and `None` if this is not decided here (more than two addends).
fn tail_sign<R: Round, const B: Word>(tail: &[FBig<R, B>]) -> Option<Option<Sign>> {
    match tail {
        [t] => Some(Some(t.sign())),
        [t, u] if t.sign() == u.sign() => Some(Some(t.sign())),
        [t, u] => Some(match t.abs_cmp(u) {
            core::cmp::Ordering::Greater => Some(t.sign()),
            core::cmp::Ordering::Less => Some(u.sign()),
            core::cmp::Ordering::Equal => None,
        }),
        _ => None,
    }
}

impl<R: Round> Context<R> {
    /// Square a complex number under this context: `(x+iy)² = (x²-y²) + i(2xy)`.
    ///
    /// Each part is correctly rounded (the exact value rounded once).
    pub fn sqr<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        let (x, y) = (z.re(), z.im());
        let fctx = self.float();
        // real part: x² − y²
        let re = sum_of_products(&fctx, &[(x, x, Positive), (y, y, Negative)]);
        // imaginary part: 2·x·y
        let im = sum_of_products(&fctx, &[(x, y, Positive), (x, y, Positive)]);
        combine_results(re, im)
    }

    /// Multiply two complex numbers under this context: `(x+iy)(u+iv) = (xu-yv) + i(xv+yu)`.
    ///
    /// Each part is correctly rounded (the exact value rounded once), so a real part that nearly
    /// cancels is still accurate to the last digit.
    pub fn mul<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        let (x, y) = (z.re(), z.im());
        let (u, v) = (w.re(), w.im());
        let fctx = self.float();
        // real part: xu − yv
        let re = sum_of_products(&fctx, &[(x, u, Positive), (y, v, Negative)]);
        // imaginary part: xv + yu
        let im = sum_of_products(&fctx, &[(x, v, Positive), (y, u, Positive)]);
        combine_results(re, im)
    }

    /// Multiply a complex number by a real scalar (context layer): `(x+iy)·s = (xs) + i(ys)`.
    ///
    /// Each part is a single correctly-rounded real product.
    pub fn mul_real<const B: Word>(&self, z: &CBig<R, B>, s: &FBig<R, B>) -> CfpResult<R, B> {
        let re = self.float().mul(z.re(), s.repr());
        let im = self.float().mul(z.im(), s.repr());
        combine_results(re, im)
    }

    /// Fused complex multiply–add under this context: `z1·z2 + sign·z3`. Each part of the exact
    /// result is rounded once, so the result is correctly rounded per component. `sign` scales
    /// `z3` ([`Sign::Positive`] adds it, [`Sign::Negative`] subtracts).
    ///
    /// An infinite operand is a terminal value and is not accepted — it is rejected with
    /// [`FpError::InfiniteInput`] (panicking at the
    /// convenience layer).
    pub fn fma<const B: Word>(
        &self,
        z1: &CBig<R, B>,
        z2: &CBig<R, B>,
        z3: &CBig<R, B>,
        sign: Sign,
    ) -> CfpResult<R, B> {
        let (a, b) = (z1.re(), z1.im());
        let (c, d) = (z2.re(), z2.im());
        let (e, f) = (z3.re(), z3.im());
        let fctx = self.float();
        let one = Repr::one();
        // real part: a·c − b·d ± e
        let re = sum_of_products(&fctx, &[(a, c, Positive), (b, d, Negative), (e, &one, sign)]);
        // imaginary part: a·d + b·c ± f
        let im = sum_of_products(&fctx, &[(a, d, Positive), (b, c, Positive), (f, &one, sign)]);
        combine_results(re, im)
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
    /// Square the complex number (convenience layer).
    #[inline]
    pub fn sqr(&self) -> Self {
        self.context().unwrap_cfp(self.context().sqr(self))
    }

    /// Fused complex multiply–add `self·b + sign·c` (convenience layer, see [`Context::fma`]).
    ///
    /// The result is rounded at the largest precision of the three operands (an unlimited
    /// precision counts as the lowest), like the binary operators and [`FBig::fma`].
    #[inline]
    pub fn fma(&self, b: &Self, c: &Self, sign: Sign) -> Self {
        let context = Context::max(self.context(), Context::max(b.context(), c.context()));
        context.unwrap_cfp(context.fma(self, b, c, sign))
    }
}

// CBig · CBig operators — forwarded through the standard macro (mirroring `dashu-float`'s `mul.rs`).
crate::helper_macros::impl_cbig_binop!(Mul, mul, MulAssign, mul_assign);

// --- scalar multiplication by a real FBig (mixed-type operators) ---

// CBig · FBig (componentwise, via the shared scalar macro).
crate::helper_macros::impl_cbig_scalar_binop!(Mul, mul, mul_real);

// FBig · CBig (commutative: FBig·CBig = CBig·FBig).
impl<R: Round, const B: Word> Mul<&CBig<R, B>> for &FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        rhs * self
    }
}
impl<R: Round, const B: Word> Mul<CBig<R, B>> for &FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: CBig<R, B>) -> CBig<R, B> {
        &rhs * self
    }
}
impl<R: Round, const B: Word> Mul<&CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        rhs * &self
    }
}
impl<R: Round, const B: Word> Mul<CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn mul(self, rhs: CBig<R, B>) -> CBig<R, B> {
        &rhs * &self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        C::from_parts(mk(re), mk(im))
    }

    #[test]
    fn sqr_basic() {
        // (3+4i)² = -7+24i
        let z = c(3, 4);
        let s = z.sqr();
        assert_eq!(s.re().significand(), &(-7i32).into());
        assert_eq!(s.im().significand(), &24.into());
    }

    #[test]
    fn mul_basic() {
        // (1+2i)(3+4i) = -5+10i  (compare full values: 10 normalizes to 1·10¹ in base 10)
        let z = c(1, 2);
        let w = c(3, 4);
        let p = &z * &w;
        assert!(p == c(-5, 10));
    }

    #[test]
    fn mul_assign_val_and_ref() {
        let z = c(1, 2);
        let w = c(3, 4);
        // (1+2i)(3+4i) = -5+10i
        let mut acc = z.clone();
        acc *= w.clone();
        assert!(acc == c(-5, 10));
        let mut acc = z.clone();
        acc *= &w;
        assert!(acc == c(-5, 10));
    }

    #[test]
    fn mul_by_one_is_identity() {
        let z = c(3, 4);
        let p = &z * &CBig::ONE;
        assert!(p == z);
    }

    #[test]
    fn mul_by_conj_is_norm() {
        // z·conj(z) = norm(z), purely real
        let z = c(3, 4);
        let p = &z * &z.conj();
        assert!(p.im().is_pos_zero() || p.im().is_neg_zero());
        assert_eq!(p.re().significand(), &25.into());
    }

    #[test]
    fn scalar_mul_by_real() {
        let z = c(3, 4);
        let s = FBig::<mode::HalfAway, 10>::from(2);
        let p = &z * &s;
        assert_eq!(p.re().significand(), &6.into());
        assert_eq!(p.im().significand(), &8.into());
        // commutes: s * z
        let p2 = &s * &z;
        assert_eq!(p2.re().significand(), &6.into());
    }

    // At unlimited precision `mul`/`sqr` are exact (the exact sum of products is not rounded), so this
    // must NOT panic (unlike the transcendentals). i·i = −1 exactly.
    #[test]
    fn mul_at_unlimited_is_exact() {
        let i = C::I;
        assert_eq!(&i * &i, C::NEG_ONE);
    }

    #[test]
    fn fma_basic() {
        // (1+2i)(3+4i) = -5+10i
        let z1 = c(1, 2);
        let z2 = c(3, 4);
        let z3 = c(5, 6);
        // + z3: (-5+10i) + (5+6i) = 0 + 16i
        let r = z1.fma(&z2, &z3, Positive);
        assert!(r == c(0, 16));
        // − z3: (-5+10i) − (5+6i) = -10 + 4i
        let r = z1.fma(&z2, &z3, Negative);
        assert!(r == c(-10, 4));
    }

    #[test]
    fn sqr_signed_zero() {
        // (x+iy)² = (x²−y²) + i·2xy: the real part is always `+0`, and the imaginary part is
        // `2·x·y` — `−0` iff the two zero parts are opposite-signed.
        let fctx = dashu_float::Context::<mode::HalfAway>::new(53);
        let neg_zero = F::from_repr(Repr::neg_zero(), fctx);
        let pos_zero = F::from_repr(Repr::zero(), fctx);
        let s = C::from_parts(neg_zero.clone(), pos_zero.clone()).sqr(); // -0 + i·0
        assert!(s.re().is_pos_zero());
        assert!(s.im().is_neg_zero());
        let s = C::from_parts(pos_zero.clone(), neg_zero.clone()).sqr(); // +0 - i·0
        assert!(s.re().is_pos_zero());
        assert!(s.im().is_neg_zero());
        let s = C::from_parts(neg_zero.clone(), neg_zero).sqr(); // -0 - i·0
        assert!(s.re().is_pos_zero());
        assert!(s.im().is_pos_zero());
    }
}
