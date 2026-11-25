//! The complex ball — a pair of real [`Ball`]s (one per component) with complex error
//! propagation. This is the value space the transcendental closures compute in, replacing the
//! hand-written `ulp()·k` radii: every rounding of the composition is tracked mechanically, and
//! an exact chain keeps `rad == 0` on both components, which is how exactly-representable
//! results (and their exact-zero components) stay certifiable under the directed rounding modes.
//!
//! The real halves are float's `Ball`/`Mag` (shared in lockstep through float's `#[doc(hidden)]`
//! re-exports), so the per-component propagation rules are float's own; only the *complex* rules
//! live here — the product rule (4 tracked products + add/sub), the quotient through `‖z‖²`,
//! and the input-error folds for the float kernels, which run on the midpoints and get their own
//! contribution from the kernel's rounding via [`kernel_ball`] (exact → radius 0, inexact →
//! one work-ulp; the public kernels are correctly rounded, so one ulp covers them with a 2×
//! margin). A kernel's `Exact` flag is only believed where the result *can* be exact — see
//! [`kernel_ball`].
//!
//! Soundness of every fold below rests on the same two facts as float's ball: a [`Mag`] is a
//! rigorous upper bound by construction (all its ops round up), and the folds are stated as
//! exact inequalities — `ln(1+t) ≤ t`, `√(a+b) ≤ √a+√b`, `√t ≤ 1+t`, `‖∇arg‖ = 1/‖z‖` — so no
//! fudge constants appear anywhere.

use dashu_base::{Approximation, Sign};
use dashu_float::round::Rounding;
use dashu_float::{ulp_mag, Ball, ConstCache, Context as FloatCtxt, FBig, FpError, Mag, Repr};
use dashu_int::{IBig, Word};

use crate::repr::reborrow_cache;
use crate::round::{mode, ErrorBounds, Round};

/// A complex ball: midpoints plus radii per component, `|component − true| ≤ rad`.
#[derive(Clone, Debug)]
pub(crate) struct CBall<const B: Word> {
    pub(crate) re: Ball<B>,
    pub(crate) im: Ball<B>,
}

impl<const B: Word> CBall<B> {
    // ========================================================================
    // Seeds
    // ========================================================================

    /// An exact complex value (both radii zero) — the entry point for a Ziv closure's input.
    pub(crate) fn exact(re: Repr<B>, im: Repr<B>) -> Self {
        Self {
            re: Ball::exact(re),
            im: Ball::exact(im),
        }
    }

    /// Seed from an input's parts, *exactly* (both radii zero), whatever the working precision.
    ///
    /// The parts are deliberately **not** rounded to the working precision, even when they
    /// carry more digits than it: every tracked operation rounds its own result, and the float
    /// kernels accept an operand of any width. Rounding the seed instead would make the input
    /// itself inexact, which (a) hides the exactly-representable results the directed modes
    /// can only certify through a zero radius (`√(w²) = w` for a wide `w`), (b) moves an input
    /// next to a branch point *onto* it (`asin(1 + 2⁻³⁰)` at 10 digits), and (c) is amplified
    /// without bound by the steep kernels — `exp` of a rounded `1.2e15` carries an input
    /// radius of ~2³⁵, i.e. a result radius astronomically larger than the result.
    pub(crate) fn from_parts(re: &Repr<B>, im: &Repr<B>, _prec: usize) -> Self {
        Self::exact(re.clone(), im.clone())
    }

    // ========================================================================
    // Exact operations — no prec, no error
    // ========================================================================

    /// Multiply by `±i` — an exact rotation, mirroring `CBig::mul_i`: `(x, y)·i = (−y, x)`,
    /// `(x, y)·(−i) = (y, −x)`. Radii swap unchanged.
    pub(crate) fn mul_i(&self, negative: bool) -> Self {
        if negative {
            Self {
                re: self.im.clone(),
                im: -self.re.clone(),
            }
        } else {
            Self {
                re: -self.im.clone(),
                im: self.re.clone(),
            }
        }
    }

    /// `−self` (exact: both midpoints change sign, the radii are unchanged).
    pub(crate) fn neg(&self) -> Self {
        Self {
            re: -self.re.clone(),
            im: -self.im.clone(),
        }
    }

    /// The complex conjugate (exact).
    pub(crate) fn conj(&self) -> Self {
        Self {
            re: self.re.clone(),
            im: -self.im.clone(),
        }
    }

    // ========================================================================
    // Propagation — component rules over the real balls
    // ========================================================================

    /// Componentwise `a + b` (the complex addition is real addition per component).
    pub(crate) fn add(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        Ok(Self {
            re: self.re.add(&rhs.re, prec)?,
            im: self.im.add(&rhs.im, prec)?,
        })
    }

    /// Componentwise `a − b`.
    pub(crate) fn sub(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        Ok(Self {
            re: self.re.sub(&rhs.re, prec)?,
            im: self.im.sub(&rhs.im, prec)?,
        })
    }

    /// `(a+bi)(c+di) = (ac−bd) + (ad+bc)i` through four tracked products. The exact cross-term
    /// algebra (`|ac − âĉ| ≤ |â|rad_c + |ĉ|rad_a + rad_a·rad_c`, identically for the other
    /// three) is precisely `Ball::mul`'s rule, so each component's radius is the sum of its two
    /// product radii plus the final add/sub rounding — nothing is dropped, second-order terms
    /// included.
    pub(crate) fn mul(&self, rhs: &Self, prec: usize) -> Result<Self, FpError> {
        let ac = self.re.mul(&rhs.re, prec)?;
        let bd = self.im.mul(&rhs.im, prec)?;
        let ad = self.re.mul(&rhs.im, prec)?;
        let bc = self.im.mul(&rhs.re, prec)?;
        Ok(Self {
            re: ac.sub(&bd, prec)?,
            im: ad.add(&bc, prec)?,
        })
    }

    /// `self² = (a²−b²) + (2ab)i` — the square kernels are cheaper than a general product, and
    /// the doubling `ab + ab` rounds exactly (same significand, exponent+1), so the imaginary
    /// radius costs a single ε.
    pub(crate) fn sqr(&self, prec: usize) -> Result<Self, FpError> {
        let a2 = self.re.sqr(prec)?;
        let b2 = self.im.sqr(prec)?;
        let ab = self.re.mul(&self.im, prec)?;
        Ok(Self {
            re: a2.sub(&b2, prec)?,
            im: ab.add(&ab, prec)?,
        })
    }

    /// The squared modulus `a² + b²` — a real ball (the division denominators).
    pub(crate) fn norm_sqr(&self, prec: usize) -> Result<Ball<B>, FpError> {
        self.re.sqr(prec)?.add(&self.im.sqr(prec)?, prec)
    }

    /// `1/self = conj(self)/‖self‖²` — the only complex division the transcendentals need.
    /// A denominator ball touching zero yields an infinite radius (a Ziv retry, soundly); an
    /// exact zero denominator fails the midpoint division first.
    pub(crate) fn inv(&self, prec: usize) -> Result<Self, FpError> {
        let d = self.norm_sqr(prec)?;
        Ok(Self {
            re: self.re.div(&d, prec)?,
            im: -self.im.clone().div(&d, prec)?,
        })
    }

    /// Componentwise division by a *real* ball: `(u/d, v/d)`. Serves `tan`'s shared denominator
    /// `cos(2x) + cosh(2y)` and `atan`'s exact halving.
    pub(crate) fn div_by_real(&self, d: &Ball<B>, prec: usize) -> Result<Self, FpError> {
        Ok(Self {
            re: div_real(&self.re, d, prec)?,
            im: div_real(&self.im, d, prec)?,
        })
    }

    // ========================================================================
    // Transcendental compositions (float kernels on the midpoints + input folds)
    // ========================================================================

    /// `exp(self) = eˣ(cos y + i·sin y)`: the float `exp`/`sin_cos` kernels evaluate on the
    /// midpoints (each certified to ≤ 1 work-ulp by its own Ziv loop, which `from_rounded`
    /// folds), then the input radii propagate:
    /// * real part `|e^{x+θ} − eˣ| ≤ rad_x·e^{x+rad_x}` — float's own fold
    ///   (`‖result‖·exp_upper(rad_x)·rad_x`, both factors essential — see float's `exp_ball`);
    /// * imaginary part `|Δsin|, |Δcos| ≤ |θ| = rad_y` (the derivatives are bounded by 1),
    ///   which the two product rules then carry into both components.
    pub(crate) fn exp<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Self, FpError> {
        // e^x is exact only at x = 0, sin/cos only at 0
        let mut ex = kernel_ball(fctx.exp(&self.re.mid, reborrow_cache(&mut cache))?, prec, |_| {
            is_zero(&self.re.mid)
        });
        if !self.re.rad.is_zero() {
            // float's exp fold, verbatim (the magnitude before the error, never clamped)
            let factor = ex.mag().mul(&self.re.rad.exp_upper());
            ex.add_error(factor.mul(&self.re.rad));
        }
        let (sin_y, cos_y) = fctx.sin_cos(&self.im.mid, reborrow_cache(&mut cache));
        let mut sy = kernel_ball(sin_y?, prec, |_| is_zero(&self.im.mid));
        let mut cy = kernel_ball(cos_y?, prec, |_| is_zero(&self.im.mid));
        if !self.im.rad.is_zero() {
            sy.add_error(self.im.rad);
            cy.add_error(self.im.rad);
        }
        Ok(Self {
            re: ex.mul(&cy, prec)?,
            im: ex.mul(&sy, prec)?,
        })
    }

    /// `‖self‖` as a ball, with a lower bound of the true modulus (which drives the folds of
    /// [`log`](Self::log)).
    fn modulus<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
    ) -> Result<(Ball<B>, Mag), FpError> {
        // ‖z‖ with the hypot fold (the Euclidean norm is 1-Lipschitz: |Δ‖z‖| ≤ |δx| + |δy|)
        let mut r = kernel_ball(fctx.hypot(&self.re.mid, &self.im.mid)?, prec, |mid| {
            hypot_is_exact(mid, &self.re.mid, &self.im.mid, prec)
        });
        r.add_error(self.re.rad.add(&self.im.rad));

        // A lower bound of the true ‖z‖ (drives both folds). The *bracket width* must not be
        // taken from `Mag::from_repr`/`from_repr_lower`: at base ≠ 2 the `B^e` scaling enters
        // the Mag only through the fixed-point `log₂B` bracket, so the two bounds of an exact
        // mid can sit a factor ~2 apart — a value-space `hi − lo` there would inflate the ln
        // fold to O(1) (retrying forever). The `rad` side is a true Mag (1-word tight), so the
        // fold keeps the bracket on `rad` alone:
        // |Δln| ≤ ln((m+rad)/(m−rad)) ≤ 2·rad/(m−rad), with `lo` a lower bound of `m−rad`.
        let lo = Mag::from_repr_lower(&r.mid).sub_down(&r.rad);
        Ok((r, lo))
    }

    /// `ln r` for the modulus ball `r` with the lower bound `lo` (see
    /// [`modulus`](Self::modulus)).
    fn ln_of_modulus<R: ErrorBounds>(
        r: &Ball<B>,
        lo: &Mag,
        fctx: &FloatCtxt<R>,
        prec: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Ball<B>, FpError> {
        let ln_fold = if lo.is_zero() {
            Mag::INFINITY
        } else {
            r.rad.mul2().div(lo)
        };
        // ln r is exact only at r = 1
        let mut ln_r = kernel_ball(fctx.ln(&r.mid, reborrow_cache(&mut cache))?, prec, |_| {
            r.mid == Repr::one()
        });
        ln_r.add_error(ln_fold);
        Ok(ln_r)
    }

    /// `ln‖self‖`, the real part of [`log`](Self::log), for an exact entry.
    pub(crate) fn ln_abs<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
        cache: Option<&mut ConstCache>,
    ) -> Result<Ball<B>, FpError> {
        let (r, lo) = self.modulus(fctx, prec)?;
        Self::ln_of_modulus(&r, &lo, fctx, prec, cache)
    }

    /// `log(self) = (ln‖self‖, arg self)`: the float `hypot`/`ln`/`atan2` kernels evaluate on
    /// the midpoints; the input radii propagate through the exact inequalities
    /// `|Δln r| ≤ ln(hi/lo) ≤ (hi−lo)/lo` (monotonicity + `ln(1+t) ≤ t`, `[lo, hi]` the input
    /// ball's bracket of the true `‖z‖`) and `|Δarg| ≤ ‖δz‖/lo` (`‖∇arg‖ = 1/‖z‖`, mean value
    /// theorem along the input segment). Near `‖z‖ = 1` this reproduces the old hand-written
    /// `B^{1−pw}` amplification term at the same order; away from it the fold is far tighter
    /// because it is no longer paid unconditionally.
    pub(crate) fn log<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
        mut cache: Option<&mut ConstCache>,
    ) -> Result<Self, FpError> {
        // Both midpoints cancelled onto zero while the ball still has a radius (the true value
        // is some unknown nonzero point near the origin): neither `ln` nor `atan2` has a value
        // at the origin, so export the whole plane — a Ziv retry at a higher guard, where the
        // midpoint re-emerges from its addends' rounding noise. (An *exact* zero is the genuine
        // `log 0`, left to the kernels.)
        if self.re.mid.significand().is_zero()
            && self.im.mid.significand().is_zero()
            && !self.re.mid.is_infinite()
            && !self.im.mid.is_infinite()
            && !(self.re.rad.is_zero() && self.im.rad.is_zero())
        {
            return Ok(Self {
                re: Ball::with_error(Repr::zero(), Mag::INFINITY),
                im: Ball::with_error(Repr::zero(), Mag::INFINITY),
            });
        }

        let (r, lo) = self.modulus(fctx, prec)?;
        let ln_r = Self::ln_of_modulus(&r, &lo, fctx, prec, reborrow_cache(&mut cache))?;

        // The argument fold is the *componentwise* gradient bound, not the joint 1-Lipschitz
        // `(rad_x + rad_y)/lo`: `∇arg = (−y, x)/‖z‖²`, so along the input segment
        // `|Δarg| ≤ (|y|·rad_x + |x|·rad_y)/lo²` (`lo²` a proven lower bound of `‖z‖²` —
        // rounded *down*, a round-up denominator would under-price the fold). On the axes this
        // vanishes exactly (`y ≡ 0` ⇒ the angle is invariant along the real error direction),
        // so `log` of a real/imaginary ball carries a *zero* argument radius — the structural
        // zero the strict zero-candidate certification needs (e.g. `asin(±i)`'s real part).
        // Away from the axes it is also tighter than the joint bound by the component ratio.
        let arg_fold = if lo.is_zero() {
            Mag::INFINITY
        } else {
            let lo2 = lo.mul_down(&lo);
            self.im
                .mag()
                .mul(&self.re.rad)
                .add(&self.re.mag().mul(&self.im.rad))
                .div(&lo2)
        };

        // the angle is exact only on the positive real axis (0); every other axis value is a
        // multiple of π/2
        let mut arg = kernel_ball(
            fctx.atan2(&self.im.mid, &self.re.mid, reborrow_cache(&mut cache))?,
            prec,
            |_| {
                is_zero(&self.im.mid)
                    && !is_zero(&self.re.mid)
                    && self.re.mid.sign() == Sign::Positive
            },
        );
        arg.add_error(arg_fold);

        Ok(Self { re: ln_r, im: arg })
    }

    /// Principal square root through the cancellation-free form (for `x ≥ 0`:
    /// `a = √((r+x)/2)`, `b = y/(2a)`; `x < 0` mirrored, `b` carrying the sign of `y`) — the
    /// same recipe as the hand-radius version, but every add/sub/div/mul/sqrt is a tracked real
    /// ball op and only `hypot` is a kernel. The real `sqrt` fold is float's own
    /// `rad_arg/(2·LB(|mid|))`; when the argument ball touches zero the bound
    /// `√A ≤ √M + √rad ≤ mid + ε + (1 + rad)` (subadditivity, then `√t ≤ 1+t`) keeps a *finite*
    /// radius that shrinks as the guard grows, so a straddling argument is a Ziv retry, not a
    /// dead end.
    pub(crate) fn sqrt<R: ErrorBounds>(
        &self,
        fctx: &FloatCtxt<R>,
        prec: usize,
    ) -> Result<Self, FpError> {
        // an exact (0, 0) ball roots to exactly 0 — short-circuit before the cancellation-free
        // form divides 0/(2·0) (asin/acosh at z = ±1 reach exactly this interior zero). The
        // infinite check matters: an infinite midpoint's significand is zero too.
        if !self.re.mid.is_infinite()
            && !self.im.mid.is_infinite()
            && self.re.mid.significand().is_zero()
            && self.im.mid.significand().is_zero()
            && self.re.rad.is_zero()
            && self.im.rad.is_zero()
        {
            return Ok(Self::exact(Repr::zero(), Repr::zero()));
        }
        let mut r = kernel_ball(fctx.hypot(&self.re.mid, &self.im.mid)?, prec, |mid| {
            hypot_is_exact(mid, &self.re.mid, &self.im.mid, prec)
        });
        r.add_error(self.re.rad.add(&self.im.rad));

        let two = Ball::exact(Repr::<B>::new(IBig::from(2u8), 0));
        if self.re.mid.sign() != Sign::Negative {
            let a = sqrt_real(r.add(&self.re, prec)?.div(&two, prec)?, fctx, prec)?;
            let b = div_real(&self.im, &a.mul(&two, prec)?, prec)?;
            Ok(Self { re: a, im: b })
        } else {
            // r − x = r + |x| — no cancellation
            let b0 = sqrt_real(r.sub(&self.re, prec)?.div(&two, prec)?, fctx, prec)?;
            let b = if self.im.mid.sign() == Sign::Negative {
                -b0.clone()
            } else {
                b0.clone()
            };
            // a = |y|/(2|b|) ≥ 0: the real part of the principal root is never negative, also
            // as a zero (`√(−x − 0i) = +0 − i√x`, the conjugate of `√(−x + 0i)`)
            let a = div_real(&abs_ball(&self.im), &b0.mul(&two, prec)?, prec)?;
            Ok(Self { re: a, im: b })
        }
    }

    // ========================================================================
    // Ziv boundary
    // ========================================================================

    /// Export both components as `(value, radius)` pairs — the driver's closure contract.
    pub(crate) fn to_parts_radius<R: Round>(
        &self,
        ctx: &FloatCtxt<R>,
    ) -> [(FBig<R, B>, FBig<R, B>); 2] {
        [self.re.to_value_radius(ctx), self.im.to_value_radius(ctx)]
    }
}

/// Wrap the result of a `dashu-float` kernel, evaluated at working precision `prec`, as a ball:
/// an exact result keeps a zero radius, a rounded one gets one work-ulp (the kernels are
/// correctly rounded, so one ulp covers the rounding with a 2× margin).
///
/// The zero radius is what certifies exactly-representable results, so it must only be granted
/// when the result really is exact. The kernel's own `Exact` flag is not sufficient evidence:
/// a kernel may report `Exact` when its *internal* approximation happened to land on the
/// returned value while the true value is merely within its error bound (`hypot(1, 2⁴⁰)`,
/// `cosh(2⁻⁴³)`, …) — and a zero radius on a rounded value makes every bound derived from it
/// unsound (a `ln|z|` certified with 20 good bits, an exact `0/0` where the denominator was
/// really `1 + 2⁻⁸⁷ − 1`). So the flag is only believed when `can_be_exact` — a cheap
/// *mathematical* criterion supplied by the call site (the argument is the function's only
/// rational point, or the claimed value verifies exactly) — agrees. It is consulted only when
/// the kernel claims exactness.
pub(crate) fn kernel_ball<R: Round, const B: Word>(
    rounded: Approximation<FBig<R, B>, Rounding>,
    prec: usize,
    can_be_exact: impl FnOnce(&Repr<B>) -> bool,
) -> Ball<B> {
    match rounded {
        // (an infinite result is a special value of the kernel, e.g. ln 0, not a rounding)
        Approximation::Exact(v) if v.repr().is_infinite() || can_be_exact(v.repr()) => {
            Ball::exact(v.into_repr())
        }
        Approximation::Exact(v) | Approximation::Inexact(v, _) => {
            let mid = v.into_repr();
            let rad = ulp_mag::<B>(&mid, prec);
            Ball::with_error(mid, rad)
        }
    }
}

/// `true` ⟺ `x` is a (signed) zero.
pub(crate) fn is_zero<const B: Word>(x: &Repr<B>) -> bool {
    !x.is_infinite() && x.significand().is_zero()
}

/// An exact product (unlimited precision); `None` on a range error.
fn exact_mul<const B: Word>(a: &Repr<B>, b: &Repr<B>) -> Option<Repr<B>> {
    FloatCtxt::<mode::Zero>::new(0)
        .mul(a, b)
        .ok()
        .map(|r| r.value().into_repr())
}

/// `true` ⟺ `mid² = arg` exactly (the test for an exact real square root).
fn square_is<const B: Word>(mid: &Repr<B>, arg: &Repr<B>) -> bool {
    exact_mul(mid, mid).map_or(false, |sq| &sq == arg)
}

/// `true` ⟺ `mid² = a² + b²` exactly (the test for an exact `hypot`).
fn hypot_is_exact<const B: Word>(mid: &Repr<B>, a: &Repr<B>, b: &Repr<B>, prec: usize) -> bool {
    if is_zero(a) || is_zero(b) {
        // hypot(x, 0) = |x| is a plain rounding of the operand, whose flag is reliable
        return true;
    }
    // The exact sum aligns the two squares, so bound its width before building it. When the
    // squares are this far apart their digit ranges do not overlap, the sum spans (to within a
    // digit or two) the whole distance, and that is more than the `2·prec` digits `mid²` can
    // have — so the result cannot be exact.
    let top = |x: &Repr<B>| 2 * (x.exponent() as i128 + x.digits_ub() as i128);
    let bottom = |x: &Repr<B>| 2 * x.exponent() as i128;
    let span = top(a).max(top(b)) - bottom(a).min(bottom(b));
    let limit = 8 * (prec as i128 + a.digits_ub() as i128 + b.digits_ub() as i128) + 64;
    if span > limit {
        return false;
    }
    let (Some(m2), Some(a2), Some(b2)) = (exact_mul(mid, mid), exact_mul(a, a), exact_mul(b, b))
    else {
        return false;
    };
    FloatCtxt::<mode::Zero>::new(0)
        .add(&a2, &b2)
        .map_or(false, |sum| sum.value().into_repr() == m2)
}

/// `true` ⟺ `6·x` is an integer — a superset of the arguments at which `sin(πx)` or `cos(πx)`
/// is rational (Niven: the only rational values are `0`, `±1/2`, `±1`), i.e. of the arguments
/// at which the ×π kernels can be exact.
pub(crate) fn six_x_is_integer<const B: Word>(x: &Repr<B>) -> bool {
    if x.is_infinite() {
        return false;
    }
    exact_mul(x, &Repr::new(IBig::from(6u8), 0)).map_or(false, |t| is_zero(&t) || t.exponent() >= 0)
}

/// `true` ⟺ the claimed `mid` equals `base^exp` exactly (the test for an exact real integer
/// power; `exp ≠ 0`). The exact power is only built when it is small enough to possibly fit
/// `prec` digits — in an exotic base a reciprocal `1/sⁿ` can be far shorter than `sⁿ`, hence
/// the generous factor; beyond it the answer is a conservative `false`, which a later, wider
/// attempt revisits.
pub(crate) fn powi_is_exact<const B: Word>(
    mid: &Repr<B>,
    base: &Repr<B>,
    exp: &IBig,
    prec: usize,
) -> bool {
    use dashu_base::Signed;
    if base.is_infinite() || mid.is_infinite() || is_zero(base) {
        return false;
    }
    let n = match usize::try_from(exp.clone().into_parts().1) {
        Ok(n) => n,
        Err(_) => return false,
    };
    if n.saturating_mul(base.digits_ub()) > prec.saturating_mul(128).saturating_add(64) {
        // (a power of the base itself is exact at any size, and is a bare exponent)
        return base.significand().is_one() || (-base.significand()).is_one();
    }
    // base^n exactly: significand power and exponent multiple
    let Some(e) = (n as i128).checked_mul(base.exponent() as i128) else {
        return false;
    };
    let Ok(e) = isize::try_from(e) else {
        return false;
    };
    let power = Repr::<B>::new(base.significand().pow(n), e);
    if exp.is_positive() {
        mid == &power
    } else {
        // mid = 1/base^n ⟺ mid · base^n = 1
        exact_mul(mid, &power).map_or(false, |one| one == Repr::one())
    }
}

/// `|b|` as a ball: the midpoint's magnitude with the radius unchanged (`||x| − |y|| ≤ |x − y|`).
/// A negative zero becomes `+0`.
pub(crate) fn abs_ball<const B: Word>(b: &Ball<B>) -> Ball<B> {
    if b.mid.sign() == Sign::Negative {
        -b.clone()
    } else {
        b.clone()
    }
}

/// `num / d` by a real ball. A precisely-zero numerator divides to a precisely-zero quotient —
/// unless the denominator is exactly zero too: that is the genuine 0/0 (e.g. `tan_pi` at a
/// real-axis pole), and it falls to the kernel division, which reports `Indeterminate`. A
/// *nonzero* numerator over a zero-or-straddling denominator has an unbounded quotient: the
/// whole-line ball (`Mag::INFINITY` radius) is exported, which no Ziv attempt can certify — the
/// loop retries at a higher guard, where the strictly positive true denominator (`D = cos 2x +
/// cosh 2y > 0` for `y ≠ 0`, `2·root > 0` in `sqrt`) re-emerges above its addends' rounding
/// noise. The kernel division must not run in that state: a zero-mid denominator surfaces as
/// the infinity that panics the part arithmetic.
pub(crate) fn div_real<const B: Word>(
    num: &Ball<B>,
    d: &Ball<B>,
    prec: usize,
) -> Result<Ball<B>, FpError> {
    let d_exact_zero = d.rad.is_zero() && d.mid.significand().is_zero();
    if num.rad.is_zero() && num.mid.significand().is_zero() {
        if d_exact_zero {
            return num.div(d, prec); // genuine 0/0 → kernel `Indeterminate`
        }
        return Ok(Ball::exact(num.mid.clone())); // 0 over anything else is exactly 0
    }
    if d_exact_zero || Mag::from_repr_lower(&d.mid).sub_down(&d.rad).is_zero() {
        // exactly-zero denominator (nonzero numerator) or touching/straddling zero:
        // unbounded quotient → whole-line retry signal
        return Ok(Ball::with_error(Repr::<B>::zero(), Mag::INFINITY));
    }
    num.div(d, prec)
}

/// The tracked real `√`: the float kernel on the midpoint, then `rad_arg/(2·LB(|mid|))` —
/// float's documented `Ball::sqrt` fold — with the straddle fallback for a zero-touching
/// argument (see [`CBall::sqrt`]).
fn sqrt_real<R: ErrorBounds, const B: Word>(
    arg: Ball<B>,
    fctx: &FloatCtxt<R>,
    prec: usize,
) -> Result<Ball<B>, FpError> {
    // float's `finish_mid` rule: the fresh ε joins only when the kernel rounded inexact — an
    // exact root keeps `rad = 0` so the exact chain (e.g. √4) stays certifiable
    let root = kernel_ball(fctx.sqrt(&arg.mid)?, prec, |mid| square_is(mid, &arg.mid));
    let (mid, eps) = (root.mid, root.rad);
    if mid.significand().is_zero() {
        // √(exact 0): the kernel returned an exact zero — keep the chain exact (directed modes
        // can only certify a zero component through a zero radius). A zero midpoint that still
        // carries a radius is a different matter: the true argument is some unknown value within
        // `rad` of zero, its root is within `√rad`, and no multiple of the *input* radius bounds
        // that (the derivative at the origin is infinite). Export the whole line instead — a
        // Ziv retry at a higher guard, where the argument stops rounding onto zero.
        return Ok(if arg.rad.is_zero() {
            Ball::exact(mid)
        } else {
            Ball::with_error(mid, Mag::INFINITY)
        });
    }
    let lo = Mag::from_repr_lower(&arg.mid).sub_down(&arg.rad);
    let mut rad = if lo.is_zero() {
        Mag::ONE.add(&Mag::from_repr(&mid)).add(&arg.rad)
    } else {
        arg.rad.div(&Mag::from_repr_lower(&mid).mul2())
    };
    rad = rad.add(&eps);
    Ok(Ball { mid, rad })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use core::cmp::Ordering;

    type F = FBig<mode::HalfEven, 2>;

    fn repr(sig: i64, exp: isize) -> Repr<2> {
        Repr::new(IBig::from(sig), exp)
    }

    fn cb(re: Repr<2>, im: Repr<2>) -> CBall<2> {
        CBall::exact(re, im)
    }

    /// Assert `[mid − rad, mid + rad]` covers the exact `true_v`, decided on lossless
    /// unlimited-precision sums (the radius enters as its exported exact `Repr`).
    fn covers(ball: &Ball<2>, true_v: &F) -> bool {
        let x = FloatCtxt::<mode::HalfEven>::new(0);
        let (v, r) = ball.to_value_radius(&x);
        let hi = x.add(v.repr(), r.repr()).unwrap().value();
        let lo = x.sub(v.repr(), r.repr()).unwrap().value();
        hi.cmp(true_v) != Ordering::Less && lo.cmp(true_v) != Ordering::Greater
    }

    fn covers2(out: &CBall<2>, true_re: &F, true_im: &F) -> bool {
        covers(&out.re, true_re) && covers(&out.im, true_im)
    }

    #[test]
    fn mul_exact_chain() {
        let p = 10;
        let a = cb(repr(3, 0), repr(4, 0));
        let b = cb(repr(1, 0), repr(-2, 0));
        let prod = a.mul(&b, p).unwrap();
        // (3+4i)(1−2i) = 11 − 2i, exactly representable → exact chain
        assert!(prod.re.rad.is_zero() && prod.im.rad.is_zero());
        assert_eq!(prod.re.mid, repr(11, 0));
        assert_eq!(prod.im.mid, repr(-2, 0));
    }

    #[test]
    fn sqr_exact_and_covers() {
        let p = 10;
        let z = cb(repr(3, 0), repr(4, 0));
        let sq = z.sqr(p).unwrap();
        // (3+4i)² = −7 + 24i exactly
        assert!(sq.re.rad.is_zero() && sq.im.rad.is_zero());
        assert_eq!(sq.re.mid, repr(-7, 0));
        assert_eq!(sq.im.mid, repr(24, 0));
        // and it agrees with the general product rule on an inexact input (coverage)
        let one = cb(repr(1, 0), Repr::<2>::zero());
        let via_mul = z.mul(&one, p).unwrap().mul(&z, p).unwrap();
        let true_re = F::from(-7).with_precision(50).value();
        let true_im = F::from(24).with_precision(50).value();
        assert!(covers2(&sq, &true_re, &true_im));
        assert!(covers2(&via_mul, &true_re, &true_im));
    }

    #[test]
    fn mul_i_rotations_exact() {
        let z = cb(repr(3, 0), repr(4, 0));
        // (3+4i)·i = −4 + 3i
        let t = z.mul_i(false);
        assert!(t.re.rad.is_zero() && t.im.rad.is_zero());
        assert_eq!(t.re.mid, repr(-4, 0));
        assert_eq!(t.im.mid, repr(3, 0));
        // (3+4i)·(−i) = 4 − 3i
        let t = z.mul_i(true);
        assert!(t.re.rad.is_zero() && t.im.rad.is_zero());
        assert_eq!(t.re.mid, repr(4, 0));
        assert_eq!(t.im.mid, repr(-3, 0));
    }

    #[test]
    fn inv_covers_reciprocal() {
        let p = 20;
        let z = cb(repr(3, 0), repr(4, 0));
        let inv = z.inv(p).unwrap();
        // (3+4i)⁻¹ = (3 − 4i)/25, an exact rational — compare 25·(mid ± rad) against the exact
        // integer numerators on raw Reprs (lossless)
        let x = FloatCtxt::<mode::HalfEven>::new(0);
        let twenty5 = repr(25, 0);
        let (vre, rre) = inv.re.to_value_radius(&x);
        let (vim, rim) = inv.im.to_value_radius(&x);
        let v25 = vre.repr() * &twenty5;
        let r25 = rre.repr() * &twenty5;
        let w25 = vim.repr() * &twenty5;
        let s25 = rim.repr() * &twenty5;
        let lo_re = &v25 - &r25;
        let hi_re = &v25 + &r25;
        let lo_im = &w25 - &s25;
        let hi_im = &w25 + &s25;
        assert!(lo_re.cmp(&repr(3, 0)) != Ordering::Greater);
        assert!(hi_re.cmp(&repr(3, 0)) != Ordering::Less);
        assert!(lo_im.cmp(&repr(-4, 0)) != Ordering::Greater);
        assert!(hi_im.cmp(&repr(-4, 0)) != Ordering::Less);
    }

    #[test]
    fn sqrt_exact_chains() {
        let p = 10;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        // √(4) = 2: both components exact (directed modes certify only through rad == 0)
        let out = cb(repr(4, 0), Repr::<2>::zero()).sqrt(&fctx, p).unwrap();
        assert!(out.re.rad.is_zero() && out.im.rad.is_zero());
        assert_eq!(out.re.mid, repr(2, 0));
        assert!(out.im.mid.significand().is_zero());
        // √(−4) = 2i: the real component collapses to exact zero with a zero radius — the
        // containment guard's load-bearing case
        let out = cb(repr(-4, 0), Repr::<2>::zero()).sqrt(&fctx, p).unwrap();
        assert!(out.re.rad.is_zero() && out.im.rad.is_zero());
        assert!(out.re.mid.significand().is_zero());
        assert_eq!(out.im.mid, repr(2, 0));
    }

    #[test]
    fn sqrt_covers_exact_result() {
        let p = 20;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        // √(3+4i) = 2 + i exactly
        let out = cb(repr(3, 0), repr(4, 0)).sqrt(&fctx, p).unwrap();
        let two = F::from(2).with_precision(50).value();
        let one = F::from(1).with_precision(50).value();
        assert!(covers2(&out, &two, &one));
    }

    #[test]
    fn sqrt_straddling_argument_stays_finite() {
        let p = 30;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        // a real ball touching zero: the straddle fold keeps a finite radius covering 0
        let mut z = cb(repr(1, -10), Repr::<2>::zero());
        z.re = Ball::with_error(repr(1, -10), Mag::from_repr(&repr(1, -9)));
        let out = z.sqrt(&fctx, p).unwrap();
        assert!(!out.re.rad.is_infinite());
        assert!(covers(&out.re, &F::ZERO.with_precision(0).value()));
        assert!(covers(&out.im, &F::ZERO.with_precision(0).value()));
    }

    #[test]
    fn exp_matches_high_precision_oracle() {
        let p = 20;
        let z = cb(repr(3, -2), repr(5, -2)); // 0.75 + 1.25i
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let hi = FloatCtxt::<mode::HalfEven>::new(p + 64);
        let out = z.exp(&fctx, p, None).unwrap();
        let ex = hi.exp(&z.re.mid, None).unwrap().value();
        let (sy, cy) = hi.sin_cos(&z.im.mid, None);
        let true_re = &ex * &cy.unwrap().value();
        let true_im = &ex * &sy.unwrap().value();
        assert!(covers2(&out, &true_re, &true_im));
    }

    #[test]
    fn log_matches_high_precision_oracle() {
        let p = 20;
        let z = cb(repr(3, 0), repr(4, 0)); // ln 5, atan(4/3)
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let hi = FloatCtxt::<mode::HalfEven>::new(p + 64);
        let out = z.log(&fctx, p, None).unwrap();
        let five = hi.hypot(&repr(3, 0), &repr(4, 0)).unwrap().value();
        let true_re = hi.ln(five.repr(), None).unwrap().value();
        let true_im = hi.atan2(&repr(4, 0), &repr(3, 0), None).unwrap().value();
        assert!(covers2(&out, &true_re, &true_im));
    }

    #[test]
    fn exp_input_error_fold_keeps_coverage() {
        // a nonzero input radius must not shrink the certified interval: compare against the
        // same composition from an exact input — the fold can only widen the radius
        let p = 20;
        let z = cb(repr(3, -2), repr(5, -2));
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let mut z_err = z.clone();
        z_err.re = Ball::with_error(z.re.mid.clone(), Mag::from_repr(&repr(1, -30)));
        z_err.im = Ball::with_error(z.im.mid.clone(), Mag::from_repr(&repr(1, -30)));
        let out = z.exp(&fctx, p, None).unwrap();
        let out_err = z_err.exp(&fctx, p, None).unwrap();
        // the perturbed input's export must cover the exact-input result values
        let (re_true, _) = out.re.to_value_radius(&fctx);
        let (im_true, _) = out.im.to_value_radius(&fctx);
        assert!(covers(&out_err.re, &re_true));
        assert!(covers(&out_err.im, &im_true));
    }

    #[test]
    fn from_parts_is_exact_at_any_width() {
        let z = CBall::from_parts(&repr(3, 0), &repr(4, 0), 20);
        assert!(z.re.rad.is_zero() && z.im.rad.is_zero());
        // an over-precise part is *not* rounded at the seed precision: the seed stays exact
        // (the operations round their own results)
        let wide = Repr::new(IBig::from(1025), -10); // 11 bits at seed precision 10
        let z = CBall::from_parts(&wide, &Repr::<2>::zero(), 10);
        assert!(z.re.rad.is_zero());
        assert_eq!(z.re.mid, wide);
    }

    // √(−1 − 0i) = +0 − i: the real part of the principal root is never a negative zero.
    #[test]
    fn sqrt_negative_real_with_negative_zero_imaginary() {
        let p = 10;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let out = cb(repr(-1, 0), Repr::<2>::neg_zero())
            .sqrt(&fctx, p)
            .unwrap();
        assert!(out.re.rad.is_zero() && out.im.rad.is_zero());
        assert!(out.re.mid.is_pos_zero());
        assert_eq!(out.im.mid, repr(-1, 0));
    }

    // A zero midpoint that still carries a radius must not root to an *exact* zero.
    #[test]
    fn sqrt_real_of_inexact_zero_is_not_exact() {
        let p = 10;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let arg = Ball::with_error(Repr::<2>::zero(), Mag::from_repr(&repr(1, -20)));
        let out = sqrt_real(arg, &fctx, p).unwrap();
        assert!(out.rad.is_infinite());
    }
    // A kernel's `Exact` flag only yields a zero radius where the call site says the result
    // can be exact; otherwise the value is treated as rounded (one work-ulp).
    #[test]
    fn kernel_ball_trusts_exact_only_when_possible() {
        let p = 20;
        let fctx = FloatCtxt::<mode::HalfEven>::new(p);
        let one = F::from_repr(repr(1, 0), fctx);
        let b = kernel_ball(Approximation::Exact(one.clone()), p, |_| true);
        assert!(b.rad.is_zero());
        let b = kernel_ball(Approximation::Exact(one.clone()), p, |_| false);
        assert!(!b.rad.is_zero() && b.mid == repr(1, 0));
        let b = kernel_ball(Approximation::Inexact(one, Rounding::NoOp), p, |_| true);
        assert!(!b.rad.is_zero());
        // an infinite special value passes through as it is
        let inf = F::from_repr(Repr::neg_infinity(), fctx);
        let b = kernel_ball(Approximation::Exact(inf), p, |_| false);
        assert!(b.mid.is_infinite() && b.rad.is_zero());
    }

    #[test]
    fn exactness_criteria() {
        // hypot: 3² + 4² = 5², but 1² + (2⁴⁰)² is not a square
        assert!(hypot_is_exact(&repr(5, 0), &repr(3, 0), &repr(4, 0), 10));
        assert!(!hypot_is_exact(&repr(1, 40), &repr(1, 0), &repr(1, 40), 53));
        assert!(hypot_is_exact(&repr(7, 3), &repr(-7, 3), &Repr::zero(), 10));
        // far-apart operands are answered without building the aligned sum
        assert!(!hypot_is_exact(&repr(1, 1 << 40), &repr(1, 0), &repr(1, 1 << 40), 53));
        // sqrt
        assert!(square_is(&repr(3, -1), &repr(9, -2)));
        assert!(!square_is(&repr(1, 40), &(repr(1, 80).clone() + repr(1, 0))));
        // the ×π lattice
        assert!(six_x_is_integer(&repr(1, -1)) && six_x_is_integer(&repr(-5, 0)));
        assert!(six_x_is_integer(&Repr::<2>::zero()));
        assert!(!six_x_is_integer(&repr(1, -2)));
        assert!(six_x_is_integer(&Repr::<3>::new(IBig::from(1), -1))); // 1/3 in base 3
                                                                       // integer powers
        assert!(powi_is_exact(&repr(81, 0), &repr(3, 0), &IBig::from(4), 20));
        assert!(!powi_is_exact(&repr(80, 0), &repr(3, 0), &IBig::from(4), 20));
        assert!(powi_is_exact(&repr(1, -6), &repr(1, 2), &IBig::from(-3), 20));
        assert!(!powi_is_exact(&repr(1, -4), &repr(3, 0), &IBig::from(-2), 20));
        // 1/8 = 0.125 in base 10
        let eighth = Repr::<10>::new(IBig::from(125), -3);
        let two = Repr::<10>::new(IBig::from(2), 0);
        assert!(powi_is_exact(&eighth, &two, &IBig::from(-3), 20));
    }
}
