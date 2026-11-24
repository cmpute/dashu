//! Fixed regression cases for the complex square root: exactness flags, results next to a
//! rounding midpoint, and signed zeros on the branch cut.

mod helpers;

use dashu_cmplx::round::mode::{Down, HalfAway, HalfEven, Up};
use dashu_cmplx::{CBig, Context, FBig, Rounding};
use dashu_int::IBig;
use helpers::{assert_rounds_from, cb, fb, flags, retag_c, zero};

const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

type C = CBig<HalfEven, 2>;

fn c(re: (i64, isize), im: (i64, isize)) -> C {
    cb(re, im)
}

/// `√(±w²)` for an integer `w` that exactly fills the precision is `w`, and is reported
/// `Exact` — also when `w²` itself is wider than the working precision.
#[test]
fn sqrt_of_perfect_square_is_exact() {
    fn check<R: dashu_cmplx::round::ErrorBounds>(w: &str, p: usize) {
        let w: IBig = w.parse().unwrap();
        let sq: FBig<R, 10> = FBig::from_parts(&w * &w, 0);
        let root: FBig<R, 10> = FBig::from_parts(w, 0);
        let ctx = Context::<R>::new(p);
        // √(w² + 0i) = w + 0i
        let r = ctx.sqrt(&CBig::from_parts(sq.clone(), FBig::ZERO)).unwrap();
        assert_eq!(flags(&r), None, "sqrt(w^2) at p={p}");
        let v = r.value();
        assert_eq!(v.re(), root.repr());
        assert!(v.im().is_pos_zero());
        // √(−w² + 0i) = 0 + wi
        let r = ctx.sqrt(&CBig::from_parts(-sq, FBig::ZERO)).unwrap();
        assert_eq!(flags(&r), None, "sqrt(-w^2) at p={p}");
        let v = r.value();
        assert!(v.re().is_pos_zero());
        assert_eq!(v.im(), root.repr());
    }
    check::<HalfAway>("10000000000001", 14);
    check::<HalfAway>("8090807876197", 13);
    check::<HalfEven>("36396403115761677810", 20);
    check::<Down>("8090807876197", 13);
    check::<Up>("8090807876197", 13);
    check::<HalfEven>("8090807876197", 20);

    // a Gaussian-integer square: √((3 + 4i)²) = √(−7 + 24i) = 3 + 4i, at every precision
    for p in PRECISIONS {
        let r = Context::<HalfEven>::new(p)
            .sqrt(&c((-7, 0), (24, 0)))
            .unwrap();
        assert_eq!(flags(&r), None);
        assert!(r.value() == c((3, 0), (4, 0)));
    }
}

/// An irrational root is `Inexact` on both parts, also when a part's significand needed no
/// adjustment (a truncation is still a rounding).
#[test]
fn sqrt_of_non_square_is_inexact() {
    let ctx = Context::<HalfEven>::new(12);
    // √(8186i) = √4093·(1 + i), √4093 = 63.9765…: rounded down to 2047·2⁻⁵
    let r = ctx.sqrt(&c((0, 0), (8186, 0))).unwrap();
    assert_eq!(flags(&r), Some((Rounding::NoOp, Rounding::NoOp)));
    let v = r.value();
    assert_eq!(v.re(), fb::<HalfEven, 2>(2047, -5).repr());
    assert_eq!(v.im(), fb::<HalfEven, 2>(2047, -5).repr());
    for p in PRECISIONS {
        let r = Context::<HalfEven>::new(p)
            .sqrt(&c((2, 0), (0, 0)))
            .unwrap();
        // the real part is rounded, the imaginary part is an exact zero
        assert!(flags(&r).is_some());
        assert!(r.value().im().is_pos_zero());
    }
}

/// A root just below a rounding midpoint whose upper neighbour is a power of the base: the gap
/// below a power of the base is narrower, so the midpoint is nearer than "half an ulp" of the
/// neighbour. `√(8190i) = √4095·(1 + i)`, `√4095 = 63.99218702…` against the midpoint
/// `63.9921875` between `4095·2⁻⁶` and `64`.
#[test]
fn sqrt_just_below_midpoint_under_power_of_base() {
    let ctx = Context::<HalfEven>::new(12);
    let fctx = dashu_float::Context::<HalfEven>::new(12);
    for n in [4093i64, 4094, 4095] {
        let real = fctx.sqrt(fb::<HalfEven, 2>(n, 0).repr()).unwrap().value();
        let v = ctx.sqrt(&c((0, 0), (2 * n, 0))).unwrap().value();
        assert_eq!(v.re(), real.repr(), "re sqrt({}i)", 2 * n);
        assert_eq!(v.im(), real.repr(), "im sqrt({}i)", 2 * n);
    }
    let v = ctx.sqrt(&c((0, 0), (8190, 0))).unwrap().value();
    assert_eq!(v.re(), fb::<HalfEven, 2>(4095, -6).repr());

    // decimal: Im √(1 + 21e-15 i) = 10.4999…e-15, just below the tie 10.5e-15 (|z| > 1)
    let ctx = Context::<HalfAway>::new(2);
    for (y, want) in [(21i64, 10i64), (22, 11), (23, 11)] {
        let z: CBig<HalfAway, 10> = cb((1, 0), (y, -15));
        let r = ctx.sqrt(&z).unwrap();
        assert!(flags(&r).is_some());
        let v = r.value();
        assert_eq!(v.re(), fb::<HalfAway, 10>(1, 0).repr());
        assert_eq!(v.im(), fb::<HalfAway, 10>(want, -15).repr(), "im sqrt(1 + {y}e-15 i)");
    }
}

/// Under every mode and at every precision the root is the correctly rounded one (against a
/// higher-precision evaluation), including inputs whose root sits just under a power of two.
#[test]
fn sqrt_matches_oracle_in_all_modes() {
    macro_rules! check {
        ($mode:ty) => {{
            for p in PRECISIONS {
                let ctx = Context::<$mode>::new(p);
                let hi = Context::<HalfEven>::new(2 * p + 80);
                for z in [
                    cb::<$mode, 2>((0, 0), (8190, 0)),
                    cb((0, 0), (-8190, 0)),
                    cb((-4095, 0), (1, -40)),
                    cb((4095, 0), (-1, -40)),
                    cb((-5, 0), (-12, 0)),
                    cb((1, 0), (21, -50)),
                ] {
                    let got = ctx.sqrt(&z).unwrap().value();
                    let want = hi.sqrt(&retag_c(&z)).unwrap().value();
                    assert_rounds_from(
                        &got,
                        &want,
                        p,
                        &format!("{} sqrt p={p}", stringify!($mode)),
                    );
                }
            }
        }};
    }
    check!(HalfEven);
    check!(HalfAway);
    check!(Down);
    check!(Up);
}

/// On the branch cut the real part of the principal root is `+0`, whatever the sign of the
/// imaginary zero: `√(−x − 0i) = +0 − i√x = conj(√(−x + 0i))`.
#[test]
fn sqrt_of_negative_real_with_signed_zero_imaginary() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        for x in [1i64, 4, 2, 9] {
            let re: FBig<HalfEven, 2> = fb(-x, 0);
            let above = ctx
                .sqrt(&CBig::from_parts(re.clone(), zero(false)))
                .unwrap()
                .value();
            let below = ctx.sqrt(&CBig::from_parts(re, zero(true))).unwrap().value();
            assert!(above.re().is_pos_zero(), "sqrt(-{x} + 0i) real part");
            assert!(below.re().is_pos_zero(), "sqrt(-{x} - 0i) real part");
            assert!(below == above.conj());
            assert_eq!(below.im().sign(), dashu_base::Sign::Negative);
            assert_eq!(above.im().sign(), dashu_base::Sign::Positive);
        }
        // off the cut the imaginary zero keeps its sign: √(4 − 0i) = 2 − 0i
        let v = ctx
            .sqrt(&CBig::from_parts(fb(4, 0), zero(true)))
            .unwrap()
            .value();
        assert_eq!(v.re(), fb::<HalfEven, 2>(2, 0).repr());
        assert!(v.im().is_neg_zero());
    }
}
