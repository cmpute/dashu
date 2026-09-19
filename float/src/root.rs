use dashu_base::{
    ring::DivRem, Approximation, CubicRoot, EstimatedLog2, Sign, SquareRoot, SquareRootRem,
    UnsignedAbs,
};
use dashu_int::{IBig, UBig};

use crate::{
    ball::Ball,
    error::{assert_limited_precision, panic_root_zeroth, FpError, FpResult},
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{mode, ErrorBounds, Round, Rounding},
    utils::{digit_len, shl_digits, split_digits_ref},
};
use core::cmp::Ordering;

impl<R: ErrorBounds, const B: Word> SquareRoot for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn sqrt(&self) -> Self {
        self.context.unwrap_fp(self.context.sqrt(self.repr()))
    }
}

impl<R: Round, const B: Word> CubicRoot for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn cbrt(&self) -> Self {
        self.context.unwrap_fp(self.context.cbrt(self.repr()))
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the nth root of the floating point number.
    ///
    /// When `n` is large the computation can be expensive — the significand is
    /// aligned to up to `n · precision` digits before the integer root is taken, and
    /// the integer Newton iteration works with numbers of that size. For large
    /// `n` consider [`powf`][`FBig::powf`] with a rational exponent `1 / n`
    /// as a faster approximate alternative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("16")?;
    /// assert_eq!(a.nth_root(4), DBig::from_str("2")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero, or if `n` is even and the number is negative.
    #[inline]
    pub fn nth_root(&self, n: usize) -> Self {
        self.context
            .unwrap_fp(self.context.nth_root(n, self.repr()))
    }
}

impl<R: Round> Context<R> {
    /// Calculate the cubic root of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("8")?;
    /// assert_eq!(context.cbrt(&a.repr()), Ok(Exact(DBig::from_str("2")?)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn cbrt<const B: Word>(&self, x: &Repr<B>) -> FpResult<FBig<R, B>> {
        self.nth_root(3, x)
    }

    /// Calculate the nth root of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("27")?;
    /// assert_eq!(context.nth_root(3, &a.repr()), Ok(Exact(DBig::from_str("3")?)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero, if the precision is unlimited, or if `n` is even and `x` is negative.
    pub fn nth_root<const B: Word>(&self, n: usize, x: &Repr<B>) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);
        if n == 0 {
            panic_root_zeroth()
        }
        debug_assert!(n < isize::MAX as usize);
        let sign = x.sign();
        if sign == Sign::Negative && n % 2 == 0 {
            return Err(FpError::OutOfDomain);
        }
        if n == 1 {
            return Ok(self.repr_round_ref(x).map(|v| FBig::new(v, *self)));
        }
        if x.significand.is_zero() {
            // UBig::ZERO.nth_root(n) erroneously returns ONE, so short-circuit here.
            // An even root of -0 already errored above, so reaching here the sign is
            // preserved: odd root of ±0 is ±0.
            return Ok(Approximation::Exact(FBig::new(x.clone(), *self)));
        }

        // operate on the magnitude so that shifting/splitting keep a clean sign;
        // the original sign is re-applied to the result at the end.
        let xmag: IBig = if sign == Sign::Negative {
            -&x.significand
        } else {
            x.significand.clone()
        };

        // Adjust the significand so that the exponent is divisible by n and the root of the
        // aligned significand carries exactly `precision` digits. The alignment allows two
        // shifts that differ by n digits: `+r` (padding the significand up) or `r − n`
        // (truncating it, with the dropped part kept as the sticky `low`). When r > 0 the
        // padding choice yields n·precision + r aligned digits whose root carries
        // `precision + 1` digits, which would then have to be rounded to the precision in a
        // second step — and the two roundings disagree exactly at midpoints of the coarse
        // grid (round-to-integer-then-re-round is a double rounding). The truncating choice
        // keeps the root at exactly `precision` digits, so a single rounding decides.
        let digits = x.digits() as isize;
        let r = (x.exponent + digits).rem_euclid(n as isize);
        let shift =
            n as isize * self.precision as isize - digits + r - n as isize * (r > 0) as isize;
        let (signif, low, low_digits) = if shift > 0 {
            (shl_digits::<B>(&xmag, shift as usize), IBig::ZERO, 0)
        } else {
            let shift = (-shift) as usize;
            let (hi, lo) = split_digits_ref::<B>(&xmag, shift);
            (hi, lo, shift)
        };

        let mag: UBig = signif.unsigned_abs();
        let root: UBig = mag.nth_root(n);
        let rem: UBig = &mag - root.clone().pow(n);
        let exp = (x.exponent - shift) / n as isize;

        let result_sign = if sign == Sign::Negative {
            Sign::Negative
        } else {
            Sign::Positive
        };
        let signed_root: IBig = result_sign * root.clone();
        debug_assert!(digit_len::<B>(root.as_ibig()) <= self.precision);

        let res = if rem.is_zero() && low.is_zero() {
            Approximation::Exact(signed_root)
        } else {
            // The true value is (root + frac)·BASE^exp with frac ∈ (0, 1), where root =
            // floor(mag^(1/n)) and the fraction continues into the truncated input part
            // `low`. Comparing frac against 1/2:
            //   2·(root + frac) vs 2·root + 1
            //   ⟺ 2^n·full vs (2·root + 1)^n·BASE^low_digits
            // where full = mag·BASE^low_digits + low is the full aligned significand.
            let adjust = R::round_low_part(&signed_root, result_sign, || {
                let base_pow = Repr::<B>::BASE.pow(low_digits);
                let full = &mag * &base_pow + low.unsigned_abs();
                let lhs = full << n;
                let rhs = ((root.clone() << 1) + UBig::from_word(1)).pow(n) * base_pow;
                lhs.cmp(&rhs)
            });
            Approximation::Inexact(signed_root.clone() + adjust, adjust)
        };
        Ok(res
            .map(|signif| Repr::new(signif, exp))
            .map(|v| FBig::new(v, *self)))
    }
}

impl<R: ErrorBounds> Context<R> {
    /// Calculate the square root of the floating point number (correctly rounded).
    ///
    /// The integer square root of the exponent-aligned significand (which carries ~2·p digits) is
    /// computed exactly, and its rounding to `p` digits is decided in a single step from the round
    /// digit and the `sqrtrem` remainder (the sticky bit) — the same principle as MPFR. When the
    /// root has `p + 1` digits it is rounded directly to `p` digits, avoiding the double rounding
    /// that a rem-vs-root + re-round path incurs. For a power-of-two base this fast path is already
    /// correctly rounded; for other bases the result is additionally certified by a Ziv loop (the
    /// base-`B` digit alignment of an integer square root is only clean when the base is a power of
    /// two).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn sqrt<const B: Word>(&self, x: &Repr<B>) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        if x.significand.is_zero() {
            // sqrt(+0) = +0, sqrt(-0) = -0 (preserve the sign of zero). Exact, so handle
            // it before the limited-precision assertion: a precision-0 (unlimited) value
            // such as the one from `try_from(0.0)` must still compute sqrt(0) exactly.
            return Ok(Approximation::Exact(FBig::new(x.clone(), *self)));
        }
        assert_limited_precision(self.precision);
        if x.sign() == Sign::Negative {
            return Err(FpError::OutOfDomain);
        }

        // One-step correctly-rounded sqrt of the (finite, positive, limited) input at a working
        // precision, used directly by the power-of-two fast path and by the Ziv loop otherwise.
        let sqrt_rounded = |guard: usize| -> FpResult<FBig<R, B>> {
            let gctx = Context::<R>::new(self.precision + guard);
            let p = gctx.precision;

            // Adjust the significand so the exponent is even, with ~2p significant digits.
            let digits = x.digits() as isize;
            let shift = p as isize * 2 - (digits & 1) + (x.exponent & 1) - digits;
            let (signif, low, low_digits) = if shift > 0 {
                (shl_digits::<B>(&x.significand, shift as usize), IBig::ZERO, 0)
            } else {
                let shift = (-shift) as usize;
                let (hi, lo) = split_digits_ref::<B>(&x.significand, shift);
                (hi, lo, shift)
            };

            let (root, rem) = signif.unsigned_abs().sqrt_rem();
            let exp = (x.exponent - shift) / 2;
            let exact = rem.is_zero() && low.is_zero();
            // The root has `p` or `p+1` base-B digits, decided exactly in O(1) from the shifted
            // significand's digit count: `signif` carries `2p − (digits&1) + (exp&1)` digits, so it
            // has 2p+1 digits (root has p+1) exactly when the significand's digit count is even and
            // the input exponent is odd.
            let root_is_p1 = (digits & 1) == 0 && (x.exponent & 1) == 1;

            // The result's exponent. A p+1-digit root is rounded to p digits by dropping the lowest
            // base-B digit (`r = root / B`), which shifts the value by one base-B digit: `exp + 1`.
            // The arithmetic works on the unsigned `root`; it is converted to `IBig` only where
            // `round_low_part` (signed) and the result's `+ Rounding` need it.
            let (sig, adjust, result_exp) = if !root_is_p1 {
                // p-digit root: the remainder (and any truncated input) decide the rounding. An
                // integer sqrt has no exact half-tie (`sqrt(n) = root + 1/2` would require
                // `4·rem = 2·root + 1`, impossible), so the rem-vs-root comparison is the exact
                // single rounding.
                let adjust = if exact {
                    Rounding::NoOp
                } else {
                    R::round_low_part(root.as_ibig(), Sign::Positive, || {
                        rem.cmp(&root)
                            .then_with(|| (low << 2).cmp(&Repr::<B>::BASE.pow(low_digits).into()))
                    })
                };
                (IBig::from(root), adjust, exp)
            } else {
                // p+1-digit root: round to p digits in one step. `r` = top p digits, `d` = round
                // digit (a Word, from the single `div_rem`).
                let (r, d) = root.div_rem(B);
                let adjust = if exact {
                    // The exact p+1-digit root: a clean rounding of `2·d` vs `B`, ties per the mode
                    // (`2·d = B` is the real half-tie; `d vs ⌊B/2⌋` would mis-round odd bases).
                    R::round_low_part(r.as_ibig(), Sign::Positive, || (d * 2).cmp(&B))
                } else {
                    // The true value is strictly above `root` (sticky remainder / truncated input),
                    // so a round digit at the real half (2·d = B) is strictly past the half — report
                    // Greater so every mode (including the directed ones) rounds from the correct
                    // side. Delegating to `round_low_part` is what makes directed modes work: they
                    // ignore the comparison and truncate/extend per their own direction.
                    R::round_low_part(r.as_ibig(), Sign::Positive, || match (d * 2).cmp(&B) {
                        Ordering::Equal => Ordering::Greater,
                        other => other,
                    })
                };
                (IBig::from(r), adjust, exp + 1)
            };

            let res = if exact && !root_is_p1 {
                Approximation::Exact(sig)
            } else {
                Approximation::Inexact(sig + adjust, adjust)
            };
            Ok(res
                .map(|signif| Repr::new(signif, result_exp))
                .and_then(|v| gctx.repr_round(v))
                .map(|v| FBig::new(v, gctx)))
        };

        if B.is_power_of_two() {
            sqrt_rounded(0)
        } else {
            // Near-correct kernel, mechanical radius: the base-`B` digit alignment of the integer
            // square root is only clean when the base is a power of two, so for other bases the
            // single rounding step is bounded by one ulp at the working precision — the same
            // assumption every other [`Ball`] operator makes of its kernel.
            //
            // Routing it through `Ball::from_rounded` also gives an *exact* root `rad == 0`, and
            // that is load-bearing: a zero radius is the only one Ziv can certify against a
            // one-sided directed preimage (`Down`'s `[y, y+ulp)` cannot contain `[y−r, y+r]` for
            // any `r > 0`). With a blanket `value.ulp()` the loop never converged on a perfect
            // square — `sqrt(4)` in base 10 under `Down`/`Up`/`Zero` doubled its working precision
            // to ~10^8 digits instead of returning `2`.
            let initial_guard = crate::utils::ceil_usize(self.precision.log2_est()) + 10;
            self.ziv(initial_guard, |guard| {
                let wp = self.precision + guard;
                let rounded = sqrt_rounded(guard)?;
                Ok(Ball::from_rounded(rounded.map(FBig::into_repr), wp)
                    .to_value_radius::<R>(&Context::<R>::new(wp)))
            })
        }
    }

    /// Compute `sqrt(a² + b²)` without spurious overflow/underflow.
    ///
    /// This is the overflow-safe scaled sum-of-squares: the larger-magnitude operand is never
    /// squared. Writing `m = max(|a|, |b|)` and `r = min(|a|,|b|) / m` (so `|r| ≤ 1`), the result is
    /// `m · sqrt(1 + r²)`, where `1 + r² ∈ [1, 2]` cannot overflow. The result is correctly rounded
    /// via a Ziv retry loop (`hypot(±inf, ·) = +inf`, `hypot(0, 0) = +0`).
    ///
    /// This is a field-arithmetic-class op (no constant cache), like `sqrt`/`atan2`.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn hypot<const B: Word>(&self, a: &Repr<B>, b: &Repr<B>) -> FpResult<FBig<R, B>> {
        if a.is_infinite() || b.is_infinite() {
            return Ok(Approximation::Exact(FBig::new(Repr::infinity(), *self)));
        }
        assert_limited_precision(self.precision);
        if a.significand.is_zero() && b.significand.is_zero() {
            return Ok(Approximation::Exact(FBig::new(Repr::zero(), *self)));
        }

        // magnitudes, ordered large >= small (both finite, not both zero here)
        let a_mag = if a.sign() == Sign::Negative {
            -a.clone()
        } else {
            a.clone()
        };
        let b_mag = if b.sign() == Sign::Negative {
            -b.clone()
        } else {
            b.clone()
        };
        let (large, small) = if a_mag.cmp(&b_mag).is_ge() {
            (a_mag, b_mag)
        } else {
            (b_mag, a_mag)
        };

        if small.significand.is_zero() {
            // hypot(x, 0) = |x|; `large` is already a magnitude.
            return Ok(self.repr_round_ref(&large).map(|v| FBig::new(v, *self)));
        }

        // The result is `sqrt(large² + small²)`, i.e. ∈ [large, large·√2]. It overflows only when
        // `large` is so large that the result reaches the infinity sentinel exponent — unreachable
        // for real inputs, but pre-checked here so the Ziv closure can use infallible `FBig`
        // arithmetic.
        if large.exponent >= isize::MAX - 1 {
            return Err(FpError::Overflow(Sign::Positive));
        }

        let initial_guard = crate::utils::ceil_usize(self.precision.log2_est()) + 10;
        self.ziv(initial_guard, |guard| {
            let gctx = Context::<mode::HalfEven>::new(self.precision + guard);
            // result = sqrt(large² + small²), with both operands scaled down by `k` base-B digits
            // before squaring (so `large²` can't overflow the exponent) and the root scaled back:
            // sqrt(L² + S²) · B^k = sqrt(large² + small²) for L = large·B⁻ᵏ, S = small·B⁻ᵏ. No
            // division — so for integer inputs every step is exact (MPFR's `exact` flag), and an
            // all-exact chain yields the exact true value. The tracking variants report radius 0
            // then, which `ziv` accepts without the containment test (it can't certify an
            // exactly-representable result under directed rounding — e.g. hypot(3,4)=5,
            // hypot(5,12)=13 — which sits on a one-sided preimage boundary).
            let k = (large.exponent as i128 - (isize::MAX as i128 - 2) / 2).max(0) as isize;
            let wp = gctx.precision;
            // The input roundings' exactness folds through `from_rounded`: an exact input keeps
            // `rad = 0`, so an all-exact chain (integer inputs, no rounding anywhere) carries a
            // zero radius — the exactly-representable directed-rounding case that no nonzero
            // radius could certify (e.g. hypot(3,4)=5, hypot(5,12)=13).
            let large_ball = Ball::from_rounded(gctx.repr_round_ref(&large), wp);
            let small_ball = Ball::from_rounded(gctx.repr_round_ref(&small), wp);
            // The shifted balls are used twice (the square), so bind them once — a shift is a full
            // O(p) clone otherwise.
            let l = large_ball.shift(-k);
            let s = small_ball.shift(-k);
            let l_sq = l.mul(&l, wp)?;
            let s_sq = s.mul(&s, wp)?;
            let sum = l_sq.add(&s_sq, wp)?;
            let root = sum.sqrt(wp)?;
            let result = root.shift(k); // exact exponent shift — scales back, radius unchanged
            Ok(result.to_value_radius::<R>(&Context::<R>::new(wp)))
        })
    }
}

impl<R: ErrorBounds, const B: Word> FBig<R, B> {
    /// Calculate the square root of the floating point number (correctly rounded).
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn sqrt(&self) -> Self {
        self.context.unwrap_fp(self.context.sqrt(&self.repr))
    }

    /// Compute `sqrt(self² + other²)` without spurious overflow/underflow.
    ///
    /// The result precision is `max(self.precision(), other.precision())`. See
    /// [`Context::hypot`] for the overflow-safety strategy.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("3")?;
    /// let b = DBig::from_str("4")?;
    /// assert_eq!(a.hypot(&b), DBig::from_str("5")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn hypot(&self, other: &Self) -> Self {
        let context = Context::max(self.context, other.context);
        context.unwrap_fp(context.hypot(&self.repr, &other.repr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use dashu_base::Approximation::*;

    fn r2(sig: i32, exp: isize) -> Repr<2> {
        Repr::new(sig.into(), exp)
    }

    // sqrt under directed modes must bracket the true value (issue #99): for a positive
    // non-square x, sqrt_down(x)² < x < sqrt_up(x)² and the two results differ. The inputs
    // 6 = 3·2¹ and 24 = 3·2³ exercise the p+1-digit integer-root path (even digit count, odd
    // exponent) whose sticky remainder the old code rounded the wrong way under Up/Away. Every
    // input here has a non-square odd part, so x is never a perfect square at any exponent.
    #[test]
    fn test_sqrt_directed_modes_bracket() {
        for x in [3i32, 5, 6, 7, 10, 24, 30] {
            for exp in [-3isize, -1, 0, 1, 3] {
                let input = Repr::<2>::new(IBig::from(x), exp);
                for p in [20usize, 50, 100, 500] {
                    let down = Context::<mode::Down>::new(p)
                        .sqrt(&input)
                        .unwrap()
                        .value()
                        .with_precision(0)
                        .value();
                    let up = Context::<mode::Up>::new(p)
                        .sqrt(&input)
                        .unwrap()
                        .value()
                        .with_precision(0)
                        .value();
                    let x = FBig::<mode::HalfEven>::new(input.clone(), Context::new(0));
                    // squares at unlimited precision are exact, so these comparisons are exact
                    assert!(&down * &down < x, "sqrt({input:?}) @ p{p}: Down bound too high");
                    assert!(&up * &up > x, "sqrt({input:?}) @ p{p}: Up bound too low");
                    assert!(down < up, "sqrt({input:?}) @ p{p}: Down == Up on a non-square");
                }
            }
        }
    }

    // Perfect squares are exact under every mode and yield identical Down/Up results.
    #[test]
    fn test_sqrt_perfect_square_is_exact() {
        for (input, root) in [
            (r2(1, 2), r2(1, 1)),
            (r2(9, -2), r2(3, -1)),
            (r2(25, -4), r2(5, -2)),
        ] {
            for p in [20usize, 50] {
                let r = Context::<mode::HalfEven>::new(p).sqrt(&input).unwrap();
                assert!(matches!(r, Exact(..)), "sqrt({input:?}) @ p{p} should be exact");
                assert_eq!(r.value().repr(), &root);
                assert_eq!(
                    Context::<mode::Down>::new(p)
                        .sqrt(&input)
                        .unwrap()
                        .value()
                        .repr(),
                    &root
                );
                assert_eq!(
                    Context::<mode::Up>::new(p)
                        .sqrt(&input)
                        .unwrap()
                        .value()
                        .repr(),
                    &root
                );
            }
        }
    }

    // The same class of bug at tiny precision: sqrt(1.75) = 1.3228… has the p+1-digit integer
    // root 5 (= 5·2^-2 = 1.25), exactly the midpoint of the p2 neighbours 1 and 1.5 — but the
    // inexact root's remainder puts the true value strictly above the midpoint, so HalfEven
    // must round up to 1.5 (the old code tied down to 1.0).
    #[test]
    fn test_sqrt_p1_digit_root_midpoint() {
        let v = Context::<mode::HalfEven>::new(2)
            .sqrt(&r2(7, -2))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(3, -1));
        // the same input under Up/Down brackets 1.3228… correctly
        let up = Context::<mode::Up>::new(2)
            .sqrt(&r2(7, -2))
            .unwrap()
            .value();
        let down = Context::<mode::Down>::new(2)
            .sqrt(&r2(7, -2))
            .unwrap()
            .value();
        assert_eq!(up.repr(), &r2(3, -1));
        assert_eq!(down.repr(), &r2(1, 0));
    }

    // An exact p+1-digit root is rounded to the precision in one step: sqrt(1.5625) = 1.25 is
    // exactly the midpoint of 1.0 and 1.5; ties to even picks the significand 10₂ -> 1.0. The
    // result differs from the exact root, so the operation is still Inexact.
    #[test]
    fn test_sqrt_exact_p1_digit_root_ties_to_even() {
        let r = Context::<mode::HalfEven>::new(2).sqrt(&r2(25, -4)).unwrap();
        assert!(matches!(r, Inexact(..)));
        assert_eq!(r.value().repr(), &r2(1, 0));
    }

    // nth_root must round a precision+1-digit integer root in a single step (issue #100):
    // the old code rounded the root to an integer first and re-rounded, which ties the wrong
    // way exactly at midpoints of the coarse grid.
    #[test]
    fn test_nth_root_single_step_rounding() {
        // nth_root(2, 1.75) @ p2 HalfEven = 1.5 (see test_sqrt_p1_digit_root_midpoint)
        let v = Context::<mode::HalfEven>::new(2)
            .nth_root(2, &r2(7, -2))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(3, -1));

        // exact root one digit wider than the precision: nth_root(2, 2.25) = 1.5 @ p2
        let r = Context::<mode::HalfEven>::new(2)
            .nth_root(2, &r2(9, -2))
            .unwrap();
        assert!(matches!(r, Exact(..)));
        assert_eq!(r.value().repr(), &r2(3, -1));

        // directed modes bracket on the same input
        let up = Context::<mode::Up>::new(2)
            .nth_root(2, &r2(7, -2))
            .unwrap()
            .value();
        let down = Context::<mode::Down>::new(2)
            .nth_root(2, &r2(7, -2))
            .unwrap()
            .value();
        assert_eq!(up.repr(), &r2(3, -1));
        assert_eq!(down.repr(), &r2(1, 0));

        // agreement with sqrt on a perfect square: nth_root(2, 16) @ p5 = 4
        let v = Context::<mode::HalfEven>::new(5)
            .nth_root(2, &r2(1, 4))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 2));
    }

    #[test]
    #[should_panic]
    fn test_fbig_sqrt_negative_panics() {
        // sqrt(-1) is out of domain; the FBig layer panics.
        let neg_one = FBig::<mode::HalfEven>::try_from(-1.0f64).unwrap();
        let _ = neg_one.sqrt();
    }

    #[test]
    fn test_hypot_pythagorean() {
        let ctx = Context::<mode::HalfEven>::new(53);
        let mk = |v: i32| Repr::<2>::new(v.into(), 0);
        // hypot(3, 4) = 5
        let r = ctx.hypot(&mk(3), &mk(4)).unwrap().value();
        assert_eq!(r.repr().significand(), &5.into());
        // hypot(5, 0) = 5
        let r = ctx.hypot(&mk(5), &mk(0)).unwrap().value();
        assert_eq!(r.repr().significand(), &5.into());
        // hypot(0, 0) = 0
        let r = ctx.hypot(&mk(0), &mk(0)).unwrap().value();
        assert!(r.repr().is_pos_zero());
        // hypot(inf, x) = +inf
        let r = ctx.hypot(&Repr::infinity(), &mk(3)).unwrap().value();
        assert!(r.repr().is_infinite());
        assert_eq!(r.repr().sign(), Sign::Positive);
    }

    fn check_hypot_exact_triples<R: ErrorBounds>(ctx: Context<R>) {
        let mk = |v: i32| Repr::<2>::new(v.into(), 0);
        // Pythagorean triples: the result is exactly representable, so under a directed mode it
        // sits on a one-sided preimage boundary. The closure must terminate (radius 0 from the
        // all-exact `sqrt(large²+small²)` chain) rather than infinite-retry.
        for (a, b, h) in [(3, 4, 5), (5, 12, 13), (8, 15, 17)] {
            let r = ctx.hypot(&mk(a), &mk(b)).unwrap().value();
            assert_eq!(r.repr().significand(), &h.into(), "hypot({a}, {b})");
        }
    }

    #[test]
    fn test_hypot_exact_under_directed_rounding() {
        check_hypot_exact_triples(Context::<mode::Down>::new(53));
        check_hypot_exact_triples(Context::<mode::Up>::new(53));
        check_hypot_exact_triples(Context::<mode::Zero>::new(53));
    }

    #[test]
    fn test_hypot_no_spurious_overflow() {
        // a value whose square would collide with the +inf sentinel exponent, but whose
        // hypot is itself representable: hypot(a, 0) = |a| must not overflow via a².
        let ctx = Context::<mode::HalfEven>::new(53);
        // exponent near isize::MAX/2 so that a² would overflow, but |a| is fine
        let a = Repr::<2>::new(IBig::from(3), isize::MAX / 2);
        let r = ctx.hypot(&a, &Repr::<2>::zero()).unwrap().value();
        assert_eq!(r.repr().exponent(), isize::MAX / 2);
    }

    /// A perfect square in a *non-power-of-two* base must certify under every rounding mode,
    /// the one-sided directed ones included.
    ///
    /// Regression: `sqrt`'s Ziv closure reported a blanket `value.ulp()` radius, never zero, and
    /// no `r > 0` fits inside `Down`'s preimage `[y, y+ulp)` — so an exactly-representable root
    /// could not be certified at all. `sqrt(4)` in base 10 under `Down`/`Up`/`Zero` doubled its
    /// working precision until the retry budget ran out (~10^8 digits) instead of returning `2`.
    /// The root is now wrapped as a [`Ball`], so an exact result carries `rad == 0`.
    #[test]
    fn test_sqrt_exact_root_certifies_under_directed_rounding() {
        // (input, expected exact root) — perfect squares, and a non-square for contrast
        let cases: [(i64, isize, i64, isize, bool); 4] = [
            (4, 0, 2, 0, true),
            (100, 0, 10, 0, true),
            (9, 0, 3, 0, true),
            (25, -2, 5, -1, true), // 0.25 → 0.5
        ];
        macro_rules! check {
            ($m:ty, $name:expr) => {
                for (sig, exp, root_sig, root_exp, exact) in cases {
                    let x = Repr::<10>::new(IBig::from(sig), exp);
                    let r = Context::<$m>::new(10).sqrt::<10>(&x).unwrap();
                    let want = Repr::<10>::new(IBig::from(root_sig), root_exp);
                    assert_eq!(
                        matches!(r, Approximation::Exact(_)),
                        exact,
                        "{}: sqrt({sig}e{exp}) exactness",
                        $name
                    );
                    assert_eq!(r.value().repr(), &want, "{}: sqrt({sig}e{exp})", $name);
                }
            };
        }
        check!(mode::Down, "Down");
        check!(mode::Up, "Up");
        check!(mode::Zero, "Zero");
        check!(mode::HalfEven, "HalfEven");
        check!(mode::HalfAway, "HalfAway");
    }
}
