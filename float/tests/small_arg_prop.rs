//! Correct rounding — value *and* `Rounding` flag — of the trigonometric and hyperbolic
//! functions at small arguments, where the result hugs an exactly representable value
//! (`sin x ≈ x`, `cos x ≈ 1`, …).
//!
//! The oracle is a certified bracket, independent of the implementation: two Taylor terms
//! with a rigorous remainder bound, evaluated in exact integer arithmetic. A case counts only
//! when both ends of the bracket round identically (value and flag), which pins the rounding of
//! everything in between. The exponent range straddles the point where the implementation
//! switches from its Ziv loop to the tiny-argument shortcut, so both are checked against it.

use dashu_base::{Approximation::Inexact, Sign, UnsignedAbs};
use dashu_float::round::{mode, ErrorBounds, Rounded};
use dashu_float::{Context, FBig, FpResult, Repr};
use dashu_int::IBig;
use proptest::prelude::*;

/// A function `f(x) = lead + c·x^k/d + O(x^(k+2))` near zero.
struct Spec {
    /// `true`: `lead = 1` and `k = 2` (cos, cosh); `false`: `lead = x` and `k = 3`.
    even: bool,
    /// The sign `c` of the second term.
    plus: bool,
    /// Its divisor `d`.
    div: u8,
}

const SIN: Spec = Spec::odd(false, 6);
const TAN: Spec = Spec::odd(true, 3);
const ASIN: Spec = Spec::odd(true, 6);
const ATAN: Spec = Spec::odd(false, 3);
const SINH: Spec = Spec::odd(true, 6);
const TANH: Spec = Spec::odd(false, 3);
const ASINH: Spec = Spec::odd(false, 6);
const ATANH: Spec = Spec::odd(true, 3);
const COS: Spec = Spec::even(false);
const COSH: Spec = Spec::even(true);

impl Spec {
    const fn odd(plus: bool, div: u8) -> Self {
        Spec {
            even: false,
            plus,
            div,
        }
    }
    const fn even(plus: bool) -> Self {
        Spec {
            even: true,
            plus,
            div: 2,
        }
    }

    /// The certified rounding of `f(x)` at `p` bits under `R`, or `None` if the bracket
    /// straddles a rounding boundary.
    ///
    /// For `0 < t = |x| ≤ 1/2`, with `n = k + 2` the order of the first omitted term:
    /// `lead + c·t^k/d + t^n/128 < f(t) < lead + c·t^k/d + t^n` — the omitted Taylor tail of
    /// every function above is positive and between `t^n/120` (sin, sinh) and `t^n/5` (atan,
    /// atanh) at the leading order, comfortably inside `[1/128, 1]` on this range. Everything is
    /// an integer multiple of `2^-s`; the two inexact divisions are rounded outward.
    fn oracle<R: ErrorBounds>(&self, x: &Repr<2>, p: usize) -> Option<Rounded<FBig<R, 2>>> {
        let m = IBig::from(x.significand().clone().unsigned_abs());
        let e = x.exponent();
        let (k, n) = if self.even { (2, 4) } else { (3, 5) };
        let s = (-e) as usize * n + 80;
        let scaled = |pow: usize| m.pow(pow) << (s - (-e) as usize * pow); // t^pow · 2^s
        let lead = if self.even { IBig::ONE << s } else { scaled(1) };
        // c·t^k/d, floored and ceiled
        let term = scaled(k) * if self.plus { IBig::ONE } else { IBig::NEG_ONE };
        let d = IBig::from(self.div);
        let floor = (&term - ((&term % &d) + &d) % &d) / &d;
        let ceil = &floor + IBig::ONE;
        let tail = scaled(n);
        let lo = &lead + floor + (&tail >> 7);
        let hi = lead + ceil + tail + IBig::ONE;
        // odd functions carry the sign of x; the even ones do not depend on it
        let negate = !self.even && x.significand().sign() == Sign::Negative;
        let round = |v: IBig| {
            let v = if negate { -v } else { v };
            FBig::<R, 2>::from_repr(Repr::new(v, -(s as isize)), Context::new(0)).with_precision(p)
        };
        let (lo, hi) = (round(lo), round(hi));
        (lo == hi && matches!(lo, Inexact(..))).then_some(lo)
    }
}

type Unary<R> = fn(&Context<R>, &Repr<2>) -> FpResult<FBig<R, 2>>;

/// Checks every function under `R`; returns how many of them the oracle could certify.
fn check_mode<R: ErrorBounds>(x: &Repr<2>, p: usize) -> Result<usize, TestCaseError> {
    let ctx = Context::<R>::new(p);
    let cases: [(&str, &Spec, Unary<R>); 10] = [
        ("sin", &SIN, |c, x| c.sin(x, None)),
        ("cos", &COS, |c, x| c.cos(x, None)),
        ("tan", &TAN, |c, x| c.tan(x, None)),
        ("asin", &ASIN, |c, x| c.asin(x, None)),
        ("atan", &ATAN, |c, x| c.atan(x, None)),
        ("sinh", &SINH, |c, x| c.sinh(x, None)),
        ("cosh", &COSH, |c, x| c.cosh(x, None)),
        ("tanh", &TANH, |c, x| c.tanh(x, None)),
        ("asinh", &ASINH, |c, x| c.asinh(x, None)),
        ("atanh", &ATANH, |c, x| c.atanh(x, None)),
    ];
    let mut certified = 0;
    for (name, spec, f) in cases {
        if let Some(want) = spec.oracle::<R>(x, p) {
            prop_assert_eq!(f(&ctx, x), Ok(want), "{}({:?}) at {} bits", name, x, p);
            certified += 1;
        }
    }
    // the paired entry points agree with the single ones
    let (s, c) = ctx.sin_cos(x, None);
    prop_assert_eq!(s, ctx.sin(x, None), "sin_cos({:?}).0 at {} bits", x, p);
    prop_assert_eq!(c, ctx.cos(x, None), "sin_cos({:?}).1 at {} bits", x, p);
    let (s, c) = ctx.sinh_cosh(x, None);
    prop_assert_eq!(s, ctx.sinh(x, None), "sinh_cosh({:?}).0 at {} bits", x, p);
    prop_assert_eq!(c, ctx.cosh(x, None), "sinh_cosh({:?}).1 at {} bits", x, p);
    Ok(certified)
}

fn check(x: &Repr<2>, p: usize) -> Result<usize, TestCaseError> {
    Ok(check_mode::<mode::Zero>(x, p)?
        + check_mode::<mode::Away>(x, p)?
        + check_mode::<mode::Up>(x, p)?
        + check_mode::<mode::Down>(x, p)?
        + check_mode::<mode::HalfEven>(x, p)?
        + check_mode::<mode::HalfAway>(x, p)?)
}

/// `(x, p)`: a significand of 1..=64 bits and a leading exponent from `−p/4 − 8` (where the
/// bracket is just narrow enough to certify) down to well past `−p/2`, where the result is
/// decided by the argument's own digits.
fn small_arg() -> impl Strategy<Value = (Repr<2>, usize)> {
    (
        prop::sample::select(vec![20usize, 50, 100, 500]),
        1u32..=64,
        any::<u64>(),
        any::<bool>(),
        0usize..=100,
    )
        .prop_map(|(p, bits, raw, negative, depth)| {
            // exactly `bits` bits: the top one set
            let mag = (raw >> (64 - bits)) | (1u64 << (bits - 1));
            let sig = IBig::from(mag) * if negative { -1 } else { 1 };
            let top = -((p / 4 + 8 + depth * (p / 2 + 40) / 100) as isize);
            (Repr::<2>::new(sig, top - bits as isize), p)
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..Default::default() })]

    #[test]
    fn small_argument_rounding((x, p) in small_arg()) {
        check(&x, p)?;
    }
}

/// Fixed cases: powers of two (where `x`, `1 − x²/2` and friends are themselves representable,
/// so the result sits right next to a rounding boundary), at each of the standard precisions.
#[test]
fn small_argument_rounding_fixed() {
    let (mut certified, mut total) = (0, 0);
    for p in [20usize, 50, 100, 500] {
        for top in [
            p / 4 + 8,
            p / 2 - 3,
            p / 2,
            p / 2 + 1,
            p / 2 + 2,
            p,
            3 * p,
            1000,
        ] {
            for sig in [1i32, -1, 3, 5, -7] {
                let x = Repr::<2>::new(sig.into(), -(top as isize));
                certified += check(&x, p).unwrap();
                total += 60; // 10 functions under 6 modes
            }
        }
    }
    // the oracle is not vacuous: its bracket settles nearly every one of these
    assert!(certified * 10 >= total * 9, "only {certified} of {total} cases certified");
}
