//! Fixed regression cases for the complex hyperbolic functions and their inverses: correct
//! rounding under the directed modes (the family is derived from the circular functions, and a
//! negated part must not have its rounding direction reversed), and the branch cuts.

mod helpers;

use dashu_base::Sign;
use dashu_cmplx::round::mode::{Down, HalfEven, Up, Zero};
use dashu_cmplx::{CBig, Context, FBig, Rounding};
use dashu_float::Context as FloatContext;
use helpers::{assert_rounds_from, cb, fb, flags, retag_c, zero};

const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

type C = CBig<HalfEven, 2>;
type F = FBig<HalfEven, 2>;

fn sign_of(negative: bool) -> Sign {
    if negative {
        Sign::Negative
    } else {
        Sign::Positive
    }
}

/// `sinh(i) = i·sin(1)`: under `Down` the imaginary part is `sin 1` rounded *down*, under `Up`
/// rounded up, and it is inexact either way.
#[test]
fn sinh_of_imaginary_unit_rounds_in_the_mode_direction() {
    // sin(1) = 0.110101110110…₂
    let r = Context::<Down>::new(8)
        .sinh(&cb((0, 0), (1, 0)), None)
        .unwrap();
    assert_eq!(flags(&r), Some((Rounding::NoOp, Rounding::NoOp)));
    let v = r.value();
    assert!(v.re().is_pos_zero());
    assert_eq!(v.im(), fb::<Down, 2>(215, -8).repr());
    let r = Context::<Up>::new(8)
        .sinh(&cb((0, 0), (1, 0)), None)
        .unwrap();
    assert_eq!(flags(&r), Some((Rounding::NoOp, Rounding::AddOne)));
    assert_eq!(r.value().im(), fb::<Up, 2>(27, -5).repr());

    for p in PRECISIONS {
        for y in [(1i64, 0isize), (-1, 0), (5, -2), (-7, -1)] {
            macro_rules! check {
                ($mode:ty) => {{
                    let yr: FBig<$mode, 2> = fb(y.0, y.1);
                    let sin = FloatContext::<$mode>::new(p)
                        .sin(yr.repr(), None)
                        .unwrap()
                        .value();
                    let v = Context::<$mode>::new(p)
                        .sinh(&cb::<$mode, 2>((0, 0), y), None)
                        .unwrap()
                        .value();
                    assert_eq!(v.im(), sin.repr(), "{} Im sinh(i{y:?}) p={p}", stringify!($mode));
                    // cosh(iy) = cos y, tanh(iy) = i·tan y
                    let cos = FloatContext::<$mode>::new(p)
                        .cos(yr.repr(), None)
                        .unwrap()
                        .value();
                    let v = Context::<$mode>::new(p)
                        .cosh(&cb::<$mode, 2>((0, 0), y), None)
                        .unwrap()
                        .value();
                    assert_eq!(v.re(), cos.repr(), "{} Re cosh(i{y:?}) p={p}", stringify!($mode));
                    let tan = FloatContext::<$mode>::new(p)
                        .tan(yr.repr(), None)
                        .unwrap()
                        .value();
                    let v = Context::<$mode>::new(p)
                        .tanh(&cb::<$mode, 2>((0, 0), y), None)
                        .unwrap()
                        .value();
                    assert_eq!(v.im(), tan.repr(), "{} Im tanh(i{y:?}) p={p}", stringify!($mode));
                }};
            }
            check!(Down);
            check!(Up);
            check!(Zero);
            check!(HalfEven);
        }
    }
}

/// Every hyperbolic function and inverse, in every quadrant and under every directed mode, is
/// the higher-precision value rounded in the mode's direction.
#[test]
fn hyperbolic_functions_directed_match_oracle() {
    macro_rules! check {
        ($mode:ty) => {{
            for p in PRECISIONS {
                let ctx = Context::<$mode>::new(p);
                let hi = Context::<HalfEven>::new(2 * p + 100);
                for z in [
                    cb::<$mode, 2>((5, -3), (7, -4)),
                    cb((-5, -3), (7, -4)),
                    cb((-5, -3), (-7, -4)),
                    cb((5, -3), (-7, -4)),
                    cb((3, 0), (-2, 0)),
                    cb((-3, 1), (5, 0)),
                ] {
                    let zh = retag_c(&z);
                    let name = |f: &str| format!("{} {f}({z:?}) p={p}", stringify!($mode));
                    assert_rounds_from(
                        &ctx.sinh(&z, None).unwrap().value(),
                        &hi.sinh(&zh, None).unwrap().value(),
                        p,
                        &name("sinh"),
                    );
                    assert_rounds_from(
                        &ctx.cosh(&z, None).unwrap().value(),
                        &hi.cosh(&zh, None).unwrap().value(),
                        p,
                        &name("cosh"),
                    );
                    let (s, c) = ctx.sinh_cosh(&z, None);
                    assert_rounds_from(
                        &s.unwrap().value(),
                        &hi.sinh(&zh, None).unwrap().value(),
                        p,
                        &name("sinh_cosh.0"),
                    );
                    assert_rounds_from(
                        &c.unwrap().value(),
                        &hi.cosh(&zh, None).unwrap().value(),
                        p,
                        &name("sinh_cosh.1"),
                    );
                    assert_rounds_from(
                        &ctx.tanh(&z, None).unwrap().value(),
                        &hi.tanh(&zh, None).unwrap().value(),
                        p,
                        &name("tanh"),
                    );
                    assert_rounds_from(
                        &ctx.asinh(&z, None).unwrap().value(),
                        &hi.asinh(&zh, None).unwrap().value(),
                        p,
                        &name("asinh"),
                    );
                    assert_rounds_from(
                        &ctx.acosh(&z, None).unwrap().value(),
                        &hi.acosh(&zh, None).unwrap().value(),
                        p,
                        &name("acosh"),
                    );
                    assert_rounds_from(
                        &ctx.atanh(&z, None).unwrap().value(),
                        &hi.atanh(&zh, None).unwrap().value(),
                        p,
                        &name("atanh"),
                    );
                }
            }
        }};
    }
    check!(Down);
    check!(Up);
    check!(Zero);
}

/// The nearest-mode hyperbolic results are the textbook rotations of the circular ones
/// (`sinh z = −i·sin(iz)`, …), which is exact under a sign-symmetric mode — an independent
/// check of the part-swap formulation.
#[test]
fn hyperbolic_functions_are_rotations_of_the_circular_ones() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        for z in [
            cb::<HalfEven, 2>((5, -3), (7, -4)),
            cb((-5, -3), (7, -4)),
            cb((-5, -3), (-7, -4)),
            cb((5, -3), (-7, -4)),
            cb((7, 0), (1, -5)),
        ] {
            let iz = z.mul_i(false);
            let rot = |w: C| w.mul_i(true); // −i·w
            assert!(
                ctx.sinh(&z, None).unwrap().value() == rot(ctx.sin(&iz, None).unwrap().value())
            );
            assert!(ctx.cosh(&z, None).unwrap().value() == ctx.cos(&iz, None).unwrap().value());
            assert!(
                ctx.tanh(&z, None).unwrap().value() == rot(ctx.tan(&iz, None).unwrap().value())
            );
            assert!(
                ctx.asinh(&z, None).unwrap().value() == rot(ctx.asin(&iz, None).unwrap().value())
            );
            assert!(
                ctx.atanh(&z, None).unwrap().value() == rot(ctx.atan(&iz, None).unwrap().value())
            );
            let (s, c) = ctx.sinh_cosh(&z, None);
            assert!(s.unwrap().value() == ctx.sinh(&z, None).unwrap().value());
            assert!(c.unwrap().value() == ctx.cosh(&z, None).unwrap().value());
            // acosh z = ±i·acos z, the sign following Im z
            let acos = ctx.acos(&z, None).unwrap().value();
            let want = acos.mul_i(z.im().sign() == Sign::Negative);
            assert!(ctx.acosh(&z, None).unwrap().value() == want);
        }
    }
}

/// `asinh` on its cuts (the imaginary axis outside `[−i, i]`): the real part takes the sign of
/// the real zero. `asinh(±0 + iy) = ±acosh|y| + i·sign(y)·π/2` for `|y| > 1`, and
/// `asinh(±0 + iy) = ±0 + i·asin y` inside.
#[test]
fn asinh_on_the_imaginary_axis() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        let fctx = FloatContext::<HalfEven>::new(p);
        let half_pi = fctx
            .asin(fb::<HalfEven, 2>(1, 0).repr(), None)
            .unwrap()
            .value();
        for y in [(2i64, 0isize), (-2, 0), (5, -1)] {
            let acosh = fctx
                .acosh(fb::<HalfEven, 2>(y.0.abs(), y.1).repr(), None)
                .unwrap()
                .value();
            for re_neg in [false, true] {
                let z = C::from_parts(zero(re_neg), fb(y.0, y.1));
                let v = ctx.asinh(&z, None).unwrap().value();
                let want_re = if re_neg {
                    -acosh.clone()
                } else {
                    acosh.clone()
                };
                assert_eq!(v.re(), want_re.repr(), "Re asinh({re_neg} + i{y:?}) p={p}");
                let want_im = if y.0 < 0 {
                    -half_pi.clone()
                } else {
                    half_pi.clone()
                };
                assert_eq!(v.im(), want_im.repr(), "Im asinh({re_neg} + i{y:?}) p={p}");
            }
        }
        for y in [(1i64, 0isize), (-1, 0), (1, -1)] {
            let asin = fctx
                .asin(fb::<HalfEven, 2>(y.0, y.1).repr(), None)
                .unwrap()
                .value();
            for re_neg in [false, true] {
                let z = C::from_parts(zero(re_neg), fb(y.0, y.1));
                let v = ctx.asinh(&z, None).unwrap().value();
                assert!(v.re().significand().is_zero());
                assert_eq!(v.re().sign(), sign_of(re_neg), "Re asinh({re_neg} + i{y:?}) zero sign");
                assert_eq!(v.im(), asin.repr());
            }
        }
    }
}

/// `acosh(x ± 0i)` for a real `x`: `acosh x ± 0i` for `x > 1` (never a negative real part),
/// `±i·acos x` on the cut `[−1, 1]`, `acosh|x| ± iπ` for `x < −1`.
#[test]
fn acosh_of_real_argument() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        let fctx = FloatContext::<HalfEven>::new(p);
        let pi = fctx.pi::<2>(None).value();
        for x in [(3i64, -1isize), (3, 0), (1025, -10)] {
            let xr: F = fb(x.0, x.1);
            let acosh = fctx.acosh(xr.repr(), None).unwrap().value();
            for im_neg in [false, true] {
                let v = ctx
                    .acosh(&C::from_parts(xr.clone(), zero(im_neg)), None)
                    .unwrap()
                    .value();
                assert_eq!(v.re(), acosh.repr(), "Re acosh({x:?} {im_neg}) p={p}");
                assert!(v.im().significand().is_zero());
                assert_eq!(v.im().sign(), sign_of(im_neg));
                // x < −1
                let v = ctx
                    .acosh(&C::from_parts(-xr.clone(), zero(im_neg)), None)
                    .unwrap()
                    .value();
                assert_eq!(v.re(), acosh.repr(), "Re acosh(-{x:?} {im_neg}) p={p}");
                let want_im = if im_neg { -pi.clone() } else { pi.clone() };
                assert_eq!(v.im(), want_im.repr());
            }
        }
        for x in [(1i64, -1isize), (-1, -1), (0, 0), (1, 0)] {
            let xr: F = fb(x.0, x.1);
            let acos = fctx.acos(xr.repr(), None).unwrap().value();
            for im_neg in [false, true] {
                let v = ctx
                    .acosh(&C::from_parts(xr.clone(), zero(im_neg)), None)
                    .unwrap()
                    .value();
                assert!(v.re().is_pos_zero(), "Re acosh({x:?} {im_neg}) p={p}");
                let want_im = if im_neg { -acos.clone() } else { acos.clone() };
                assert_eq!(v.im(), want_im.repr(), "Im acosh({x:?} {im_neg}) p={p}");
            }
        }
        // the conjugate symmetry holds across the whole axis
        for x in [(3i64, -1isize), (-3, -1), (1, -1)] {
            let above = ctx
                .acosh(&C::from_parts(fb(x.0, x.1), zero(false)), None)
                .unwrap()
                .value();
            let below = ctx
                .acosh(&C::from_parts(fb(x.0, x.1), zero(true)), None)
                .unwrap()
                .value();
            assert!(below == above.conj());
        }
    }
    // the reported case through the convenience layer
    let mk = |v: f64| F::try_from(v).unwrap();
    let z = C::from_parts(mk(1.5), mk(0.0));
    assert_eq!(z.acosh().re().sign(), Sign::Positive);
    assert_eq!(z.acosh().re(), mk(1.5).acosh().repr());
    // and `acos` itself: acos(1.5 + 0i) = 0 − i·acosh(1.5)
    assert_eq!(z.acos().im(), (-mk(1.5).acosh()).repr());
}

/// `atanh` on its cuts (the real axis outside `[−1, 1]`): the imaginary part `±π/2` takes the
/// sign of the imaginary zero.
#[test]
fn atanh_on_the_real_axis_cuts() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        let half_pi = FloatContext::<HalfEven>::new(p)
            .asin(fb::<HalfEven, 2>(1, 0).repr(), None)
            .unwrap()
            .value();
        for x in [(2i64, 0isize), (-2, 0), (9, -3)] {
            for im_neg in [false, true] {
                let v = ctx
                    .atanh(&C::from_parts(fb(x.0, x.1), zero(im_neg)), None)
                    .unwrap()
                    .value();
                let want = if im_neg {
                    -half_pi.clone()
                } else {
                    half_pi.clone()
                };
                assert_eq!(v.im(), want.repr(), "Im atanh({x:?} {im_neg}) p={p}");
                assert_eq!(v.re().sign(), sign_of(x.0 < 0));
            }
        }
    }
}

/// In an odd base the midpoint of two floats has no finite expansion, and the driver's
/// containment test used to stop short of it: a part within `3^-129` ulps of a midpoint was
/// never certified and the call did not return. `asinh(3^-100)` at 255 digits is such a part
/// (`x − x³/6` is an exact midpoint there), on the real axis and — as `i·asin` — on the
/// imaginary one, and off the axes the real part keeps hugging it.
#[test]
fn asinh_near_tie_in_odd_base() {
    type C3 = CBig<HalfEven, 3>;
    type F3 = FBig<HalfEven, 3>;
    let x = F3::from_parts(1.into(), -100).with_precision(255).value();
    let zero = F3::ZERO.with_precision(255).value();
    let w = C3::from_parts(x.clone(), zero.clone()).asinh();
    assert_eq!(w.re(), x.asinh().repr());
    let w = C3::from_parts(zero, x.clone()).asinh();
    assert_eq!(w.im(), x.asin().repr());
    // a tiny imaginary part moves the real part by far less than the distance to the midpoint
    let tiny = F3::from_parts(1.into(), -400).with_precision(255).value();
    let w = C3::from_parts(x.clone(), tiny).asinh();
    assert_eq!(w.re(), x.asinh().repr());
}
