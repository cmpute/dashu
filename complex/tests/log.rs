//! Fixed regression cases for the complex logarithm: the rounding of `±π` at a negative zero
//! and correct rounding of `ln|z|` next to the unit circle.
// Several tests here need exponents beyond 32 bits and are compiled for 64-bit targets only;
// their helpers and imports are then unused elsewhere.
#![cfg_attr(not(target_pointer_width = "64"), allow(unused_imports, dead_code))]

mod helpers;

use dashu_cmplx::round::mode::{Away, Down, HalfEven, Up, Zero};
use dashu_cmplx::{CBig, Context, FBig};
use dashu_float::round::ErrorBounds;
use dashu_int::Word;
use helpers::{assert_rounds_from, cb, fb, flags, reround, retag_c, zero};

const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

/// `log(−0 ± 0i) = −∞ ± iπ`, with `±π` rounded in the mode's own direction: `−π` under `Down`
/// is the neighbour *below* `−π` (not the negation of `π` rounded down).
#[test]
fn log_of_negative_zero_rounds_pi_in_the_mode_direction() {
    // π = 804.2477…/256
    macro_rules! check {
        ($mode:ty, $pos:expr, $neg:expr) => {{
            let ctx = Context::<$mode>::new(10);
            let l = ctx
                .log(&CBig::<$mode, 2>::from_parts(zero(true), zero(false)), None)
                .unwrap()
                .value();
            assert!(l.re().is_infinite());
            assert_eq!(l.im(), fb::<$mode, 2>($pos, -8).repr(), "{} +pi", stringify!($mode));
            let l = ctx
                .log(&CBig::<$mode, 2>::from_parts(zero(true), zero(true)), None)
                .unwrap()
                .value();
            assert_eq!(l.im(), fb::<$mode, 2>($neg, -8).repr(), "{} -pi", stringify!($mode));
        }};
    }
    check!(Down, 804, -805);
    check!(Up, 805, -804);
    check!(Zero, 804, -804);
    check!(Away, 805, -805);
    check!(HalfEven, 804, -804);

    // and it agrees with the nonzero neighbour −1 ∓ εi, at every precision
    for p in PRECISIONS {
        macro_rules! neighbour {
            ($mode:ty) => {{
                let ctx = Context::<$mode>::new(p);
                for im_neg in [false, true] {
                    let at_zero = ctx
                        .log(&CBig::<$mode, 2>::from_parts(zero(true), zero(im_neg)), None)
                        .unwrap()
                        .value();
                    let eps = if im_neg { -1 } else { 1 };
                    let near = ctx
                        .log(&cb::<$mode, 2>((-1, 0), (eps, -(4 * p as isize))), None)
                        .unwrap()
                        .value();
                    // arg(−1 ± εi) = ±(π − ε): it rounds like ±π except toward zero from
                    // exactly-below, which is the same neighbour for a non-representable π
                    assert_eq!(at_zero.im(), near.im(), "{} p={p}", stringify!($mode));
                }
            }};
        }
        neighbour!(Down);
        neighbour!(Up);
        neighbour!(Zero);
        neighbour!(Away);
    }
}

/// `Re log z = ln|z|` when `|z|` is within a hair of 1: the modulus rounds to 1 (or to
/// `1 + ulp`) at any moderate working precision, and its logarithm is then pure rounding
/// noise unless the driver keeps refining.
#[test]
fn log_near_unit_circle_is_correctly_rounded() {
    // (precision, re, im, expected real part) — expected values from an exact ln(1+t) series
    for (p, re, im, want) in [
        (33usize, (1i64, -15isize), (1i64, 0isize), (2147483647i64, -62isize)),
        (33, (29, -20), (1, 0), (7054819325, -64)),
        (34, (1, 0), (81, -22), (13759414269, -66)),
        (31, (1, 0), (232471777, -56), (1610610697, -88)),
        (33, (1, -15), (1, 1), (5954088945, -33)),
        (33, (1, -2), (1, 0), (8332184577, -38)),
    ] {
        let z: CBig<HalfEven, 2> = cb(re, im);
        let l = Context::<HalfEven>::new(p).log(&z, None).unwrap().value();
        assert_eq!(
            l.re(),
            fb::<HalfEven, 2>(want.0, want.1).repr(),
            "log({re:?} + i{im:?}) at p={p}"
        );
    }

    // the same shape across the precision sweep and the directed modes, against a much higher
    // precision
    macro_rules! check {
        ($mode:ty) => {{
            for p in PRECISIONS {
                let ctx = Context::<$mode>::new(p);
                let hi = Context::<HalfEven>::new(3 * p + 200);
                for z in [
                    cb::<$mode, 2>((1, 0), (1, -(p as isize) / 2 - 3)),
                    cb((1, -(p as isize) - 7), (-1, 0)),
                    cb((-1, 0), (3, -(p as isize))),
                ] {
                    let got = ctx.log(&z, None).unwrap().value();
                    let want = hi.log(&retag_c(&z), None).unwrap().value();
                    assert_rounds_from(&got, &want, p, &format!("{} log p={p}", stringify!($mode)));
                }
            }
        }};
    }
    check!(HalfEven);
    check!(Down);
    check!(Up);

    // base 10 has exact points on the unit circle: |0.6 + 0.8i| = 1, so the real part is an
    // exact zero
    let z: CBig<HalfEven, 10> = cb((6, -1), (8, -1));
    let l = Context::<HalfEven>::new(30).log(&z, None).unwrap().value();
    assert!(l.re().is_pos_zero());
}

/// `log(1 + iy)` for a tiny `y` is `y²/2 + iy` displaced by far less than the precision shows:
/// both parts are rounded from their closed forms, at a cost that does not grow with the
/// exponent of `y`. (The composition `ln(hypot(1, y))` used to need a working precision of
/// twice that exponent: no answer within minutes at `y = 2^-(2^20)`.)
#[test]
#[cfg(target_pointer_width = "64")]
fn log_next_to_one() {
    // against the same call at a precision that resolves the displacement
    fn against_oracle<R: ErrorBounds, const B: Word>(p: usize, z: CBig<R, B>, depth: usize) {
        let hi = Context::<HalfEven>::new(p + 4 * depth + 80)
            .log(&retag_c(&z), None)
            .unwrap()
            .value();
        let got = Context::<R>::new(p).log(&z, None).unwrap();
        assert!(flags(&got).is_some(), "log({z:?}) at {p}: tagged exact");
        assert_rounds_from(&got.value(), &hi, p, &format!("log({z:?}) at {p}"));
    }
    // `v·(1 − ε)` rounded once: the closed form `v` displaced towards zero
    fn below<R: ErrorBounds, const B: Word>(v: FBig<R, B>, p: usize) -> FBig<R, B> {
        reround(&(v * (fb::<R, B>(1, 0) - fb::<R, B>(1, -(p as isize) - 5))), p)
    }
    fn check<R: ErrorBounds>(p: usize) {
        let ctx = Context::<R>::new(p);
        let k = (p + 40) as isize;
        for z in [
            cb::<R, 2>((1, 0), (1, -k)),
            cb((1, 0), (-1, -k)),
            cb((-1, 0), (1, -k)),
            cb((-1, 0), (-3, -k)),
            cb((1, -k), (1, 0)),
            cb((-5, -k), (-1, 0)),
            cb((1, 0), (7, -k / 2)),
            cb((1, 0), (1, -k / 4)),
            cb((3, -2), (1, -k)), // |z|² = 9/16: off the unit circle, still near it
            cb((1, 1), (1, -k)),  // |z|² = 4: the general path
        ] {
            against_oracle(p, z, k as usize);
        }
        // decimal: s/2 = 5·10^(−2k−1) is representable; base 3: it is not, and needs no
        // closed form
        let k10 = (p + 10) as isize;
        against_oracle(p, cb::<R, 10>((1, 0), (1, -k10)), k10 as usize);
        against_oracle(p, cb::<R, 10>((-1, 0), (-3, -k10)), k10 as usize);
        against_oracle(p, cb::<R, 3>((1, 0), (1, -k10)), k10 as usize);
        against_oracle(p, cb::<R, 3>((1, -k10), (-1, 0)), k10 as usize);

        // far beyond what any working precision could resolve
        for e in [1isize << 20, 10_000_000, 1 << 40, (1 << 61) - 7] {
            for y in [1i64, -1, 3] {
                // log(1 + iy·2^-e) = y²·2^(−2e−1)·(1 − …) + i·y·2^-e·(1 − …)
                let got = ctx.log(&cb::<R, 2>((1, 0), (y, -e)), None).unwrap();
                assert!(flags(&got).is_some());
                let got = got.value();
                assert_eq!(got.re(), below(fb::<R, 2>(y * y, -2 * e - 1), p).repr(), "e={e} p={p}");
                assert_eq!(got.im(), below(fb::<R, 2>(y, -e), p).repr(), "e={e} p={p}");
                // the same modulus on the other axes
                let got = ctx
                    .log(&cb::<R, 2>((-1, 0), (y, -e)), None)
                    .unwrap()
                    .value();
                assert_eq!(got.re(), below(fb::<R, 2>(y * y, -2 * e - 1), p).repr(), "e={e} p={p}");
                let got = ctx
                    .log(&cb::<R, 2>((y, -e), (-1, 0)), None)
                    .unwrap()
                    .value();
                assert_eq!(got.re(), below(fb::<R, 2>(y * y, -2 * e - 1), p).repr(), "e={e} p={p}");
            }
            let got = ctx
                .log(&cb::<R, 10>((1, 0), (1, -e)), None)
                .unwrap()
                .value();
            assert_eq!(got.re(), below(fb::<R, 10>(5, -2 * e - 1), p).repr(), "decimal e={e}");
            assert_eq!(got.im(), below(fb::<R, 10>(1, -e), p).repr(), "decimal e={e}");
        }

        // |z| = 1 exactly: the real part is +0 under every mode
        for z in [
            cb::<R, 2>((0, 0), (1, 0)),
            cb((-1, 0), (0, 0)),
            cb((0, 0), (-1, 0)),
        ] {
            let got = ctx.log(&z, None).unwrap().value();
            assert!(got.re().is_pos_zero(), "log({z:?}) at {p}");
        }
        // y² below the exponent range: the real part underflows on its own
        let e = (1isize << 62) + 3;
        let got = ctx
            .log(&cb::<R, 2>((1, 0), (-1, -e)), None)
            .unwrap()
            .value();
        assert!(got.re().is_pos_zero());
        assert_eq!(got.im(), below(fb::<R, 2>(-1, -e), p).repr());
    }
    for p in PRECISIONS {
        check::<HalfEven>(p);
        check::<Down>(p);
        check::<Up>(p);
        check::<Zero>(p);
        check::<Away>(p);
    }

    // the reported case: log(1 − i·2^-(2^20)) at 64 bits under `Down`
    let e = 1isize << 20;
    let z: CBig<Down, 2> = cb((1, 0), (-1, -e));
    let (re, im) = Context::<Down>::new(64)
        .log(&z, None)
        .unwrap()
        .value()
        .into_parts();
    let want: FBig<Down, 2> = FBig::from_parts(((1u128 << 64) - 1).into(), -(2 * e + 1) - 64);
    assert_eq!(re, want);
    assert_eq!(im, fb::<Down, 2>(-1, -e));
}

/// In an odd base `ln|1 + i·3^-k| = 3^(−2k)/2 − …` is within `3^(−2k)` (relative) of a rounding
/// tie, because `1/2 = 0.111…₃`. The inner half-ulp bound of an odd base (`half_ulp` in
/// `float/src/round.rs`) stops about 129 digits short of the tie in base 3, so a value closer
/// than that needs the exact-endpoint fallback of the Ziv containment test. Shallow cases never
/// reach it:
#[test]
fn log_next_to_one_odd_base_nearest_shallow() {
    for (p, k) in [(20usize, 30isize), (50, 60), (100, 110)] {
        let z: CBig<HalfEven, 3> = cb((1, 0), (1, -k));
        let hi = Context::<HalfEven>::new(p + 4 * k as usize + 80)
            .log(&z, None)
            .unwrap()
            .value();
        let got = Context::<HalfEven>::new(p).log(&z, None).unwrap().value();
        assert_rounds_from(&got, &hi, p, &format!("base 3, p={p}"));
    }
}

/// … and a deep one does (at 500 digits the value is 3^-520 ulps from the tie). Regression:
/// without the fallback this call did not return.
#[test]
fn log_next_to_one_odd_base_nearest() {
    let z: CBig<HalfEven, 3> = cb((1, 0), (1, -510));
    let got = Context::<HalfEven>::new(500).log(&z, None).unwrap().value();
    let hi = Context::<Down>::new(2700)
        .log(&retag_c(&z), None)
        .unwrap()
        .value();
    assert_rounds_from(&got, &hi, 500, "base 3, p=500");
}
