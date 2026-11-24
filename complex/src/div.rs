//! Complex division and reciprocal.
//!
//! `(x+iy)/(u+iv) = ((xu+yv) + i(yu−xv)) / (u²+v²)`: the two numerators and the denominator are
//! formed **exactly**, and each part is then a single correctly-rounded real division — so the
//! quotient is correctly rounded per component (`z / z` is exactly `1`), with the inexactness
//! flags and directed-rounding directions of the exact value.
//!
//! The exact intermediates are as wide as the exponent spread between the operand parts, so when
//! that spread is astronomically large (or `|w|²` leaves the exponent range) the division falls
//! back to Smith's overflow-safe method at guard precision, which is only near-correctly rounded.

use crate::cbig::CBig;
use crate::mul::{sum_of_products, Term};
use crate::repr::{combine_results, riemann, CfpResult, Context};
use core::ops::{Div, DivAssign};
use dashu_base::{AbsOrd, Inverse, Sign::*};
use dashu_float::round::Round;
use dashu_float::{Context as FloatCtxt, FBig, FpError, FpResult, Repr};
use dashu_int::Word;

/// Guard digits (base-B) for the Smith's-method fallback of `div`/`inv`. The naive
/// complex-division error is `~(3+√5)·u`; a fixed guard comfortably absorbs it for
/// well-conditioned denominators.
const DIV_GUARD: usize = 14;

/// Whether the exact sum of the given products is cheap to form: its width — the distance from
/// the lowest digit position of any product to the highest — stays within a small multiple of
/// the operand widths and the target precision (plus a fixed allowance that covers every
/// practical exponent spread).
fn exact_sum_is_cheap<const B: Word>(terms: &[Term<'_, B>], precision: usize) -> bool {
    let mut digits = precision;
    let mut range: Option<(isize, isize)> = None;
    for (a, b, _) in terms {
        if a.significand().is_zero() || b.significand().is_zero() {
            continue;
        }
        let width = a.digits_ub() + b.digits_ub();
        digits = digits.saturating_add(width);
        let bounds = a.exponent().checked_add(b.exponent()).and_then(|lo| {
            let hi = lo.checked_add(isize::try_from(width).ok()?)?;
            Some((lo, hi))
        });
        let (lo, hi) = match bounds {
            Some(bounds) => bounds,
            None => return false,
        };
        range = Some(match range {
            None => (lo, hi),
            Some((l, h)) => (l.min(lo), h.max(hi)),
        });
    }
    match range {
        None => true,
        Some((lo, hi)) => match hi.checked_sub(lo) {
            Some(span) => span as usize <= digits.saturating_mul(8).saturating_add(4096),
            None => false,
        },
    }
}

impl<R: Round> Context<R> {
    /// Reciprocal `1/z = conj(z)/|z|²` under this context (context layer).
    ///
    /// Each part is correctly rounded (see [`Context::div`] for the one exception).
    pub fn inv<const B: Word>(&self, z: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() {
            // An infinite input is a terminal value and is not accepted.
            return Err(FpError::InfiniteInput);
        }
        if z.is_zero() {
            return Ok(riemann(*self)); // 1/0 = ∞
        }
        let (u, v) = (z.re(), z.im());
        let (one, zero) = (Repr::one(), Repr::zero());
        // 1/z = (u − iv)/(u² + v²)
        self.quotient(&[(&one, u, Positive)], &[(&one, v, Negative)], (&one, &zero), (u, v))
    }

    /// Divide two complex numbers under this context (context layer).
    ///
    /// Each part is correctly rounded: the numerators `xu + yv`, `yu − xv` and the denominator
    /// `u² + v²` are formed exactly and each part is one correctly-rounded real division. The one
    /// exception is an astronomically large exponent spread between the operand parts (or a
    /// `|w|²` outside the exponent range), where the exact intermediates would be impractically
    /// wide: the quotient is then computed by Smith's overflow-safe method and is only
    /// near-correctly rounded.
    ///
    /// `z / 0` is the complex infinity `+∞ + 0i` for a nonzero `z`, `0 / 0` is
    /// [`FpError::Indeterminate`], and an infinite operand is rejected with
    /// [`FpError::InfiniteInput`].
    pub fn div<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        if z.is_infinite() || w.is_infinite() {
            // An infinite operand is a terminal value and is never accepted — also not over a
            // zero divisor.
            return Err(FpError::InfiniteInput);
        }
        if w.is_zero() {
            return if z.is_zero() {
                Err(FpError::Indeterminate) // 0/0
            } else {
                Ok(riemann(*self)) // z/0 (z ≠ 0) = ∞
            };
        }
        let (x, y) = (z.re(), z.im());
        let (u, v) = (w.re(), w.im());
        let re_num = [(x, u, Positive), (y, v, Positive)];
        let im_num = [(y, u, Positive), (x, v, Negative)];
        if z.is_zero() {
            // 0/w (w ≠ 0) = 0 at this context's precision; the zero numerators carry the signs
            // (the denominator is positive).
            let fctx = self.float();
            return combine_results(
                sum_of_products(&fctx, &re_num),
                sum_of_products(&fctx, &im_num),
            );
        }
        self.quotient(&re_num, &im_num, (x, y), (u, v))
    }

    /// `(re_num + i·im_num) / (u² + v²)` for a finite nonzero divisor `w = u + iv`, where the
    /// numerators are the sums of products of `z·conj(w)` for the finite numerator `z`.
    fn quotient<const B: Word>(
        &self,
        re_num: &[Term<'_, B>],
        im_num: &[Term<'_, B>],
        z: (&Repr<B>, &Repr<B>),
        w: (&Repr<B>, &Repr<B>),
    ) -> CfpResult<R, B> {
        let (u, v) = w;
        let den = [(u, u, Positive), (v, v, Positive)];
        match self.exact_quotient(re_num, im_num, &den) {
            Some((re, im)) => combine_results(re, im),
            None => self.div_smith(z, w),
        }
    }

    /// The two parts of the quotient, each correctly rounded from the exact numerator and
    /// denominator; `None` when those are not practical to form (see [`exact_sum_is_cheap`]) or
    /// leave the exponent range.
    #[allow(clippy::type_complexity)]
    fn exact_quotient<const B: Word>(
        &self,
        re_num: &[Term<'_, B>],
        im_num: &[Term<'_, B>],
        den: &[Term<'_, B>],
    ) -> Option<(FpResult<FBig<R, B>>, FpResult<FBig<R, B>>)> {
        let p = self.precision();
        if ![re_num, im_num, den]
            .iter()
            .all(|terms| exact_sum_is_cheap(terms, p))
        {
            return None;
        }
        let exact = FloatCtxt::<R>::new(0);
        let den = sum_of_products(&exact, den).ok()?.value();
        let re_num = sum_of_products(&exact, re_num).ok()?.value();
        let im_num = sum_of_products(&exact, im_num).ok()?.value();
        let fctx = self.float();
        Some((fctx.div(re_num.repr(), den.repr()), fctx.div(im_num.repr(), den.repr())))
    }

    /// Smith's overflow-safe division `(x+iy)/(u+iv)` at guard precision (near-correctly
    /// rounded): the branch `|u| >= |v|` avoids forming `|denominator|²`.
    fn div_smith<const B: Word>(
        &self,
        (x, y): (&Repr<B>, &Repr<B>),
        (u, v): (&Repr<B>, &Repr<B>),
    ) -> CfpResult<R, B> {
        let gctx = self.work_context(DIV_GUARD);
        let p = self.precision();
        // Determine |u| >= |v| via abs_cmp on temporary FBig views (Smith's method).
        let u_ge_v = {
            let fu = FBig::from_repr(u.clone(), FloatCtxt::<R>::new(0));
            let fv = FBig::from_repr(v.clone(), FloatCtxt::<R>::new(0));
            fu.abs_cmp(&fv).is_ge()
        };

        // r, d depend on which of |u|, |v| is larger
        let (r, d) = if u_ge_v {
            // r = v/u, d = u + r·v  (fuse r·v with the add via FMA)
            let r = gctx.div(v, u)?.value();
            let d = gctx.fma(r.repr(), v, u, Positive)?.value();
            (r, d)
        } else {
            // r = u/v, d = v + r·u
            let r = gctx.div(u, v)?.value();
            let d = gctx.fma(r.repr(), u, v, Positive)?.value();
            (r, d)
        };

        let (num_re, num_im) = if u_ge_v {
            // re = (x + r·y)/d, im = (y − r·x)/d  (both fuse via FMA)
            (
                gctx.fma(r.repr(), y, x, Positive)?.value(),
                gctx.fma(r.repr(), x, y, Negative)?.value(),
            )
        } else {
            // re = (r·x + y)/d, im = (r·y − x)/d. The numerator products fuse via
            // FMA; `r·y − x` is `−(x − r·y)`, so one FMA plus an exact negation.
            (
                gctx.fma(r.repr(), x, y, Positive)?.value(),
                -gctx.fma(r.repr(), y, x, Negative)?.value(),
            )
        };
        let re = gctx
            .div(num_re.repr(), d.repr())
            .map(|q| q.value().with_precision(p));
        let im = gctx
            .div(num_im.repr(), d.repr())
            .map(|q| q.value().with_precision(p));
        combine_results(re, im)
    }

    /// Divide a complex number by a real scalar (context layer): `(x+iy)/s = (x/s) + i(y/s)`.
    ///
    /// Each part is a single correctly-rounded real division.
    pub fn div_real<const B: Word>(&self, z: &CBig<R, B>, s: &FBig<R, B>) -> CfpResult<R, B> {
        if !z.is_infinite() && (s.repr().is_pos_zero() || s.repr().is_neg_zero()) {
            // Division by zero for a finite numerator produces ∞ as a terminal output.
            if z.is_zero() {
                return Err(FpError::Indeterminate); // 0/0 (incl. ±0)
            }
            return Ok(riemann(*self)); // z/±0 (z≠0) = ∞
        }
        // An infinite operand (numerator or divisor) is a terminal value: it falls through and the
        // float layer rejects it (panicking at the convenience layer).
        let re = self.float().div(z.re(), s.repr());
        let im = self.float().div(z.im(), s.repr());
        combine_results(re, im)
    }
}

impl<R: Round, const B: Word> Inverse for CBig<R, B> {
    type Output = CBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context().unwrap_cfp(self.context().inv(&self))
    }
}

impl<R: Round, const B: Word> Inverse for &CBig<R, B> {
    type Output = CBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context().unwrap_cfp(self.context().inv(self))
    }
}

// CBig / CBig operators
crate::helper_macros::impl_cbig_binop!(Div, div, DivAssign, div_assign);

// --- scalar division by a real FBig (mixed-type operators) ---

// CBig / FBig (componentwise, via the shared scalar macro).
crate::helper_macros::impl_cbig_scalar_binop!(Div, div, div_real);

// FBig / CBig = (s + 0i) / z, reusing complex division.
impl<R: Round, const B: Word> Div<&CBig<R, B>> for &FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn div(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        let s = CBig::from(self.clone());
        let ctx = Context::max(s.context(), rhs.context());
        ctx.unwrap_cfp(ctx.div(&s, rhs))
    }
}
impl<R: Round, const B: Word> Div<CBig<R, B>> for &FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn div(self, rhs: CBig<R, B>) -> CBig<R, B> {
        let this = self.clone();
        let s = CBig::from(this);
        let ctx = Context::max(s.context(), rhs.context());
        ctx.unwrap_cfp(ctx.div(&s, &rhs))
    }
}
impl<R: Round, const B: Word> Div<&CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn div(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        let s = CBig::from(self);
        let ctx = Context::max(s.context(), rhs.context());
        ctx.unwrap_cfp(ctx.div(&s, rhs))
    }
}
impl<R: Round, const B: Word> Div<CBig<R, B>> for FBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn div(self, rhs: CBig<R, B>) -> CBig<R, B> {
        let s = CBig::from(self);
        let ctx = Context::max(s.context(), rhs.context());
        ctx.unwrap_cfp(ctx.div(&s, &rhs))
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
    fn div_inverse() {
        // z / z = 1 for z != 0
        let z = c(3, 4);
        let q = &z / &z;
        assert_eq!(q.re().significand(), &1.into());
        assert!(q.im().significand().is_zero());
    }

    #[test]
    fn div_basic() {
        // (6+8i)/(3+4i) = 2  (since 6+8i = 2·(3+4i))
        let z = c(6, 8);
        let w = c(3, 4);
        let q = &z / &w;
        assert_eq!(q.re().significand(), &2.into());
        assert!(q.im().significand().is_zero());
    }

    #[test]
    fn inv_basic() {
        // 1/(3+4i) = (3-4i)/25 = 0.12 - 0.16i — use precision 53 for exact-ish
        type F = FBig<mode::HalfAway, 10>;
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        let z = C::from_parts(mk(3), mk(4));
        let r = (&z).inv();
        // (3-4i)/25: re = 3/25, im = -4/25
        assert_eq!(r.context().precision(), 53);
        // re ≈ 0.12, im ≈ -0.16; check via multiplying back: z·inv(z) = 1
        let one = &z * &r;
        assert_eq!(one.re().significand(), &1.into());
        assert!(one.im().significand().is_zero());
    }

    #[test]
    fn scalar_div_by_real() {
        let z = c(6, 8);
        let s = FBig::<mode::HalfAway, 10>::from(2);
        let q = &z / &s;
        assert_eq!(q.re().significand(), &3.into());
        assert_eq!(q.im().significand(), &4.into());
    }

    #[test]
    fn div_real_by_negative_zero_matches_positive_zero() {
        use dashu_float::Repr;
        // Dividing a nonzero complex by the scalar -0 must give the same Riemann infinity as +0,
        // not fall through to the general path (which would divide by -0 componentwise).
        let z = c(3, 4);
        let neg_zero = F::from_repr(Repr::neg_zero(), z.context().float());
        let q_neg = &z / &neg_zero;
        let q_pos = &z / &F::ZERO;
        assert!(q_neg.is_infinite());
        assert!(q_neg == q_pos); // both are +∞ + i·0
    }

    // `div` at unlimited precision panics via the float layer (`work_context` → `self.float()` →
    // float `div`, which rejects unlimited: a quotient isn't exactly representable in general).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_div_unlimited_panics() {
        let one = C::ONE;
        let i = C::I;
        let _ = &one / &i; // 1/i at unlimited
    }
}
