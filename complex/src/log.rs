//! Complex natural logarithm `log(z) = ln|z| + i·arg(z)` (principal branch; cut on `]−∞, 0]`).

use crate::ball::{is_zero, kernel_ball, CBall};
use crate::cbig::CBig;
use crate::mul::sum_of_products;
use crate::repr::{combine_parts, combine_results, exact, reborrow_cache, CfpResult, Context};
use crate::wide::round_sticky;
use dashu_base::{Approximation, Sign};
use dashu_float::round::ErrorBounds;
use dashu_float::{Ball, ConstCache, Context as FloatCtxt, FBig, FpError, FpResult, Repr};
use dashu_int::{IBig, Word};

/// Guard digits (base-B) for `log`. Composes `hypot` (for `|z|`), `ln`, and `atan2`.
const LOG_GUARD: usize = 14;

/// The most terms of the series `ln(1 + s) = s − s²/2 + s³/3 − …` that [`ln_1p_ball`] sums
/// itself.
const LN_1P_SERIES_TERMS: i128 = 16;

/// `ln(1 + s)` for a nonzero ball `s` with `−1/2 <= s < 1`, at working precision `prec`.
///
/// A small `s` is summed through its series, each step a tracked ball operation. The value
/// is then `s` times a few simple fractions, which in a base where those are not
/// representable (`1/2` in an odd base) sits next to a rounding tie of every working
/// precision until the next term shows: a ball follows that at no cost, where a correctly
/// rounded kernel has to resolve the tie at each working precision first. Otherwise the
/// float `ln_1p` kernel runs on the midpoint.
fn ln_1p_ball<R: ErrorBounds, const B: Word>(
    s: &Ball<B>,
    fctx: &FloatCtxt<R>,
    prec: usize,
    cache: Option<&mut ConstCache>,
) -> Result<Ball<B>, FpError> {
    // `|s.mid| < B^top`
    let top = s.mid.exponent() as i128 + s.mid.digits_ub() as i128;
    // the terms from `s^(n+1)` on are below the working precision relative to `s`
    let terms = if top < 0 {
        (prec as i128 + 1 - top - 1) / -top
    } else {
        i128::MAX
    };
    if terms <= LN_1P_SERIES_TERMS {
        let int = |n: i128| Ball::<B>::exact(Repr::new(IBig::from(n), 0));
        let one = int(1);
        // Horner: s·(1 − s·(1/2 − s·(1/3 − … s/n)))
        let mut acc = one.div(&int(terms), prec)?;
        for j in (1..terms).rev() {
            acc = one.div(&int(j), prec)?.sub(&s.mul(&acc, prec)?, prec)?;
        }
        let mut ln = s.mul(&acc, prec)?;
        // the tail: |Σ_{j>n} (−s)^j/j| ≤ |s|^(n+1)·(1/(n+1))/(1 − |s|) ≤ |s|^(n+1)
        let bound = s.mag();
        let mut tail = bound;
        for _ in 0..terms {
            tail = tail.mul(&bound);
        }
        ln.add_error(tail);
        return Ok(ln);
    }
    // ln(1 + s) is exact only at s = 0
    let mut ln = kernel_ball(fctx.ln_1p(&s.mid, cache)?, prec, |_| is_zero(&s.mid));
    // |Δln(1 + s)| ≤ rad/(1 + s − rad) ≤ 4·rad, as 1 + s ≥ 1/2 and rad is far below 1/4
    ln.add_error(s.rad.mul2().mul2());
    Ok(ln)
}

impl<R: ErrorBounds> Context<R> {
    /// Complex natural logarithm under this context (context layer). `log z = ln|z| + i·arg(z)`,
    /// with the imaginary part in `]−π, π]`. The cache threads into `ln` and `atan2`.
    ///
    /// Special values: `log(0) = -∞ + i·0`; `log(±∞) = +∞`; the branch cut on `]−∞, 0]` is handled
    /// by the signed-zero `atan2` (so `log(-r ± i0) = ln r ± iπ`).
    pub fn log<const B: Word>(
        &self,
        z: &CBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> CfpResult<R, B> {
        if z.is_zero() {
            let (re, im) = (z.re(), z.im());
            let neg_inf = FBig::from_repr(Repr::neg_infinity(), self.float());
            // log(±0 ± i·0) = -∞ + i·arg(±0 ± i·0), where arg is the signed-zero atan2:
            // atan2(±0, +0) = ±0 (exact) and atan2(±0, −0) = ±π (inexact), both carrying the
            // imaginary part's sign — so `clog(-0 ± i·0) = -∞ + i·±π`.
            if re.is_neg_zero() {
                if self.precision() == 0 {
                    // π isn't exactly representable at unlimited precision — keep the historical
                    // `+0` imaginary part (the zero shortcut bypasses the Ziv precision check).
                    return Ok(exact(
                        neg_inf,
                        FBig::from_repr(Repr::zero_with_sign(Sign::Positive), self.float()),
                    ));
                }
                // `±π` rounded once under this mode. (Negating a rounded `+π` instead would
                // round `−π` in the opposite direction under `Down`/`Up`.)
                let pi = self.round_pi_multiple::<B>(
                    im.is_neg_zero(),
                    false,
                    reborrow_cache(&mut cache),
                )?;
                return Ok(combine_parts(Approximation::Exact(neg_inf), pi));
            }
            return Ok(exact(
                neg_inf,
                FBig::from_repr(Repr::zero_with_sign(im.sign()), self.float()),
            ));
        }
        if z.is_infinite() {
            // An infinite input is a terminal value and is not accepted (the convenience layer
            // panics).
            return Err(FpError::InfiniteInput);
        }
        // The `log(0)` shortcut above is exact and needs no precision; the rest is a
        // limited-precision technique.
        self.assert_limited();

        // The two parts are independent real functions of `z`, each rounded once.
        //
        // `arg z` is a single real kernel, correctly rounded by the float layer (which knows
        // the closed forms next to the axes — `atan2(y, 1) = y − …` for a tiny `y` — that a
        // composition could only certify at a working precision proportional to the exponent).
        let im = self
            .float()
            .atan2(z.im(), z.re(), reborrow_cache(&mut cache));
        let re = self.ln_abs(z, &mut cache);
        combine_results(re, im)
    }

    /// `ln|z|` for a finite nonzero `z`, correctly rounded.
    fn ln_abs<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: &mut Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if let Some(re) = self.ln_abs_near_one(z, cache) {
            return re;
        }
        // `ln(hypot(x, y))` through the ball composition: the float `hypot`/`ln` kernels run
        // on the midpoints and the modulus' rounding propagates through the exact inequality
        // `|Δln r| ≤ (hi−lo)/lo`.
        let p = self.precision();
        let [re] = self.ziv::<B, 1>(LOG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            let out =
                CBall::from_parts(z.re(), z.im(), pw).ln_abs(&gctx, pw, reborrow_cache(cache))?;
            Ok([out.to_value_radius(&gctx)])
        })?;
        Ok(re)
    }

    /// `ln|z| = ½·ln(1 + s)` with `s = x² + y² − 1`, for a modulus near 1 (`None` otherwise).
    ///
    /// Next to the unit circle `ln(hypot(x, y))` loses everything to the rounding of the
    /// modulus: `hypot(1, 2⁻ᵏ)` is `1` at any working precision below `2k` digits. Here `s` is
    /// instead formed exactly and rounded once, so it keeps its relative accuracy however
    /// small it is, and `ln_1p` does the rest.
    ///
    /// When `s` is exactly representable and tiny, `½·ln(1 + s) = s/2 − s²/4 + …` is the
    /// closed form `s/2` (representable in an even base) displaced downwards by an amount
    /// that the precision cannot show, and is rounded from that directly — no working
    /// precision short of the exponent of `s²` would certify which side of `s/2` it is on.
    fn ln_abs_near_one<const B: Word>(
        &self,
        z: &CBig<R, B>,
        cache: &mut Option<&mut ConstCache>,
    ) -> Option<FpResult<FBig<R, B>>> {
        let p = self.precision();
        let fctx = self.float();
        let (x, y) = (z.re(), z.im());
        // `|v| < B^top(v)`
        let top = |v: &Repr<B>| v.exponent() as i128 + v.digits_ub() as i128;
        if top(x) > 1 || top(y) > 1 {
            return None; // |z| >= B
        }
        let one = Repr::one();
        let terms = [
            (x, x, Sign::Positive),
            (y, y, Sign::Positive),
            (&one, &one, Sign::Negative),
        ];
        let s = match sum_of_products(&FloatCtxt::<R>::new(p + LOG_GUARD), &terms) {
            Ok(s) => s,
            // `s`, and with it `ln|z| ≈ s/2`, is below the exponent range
            Err(e) => return Some(Err(e)),
        };
        let exact = matches!(s, Approximation::Exact(_));
        let s = s.value().into_repr();
        if s.significand().is_zero() {
            // |z| = 1 exactly: ln 1 = +0 under every mode
            return Some(Ok(Approximation::Exact(FBig::from_repr(Repr::zero(), fctx))));
        }
        // `1 + s` has to stay away from zero: `s < 1`, and `s > −1/B` when negative
        let bound = if s.sign() == Sign::Negative { -1 } else { 0 };
        if top(&s) > bound {
            return None;
        }

        if exact && B % 2 == 0 && s.exponent() > isize::MIN + 2 {
            // v = s/2, and |ln(1 + s)/2 − v| <= s²/2 = 2v² < B^(2·top(v) + 1) has to be
            // invisible next to `v` padded to `p + 1` digits (see `round_sticky`)
            let v = Repr::<B>::new(s.significand() * IBig::from(B / 2), s.exponent() - 1);
            let digits = v.digits() as i128;
            let pad = (p as i128 + 1 - digits).max(0);
            if v.exponent() as i128 + 2 * digits + pad + 3 <= 0 {
                return Some(round_sticky(&fctx, &v, Sign::Negative));
            }
        }

        let part = self.ziv::<B, 1>(LOG_GUARD, |guard| {
            let pw = p + guard;
            let gctx = FloatCtxt::<R>::new(pw);
            // `s` rounded once from the exact sum: its flag is that of the exact value
            let s = kernel_ball(sum_of_products(&gctx, &terms)?, pw, |_| true);
            let ln = ln_1p_ball(&s, &gctx, pw, reborrow_cache(cache))?;
            let half = ln.div(&Ball::exact(Repr::new(IBig::from(2u8), 0)), pw)?;
            Ok([half.to_value_radius(&gctx)])
        });
        Some(part.map(|[re]| re))
    }
}

impl<R: ErrorBounds, const B: Word> CBig<R, B> {
    /// Complex natural logarithm (principal branch; convenience layer).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn ln(&self) -> Self {
        self.context().unwrap_cfp(self.context().log(self, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cbig::CBig as CBigGeneric;
    use dashu_base::{Abs, AbsOrd, Sign};
    use dashu_float::round::mode;
    use dashu_float::FBig;

    type C = CBig<mode::HalfAway, 10>;
    type F = FBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        CBig::from_parts(mk(re), mk(im))
    }

    fn within(a: &F, b: &F, k: u32) -> bool {
        if a == b {
            return true;
        }
        let diff = (a.clone() - b.clone()).abs();
        diff.abs_cmp(&(a.ulp() * F::from(k))).is_le()
    }

    #[test]
    fn ln_one_is_zero() {
        // Use a *limited*-precision input — `log` rejects unlimited precision (it would otherwise
        // silently compute at the fixed `LOG_GUARD`).
        assert!(c(1, 0).ln() == C::ZERO);
    }

    #[test]
    fn ln_exp_roundtrip() {
        // ln(exp z) ≈ z (the imaginary 1 sits inside ]-π, π], so no 2πi wrap)
        let z = c(1, 1);
        let l = z.exp().ln();
        let (zr, zi) = z.into_parts();
        let (lr, li) = l.into_parts();
        assert!(within(&zr, &lr, 16));
        assert!(within(&zi, &li, 16));
    }

    #[test]
    fn ln_zero_is_neg_infinity() {
        let l = C::ZERO.ln();
        assert!(l.re().is_infinite());
        assert_eq!(l.re().sign(), Sign::Negative);
    }

    // The mechanically tracked radius must certify at the target precision across the width
    // sweep: each result equals the same op computed at `p + 60` and re-rounded to `p` (both
    // sides are correctly rounded on the same exact integer input, so they agree bit for bit).
    #[test]
    fn log_matches_oracle_across_precisions() {
        // Both bases: `CBall::log`'s folds rest on the base-power brackets
        // (`Mag::from_repr_lower`, `log₂B`), which tighten below base 2's exact `B^e`.
        // Rungs: bit precisions at base 2, digit precisions at base 10, picked to span the
        // same significand widths (7 ≈ 20 bits, …) so both bases cover the same paths.
        macro_rules! sweep {
            ($base:literal, $name:literal, $precs:expr) => {{
                type C2 = CBigGeneric<mode::HalfEven, $base>;
                type F2 = FBig<mode::HalfEven, $base>;
                let inputs = [(1i64, 1i64), (3, 4), (-2, 1), (5, -12), (0, 1), (-7, 0)];
                for p in $precs {
                    for (re, im) in inputs {
                        let mk = |v: i64| F2::from(v).with_precision(p).value();
                        let mk_hi = |v: i64| F2::from(v).with_precision(p + 60).value();
                        let (hre, him) = C2::from_parts(mk_hi(re), mk_hi(im)).ln().into_parts();
                        let expect_re = hre.with_precision(p).value();
                        let expect_im = him.with_precision(p).value();
                        let got = C2::from_parts(mk(re), mk(im)).ln();
                        assert_eq!(got.re(), expect_re.repr(), "[$name] re p={p} z=({re},{im})");
                        assert_eq!(got.im(), expect_im.repr(), "[$name] im p={p} z=({re},{im})");
                    }
                }
            }};
        }
        sweep!(2, "base 2", [20, 50, 100, 500]);
        sweep!(10, "base 10", [7, 17, 34, 160]);
    }

    // log on an unlimited-precision CBig must panic, not silently compute at LOG_GUARD digits
    // (the `log(0)` / `log(∞)` shortcuts above are exact, so they don't hit this).
    #[test]
    #[should_panic(expected = "precision cannot be 0")]
    fn complex_log_unlimited_precision_panics() {
        let _ = C::ONE.ln();
    }

    #[test]
    fn log_signed_zero() {
        // Annex-G signed-zero cases: `clog(-0 ± i·0) = -∞ + i·±π`, and the exact `+0` imaginary
        // part carries the imaginary part's sign for a positive-real zero.
        let fctx = dashu_float::Context::<mode::HalfAway>::new(53);
        let (neg0, pos0) = (F::from_repr(Repr::neg_zero(), fctx), F::from_repr(Repr::zero(), fctx));
        let pi53 = F::pi(53);
        // clog(-0 + i·0) = -∞ + i·π
        let l = C::from_parts(neg0.clone(), pos0.clone()).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert_eq!(l.im(), pi53.repr());
        // clog(-0 − i·0) = -∞ − i·π
        let l = C::from_parts(neg0.clone(), neg0.clone()).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert_eq!(l.im(), (-pi53).repr());
        // clog(+0 − i·0) = -∞ − i·0
        let l = C::from_parts(pos0.clone(), neg0.clone()).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert!(l.im().is_neg_zero());
        // clog(+0 + i·0) = -∞ + i·0
        let l = C::from_parts(pos0.clone(), pos0).ln();
        assert!(l.re().is_infinite() && l.re().sign() == Sign::Negative);
        assert!(l.im().is_pos_zero());
    }
}
