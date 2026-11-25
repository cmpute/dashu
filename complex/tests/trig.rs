//! Fixed regression cases for the complex circular functions and their inverses: the axes and
//! branch cuts (exact zeros, signed zeros), saturation of `tan`, the ×π family near its poles,
//! and correct rounding under the directed modes.

mod helpers;

use core::str::FromStr;
use dashu_base::Sign;
use dashu_cmplx::round::mode::{Down, HalfEven, Up, Zero};
use dashu_cmplx::{CBig, Context, FBig, Rounding};
use dashu_float::Context as FloatContext;
use helpers::{assert_rounds_from, cb, fb, flags, retag_c, zero};

const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

type C = CBig<HalfEven, 2>;
type F = FBig<HalfEven, 2>;

fn c(re: (i64, isize), im: (i64, isize)) -> C {
    cb(re, im)
}

/// A real number with a signed zero imaginary part.
fn on_real_axis(x: (i64, isize), im_negative: bool) -> C {
    CBig::from_parts(fb(x.0, x.1), zero(im_negative))
}

/// A signed zero real part with an imaginary part.
fn on_imag_axis(re_negative: bool, y: (i64, isize)) -> C {
    CBig::from_parts(zero(re_negative), fb(y.0, y.1))
}

fn sign_of(negative: bool) -> Sign {
    if negative {
        Sign::Negative
    } else {
        Sign::Positive
    }
}

/// `asin` / `acos` of a real argument in `[−1, 1]` are the real kernels with an exactly-zero
/// imaginary part — returned at once (the general formula can never certify the exact zero and
/// used to retry forever).
#[test]
fn asin_acos_of_real_argument_inside_the_cut() {
    for p in [24usize, 20, 50, 53, 100, 500] {
        let ctx = Context::<HalfEven>::new(p);
        let fctx = FloatContext::<HalfEven>::new(p);
        for x in [
            (1i64, -1isize),
            (-1, -1),
            (3, -2),
            (-7, -3),
            (1, 0),
            (-1, 0),
            (1, -40),
        ] {
            for im_neg in [false, true] {
                let z = on_real_axis(x, im_neg);
                let xr: F = fb(x.0, x.1);
                let s = ctx.asin(&z, None).unwrap();
                let want = fctx.asin(xr.repr(), None).unwrap();
                assert_eq!(s.clone().value().re(), want.value().repr(), "asin({x:?}) p={p}");
                // asin(x ± 0i) = asin x ± 0i
                assert!(s.clone().value().im().significand().is_zero());
                assert_eq!(s.value().im().sign(), sign_of(im_neg), "asin({x:?}) zero sign");

                let a = ctx.acos(&z, None).unwrap();
                let want = fctx.acos(xr.repr(), None).unwrap();
                assert_eq!(a.clone().value().re(), want.value().repr(), "acos({x:?}) p={p}");
                // acos(x ± 0i) = acos x ∓ 0i
                assert!(a.clone().value().im().significand().is_zero());
                assert_eq!(a.value().im().sign(), sign_of(!im_neg), "acos({x:?}) zero sign");
            }
        }
    }
    // the convenience layer, as reported: (0.5 + 0i).asin() at 24 bits
    let half = F::from_parts(1.into(), -1).with_precision(24).value();
    let z = C::from_parts(half.clone(), F::ZERO.with_precision(24).value());
    assert_eq!(z.asin().re(), half.asin().repr());
    assert_eq!(z.acos().re(), half.acos().repr());
    // asinh of a purely imaginary argument takes the same route: asinh(i/2) = i·asin(1/2)
    let iz = C::from_parts(F::ZERO.with_precision(24).value(), half.clone());
    assert!(iz.asinh().re().significand().is_zero());
    assert_eq!(iz.asinh().im(), half.asin().repr());
}

/// `asin` / `acos` of a signed zero: `asin(±0 ± 0i) = ±0 ± 0i`, `acos(±0 ± 0i) = π/2 ∓ 0i`.
#[test]
fn asin_acos_of_signed_zero() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        let half_pi = FloatContext::<HalfEven>::new(p)
            .asin(fb::<HalfEven, 2>(1, 0).repr(), None)
            .unwrap()
            .value();
        for re_neg in [false, true] {
            for im_neg in [false, true] {
                let z = C::from_parts(zero(re_neg), zero(im_neg));
                let s = ctx.asin(&z, None).unwrap();
                assert_eq!(flags(&s), None);
                let s = s.value();
                assert!(s.is_zero());
                assert_eq!(s.re().sign(), sign_of(re_neg), "asin zero re sign");
                assert_eq!(s.im().sign(), sign_of(im_neg), "asin zero im sign");

                let a = ctx.acos(&z, None).unwrap().value();
                assert_eq!(a.re(), half_pi.repr());
                assert!(a.im().significand().is_zero());
                assert_eq!(a.im().sign(), sign_of(!im_neg), "acos zero im sign");
            }
        }
    }
}

/// `cos(±0 ± 0i) = 1 ∓ 0i`: the imaginary zero is `−sin x·sinh y`, negative iff the two zeros
/// are like-signed (Annex G: `ccos(z) = ccosh(iz)`), the sign the nonzero neighbours
/// `cos(±ε ± 0i)` and `cos(±0 ± εi)` have. The ×π family has the same table.
#[test]
fn cos_of_signed_zero() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        let one = fb::<HalfEven, 2>(1, 0);
        for re_neg in [false, true] {
            for im_neg in [false, true] {
                let z = C::from_parts(zero(re_neg), zero(im_neg));
                let want = sign_of(re_neg == im_neg);
                let what =
                    format!("cos(zeros: re negative {re_neg}, im negative {im_neg}) at p={p}");

                let (_, sc_cos) = ctx.sin_cos(&z, None);
                let (_, sc_cos_pi) = ctx.sin_cos_pi(&z, None);
                for r in [ctx.cos(&z, None), sc_cos, ctx.cos_pi(&z, None), sc_cos_pi] {
                    let r = r.unwrap();
                    assert_eq!(flags(&r), None, "{what}");
                    let r = r.value();
                    assert_eq!(r.re(), one.repr(), "{what}");
                    assert!(r.im().significand().is_zero(), "{what}");
                    assert_eq!(r.im().sign(), want, "{what}");
                }
                assert_eq!(z.cos().im().sign(), want, "{what}");

                // cos z = cosh(iz), with iz = −y + ix
                let iz = C::from_parts(zero(!im_neg), zero(re_neg));
                let cosh = ctx.cosh(&iz, None).unwrap().value();
                assert_eq!(cosh.im().sign(), want, "{what} against cosh");

                // the neighbours with one zero replaced by a tiny number of the same sign
                let tiny = |negative: bool| fb::<HalfEven, 2>(if negative { -1 } else { 1 }, -300);
                for near in [
                    C::from_parts(tiny(re_neg), zero(im_neg)),
                    C::from_parts(zero(re_neg), tiny(im_neg)),
                ] {
                    for im in [
                        ctx.cos(&near, None).unwrap().value().into_parts().1,
                        ctx.cos_pi(&near, None).unwrap().value().into_parts().1,
                    ] {
                        assert!(im.repr().significand().is_zero(), "{what}: neighbour");
                        assert_eq!(im.repr().sign(), want, "{what}: neighbour");
                    }
                }
            }
        }
    }
}

/// On the cuts (`|x| > 1` on the real axis) the sign of the imaginary zero selects the side:
/// `asin(x ± 0i) = sign(x)·π/2 ± i·acosh|x|`, `acos(x ± 0i) = (0 or π) ∓ i·acosh|x|`.
#[test]
fn asin_acos_on_the_branch_cuts() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        let fctx = FloatContext::<HalfEven>::new(p);
        let half_pi = fctx
            .asin(fb::<HalfEven, 2>(1, 0).repr(), None)
            .unwrap()
            .value();
        let pi = fctx.pi::<2>(None).value();
        for x in [(2i64, 0isize), (3, -1), (-2, 0), (-5, 0), (1025, -10)] {
            let ax: F = fb(x.0.abs(), x.1);
            let acosh = fctx.acosh(ax.repr(), None).unwrap().value();
            for im_neg in [false, true] {
                let z = on_real_axis(x, im_neg);
                let s = ctx.asin(&z, None).unwrap().value();
                let want_re = if x.0 < 0 {
                    -half_pi.clone()
                } else {
                    half_pi.clone()
                };
                assert_eq!(s.re(), want_re.repr(), "Re asin({x:?})");
                let want_im = if im_neg {
                    -acosh.clone()
                } else {
                    acosh.clone()
                };
                assert_eq!(s.im(), want_im.repr(), "Im asin({x:?} {im_neg})");

                let a = ctx.acos(&z, None).unwrap().value();
                if x.0 < 0 {
                    assert_eq!(a.re(), pi.repr(), "Re acos({x:?})");
                } else {
                    assert!(a.re().is_pos_zero(), "Re acos({x:?})");
                }
                let want_im = if im_neg {
                    acosh.clone()
                } else {
                    -acosh.clone()
                };
                assert_eq!(a.im(), want_im.repr(), "Im acos({x:?} {im_neg})");
            }
        }
        // the cut is continuous with the adjacent quadrant: asin(2 ± 0i) ≈ asin(2 ± εi)
        for im_neg in [false, true] {
            let eps = if im_neg { -1 } else { 1 };
            let on = ctx
                .asin(&on_real_axis((2, 0), im_neg), None)
                .unwrap()
                .value();
            let near = ctx
                .asin(&c((2, 0), (eps, -(4 * p as isize))), None)
                .unwrap()
                .value();
            assert_eq!(on.im(), near.im());
            let on = ctx
                .acos(&on_real_axis((2, 0), im_neg), None)
                .unwrap()
                .value();
            let near = ctx
                .acos(&c((2, 0), (eps, -(4 * p as isize))), None)
                .unwrap()
                .value();
            assert_eq!(on.im(), near.im());
        }
    }
}

/// On the imaginary axis `asin(±0 + iy) = ±0 + i·asinh y` and `acos(±0 + iy) = π/2 − i·asinh y`
/// for every finite `y` — a large `|y|` used to cancel `iz` against `√(1−z²)` down to zero and
/// surface as `Err(OutOfDomain)`.
#[test]
fn asin_acos_on_the_imaginary_axis_and_for_large_imaginary_parts() {
    for (p, y) in [
        (8usize, (8192i64, 0isize)),
        (8, (4096, 0)),
        (8, (-8192, 0)),
        (36, (8192, 0)),
        (53, (1, 35)),
        (53, (-1, 34)),
        (7, (255, -19)),
        (20, (1, 300)),
    ] {
        let ctx = Context::<HalfEven>::new(p);
        let fctx = FloatContext::<HalfEven>::new(p);
        let yr: F = fb(y.0, y.1);
        let asinh = fctx.asinh(yr.repr(), None).unwrap().value();
        for re_neg in [false, true] {
            let z = on_imag_axis(re_neg, y);
            let s = ctx.asin(&z, None).unwrap().value();
            assert!(s.re().significand().is_zero());
            assert_eq!(s.re().sign(), sign_of(re_neg));
            assert_eq!(s.im(), asinh.repr(), "asin(i{y:?}) at p={p}");
            let a = ctx.acos(&z, None).unwrap().value();
            assert_eq!(a.im(), (-asinh.clone()).repr(), "acos(i{y:?}) at p={p}");
        }
    }
    // asin(8192i) at 8 bits is 1001.1011₂·i; acos(i·255·2⁻¹⁹) at 7 bits has Im = −127·2⁻¹⁸
    let v = Context::<HalfEven>::new(8)
        .asin(&c((0, 0), (8192, 0)), None)
        .unwrap()
        .value();
    assert_eq!(v.im(), fb::<HalfEven, 2>(155, -4).repr());
    let v = Context::<HalfEven>::new(7)
        .acos(&c((0, 0), (255, -19)), None)
        .unwrap()
        .value();
    assert_eq!(v.re(), fb::<HalfEven, 2>(101, -6).repr());
    assert_eq!(v.im(), fb::<HalfEven, 2>(-127, -18).repr());

    // off the axis, in all four quadrants, with a huge imaginary part: correctly rounded
    // against a higher precision, under the nearest and the directed modes
    macro_rules! check {
        ($mode:ty) => {{
            for p in PRECISIONS {
                let ctx = Context::<$mode>::new(p);
                let hi = Context::<HalfEven>::new(2 * p + 200);
                for z in [
                    cb::<$mode, 2>((1, 0), (8192, 0)),
                    cb((1, 0), (-8192, 0)),
                    cb((-3, 0), (1, 60)),
                    cb((-3, 0), (-1, 60)),
                    cb((1, 40), (1, 40)),
                    cb((5, -3), (-7, -4)),
                ] {
                    let got = ctx.asin(&z, None).unwrap().value();
                    let want = hi.asin(&retag_c(&z), None).unwrap().value();
                    assert_rounds_from(
                        &got,
                        &want,
                        p,
                        &format!("{} asin({z:?}) p={p}", stringify!($mode)),
                    );
                    let got = ctx.acos(&z, None).unwrap().value();
                    let want = hi.acos(&retag_c(&z), None).unwrap().value();
                    assert_rounds_from(
                        &got,
                        &want,
                        p,
                        &format!("{} acos({z:?}) p={p}", stringify!($mode)),
                    );
                }
            }
        }};
    }
    check!(HalfEven);
    check!(Down);
    check!(Up);

    // the symmetries hold exactly under the (sign-symmetric) nearest mode
    let ctx = Context::<HalfEven>::new(53);
    for z in [
        c((1, 0), (8192, 0)),
        c((3, -2), (1, 35)),
        c((-7, 0), (5, -9)),
    ] {
        let s = ctx.asin(&z, None).unwrap().value();
        assert!(ctx.asin(&-z.clone(), None).unwrap().value() == -s.clone());
        assert!(ctx.asin(&z.conj(), None).unwrap().value() == s.conj());
        let a = ctx.acos(&z, None).unwrap().value();
        assert!(ctx.acos(&z.conj(), None).unwrap().value() == a.conj());
    }
}

/// An argument held at more digits than the context precision, a hair from the branch point
/// `1`, must not be rounded onto it: `asin(1 + 2⁻ᵏ)` is `π/2 + i·acosh(1 + 2⁻ᵏ)`.
#[test]
fn asin_acos_of_wide_argument_next_to_one() {
    for k in [4isize, 8, 30, 200] {
        let ctx = Context::<HalfEven>::new(10);
        let fctx = FloatContext::<HalfEven>::new(10);
        let x: F = fb(1, 0) + fb(1, -k); // exact: k + 1 bits
        let z = C::from_parts(x.clone(), zero(false));
        let acosh = fctx.acosh(x.repr(), None).unwrap().value();
        let half_pi = fctx
            .asin(fb::<HalfEven, 2>(1, 0).repr(), None)
            .unwrap()
            .value();
        let s = ctx.asin(&z, None).unwrap().value();
        assert_eq!(s.re(), half_pi.repr(), "k={k}");
        assert_eq!(s.im(), acosh.repr(), "k={k}");
        let a = ctx.acos(&z, None).unwrap().value();
        assert!(a.re().is_pos_zero(), "k={k}");
        assert_eq!(a.im(), (-acosh).repr(), "k={k}");
        // and just off the axis, where the general formula runs
        let z = C::from_parts(x, fb(1, -3 * k));
        assert!(ctx.asin(&z, None).is_ok(), "k={k}");
        assert!(ctx.acos(&z, None).is_ok(), "k={k}");
    }
}

/// Tiny base-10 arguments: `asin z = z + z³/6 + …` to full precision.
#[test]
fn asin_of_small_decimal_argument() {
    type C10 = CBig<HalfEven, 10>;
    type F10 = FBig<HalfEven, 10>;
    // asin(1e-40 + 1e-45i) at 90 digits: Re = x + x³/6, Im = y + x²y/2
    let z: C10 = cb((1, -40), (1, -45));
    let v = Context::<HalfEven>::new(90).asin(&z, None).unwrap().value();
    let want_re = F10::from_str("0.000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000166666667").unwrap();
    let want_im = F10::from_str("0.000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000005").unwrap();
    assert_eq!(v.re(), want_re.repr());
    assert_eq!(v.im(), want_im.repr());

    // asin(±605e-40 i) at 112 digits = ±i·asinh(6.05e-38), odd
    let want = F10::from_str("0.00000000000000000000000000000000000006049999999999999999999999999999999999999999999999999999999999999999999999996309247916666666666666666666666666667").unwrap();
    let ctx = Context::<HalfEven>::new(112);
    let v = ctx
        .asin(&cb::<HalfEven, 10>((0, 0), (605, -40)), None)
        .unwrap()
        .value();
    assert!(v.re().is_pos_zero());
    assert_eq!(v.im(), want.repr());
    let v = ctx
        .asin(&cb::<HalfEven, 10>((0, 0), (-605, -40)), None)
        .unwrap()
        .value();
    assert_eq!(v.im(), (-want).repr());

    // a larger argument of the same shape
    let z: C10 = cb((1, -10), (1, -15));
    let v = Context::<HalfEven>::new(30).asin(&z, None).unwrap().value();
    let want_im = F10::from_str("0.000000000000001000000000000000000005").unwrap();
    assert_eq!(v.im(), want_im.repr());
}

/// On the cuts of `atan` (`|y| > 1` on the imaginary axis) the real part is `±π/2` with the sign
/// of the real zero — the side the cut is approached from — and the imaginary part is
/// `atanh(1/y)`.
#[test]
fn atan_on_the_imaginary_axis_cuts() {
    for p in [10usize, 20, 50, 100, 500] {
        let ctx = Context::<HalfEven>::new(p);
        let fctx = FloatContext::<HalfEven>::new(p);
        let half_pi = fctx
            .asin(fb::<HalfEven, 2>(1, 0).repr(), None)
            .unwrap()
            .value();
        for y in [(2i64, 0isize), (-2, 0), (3, -1), (-1025, -10), (1, 70)] {
            for re_neg in [false, true] {
                let v = ctx.atan(&on_imag_axis(re_neg, y), None).unwrap().value();
                let want_re = if re_neg {
                    -half_pi.clone()
                } else {
                    half_pi.clone()
                };
                assert_eq!(v.re(), want_re.repr(), "Re atan({re_neg} + i{y:?}) p={p}");
                assert_eq!(v.im().sign(), sign_of(y.0 < 0), "Im atan sign");
                // continuous with the adjacent quadrant
                let eps = if re_neg { -1 } else { 1 };
                let near = ctx
                    .atan(&c((eps, -(4 * p as isize) - 100), y), None)
                    .unwrap()
                    .value();
                assert_eq!(v.im(), near.im(), "Im atan({re_neg} + i{y:?}) p={p}");
            }
        }
        // inside the cut the real part is the signed zero itself
        for re_neg in [false, true] {
            let v = ctx
                .atan(&on_imag_axis(re_neg, (1, -1)), None)
                .unwrap()
                .value();
            assert!(v.re().significand().is_zero());
            assert_eq!(v.re().sign(), sign_of(re_neg));
        }
    }
    // atan(+0 − 2i) at 10 bits: +π/2 − i·atanh(1/2) = 1.5703125 − 0.548828125i
    let v = Context::<HalfEven>::new(10)
        .atan(&on_imag_axis(false, (-2, 0)), None)
        .unwrap()
        .value();
    assert_eq!(v.re(), fb::<HalfEven, 2>(201, -7).repr());
    assert_eq!(v.im(), fb::<HalfEven, 2>(-281, -9).repr());
}

/// For a large `|Im z|` the imaginary part of `tan z` is within half an ulp of `±1` — on the
/// side given by the sign of `cos 2x + e^(−2|y|)`, so *above* 1 in magnitude when `cos 2x < 0`.
#[test]
fn tan_saturated_imaginary_part_lies_on_the_right_side_of_one() {
    // Im tan(1 + 3i) = 1.00205…, Im tan(3i) = tanh 3 = 0.99505…
    let one: (i64, isize) = (1, 0);
    let dn = Context::<Down>::new(2);
    let up = Context::<Up>::new(2);
    let v = dn.tan(&cb((1, 0), (3, 0)), None).unwrap().value();
    assert_eq!(v.im(), fb::<Down, 2>(one.0, one.1).repr());
    let v = up.tan(&cb((1, 0), (3, 0)), None).unwrap().value();
    assert_eq!(v.im(), fb::<Up, 2>(3, -1).repr());
    let v = dn.tan(&cb((0, 0), (3, 0)), None).unwrap().value();
    assert_eq!(v.im(), fb::<Down, 2>(3, -2).repr());
    let v = up.tan(&cb((0, 0), (3, 0)), None).unwrap().value();
    assert_eq!(v.im(), fb::<Up, 2>(1, 0).repr());
    // tanh(3 + i) = swap of tan(1 + 3i)
    let v = dn.tanh(&cb((3, 0), (1, 0)), None).unwrap().value();
    assert_eq!(v.re(), fb::<Down, 2>(1, 0).repr());
    // negative imaginary part mirrors: Im tan(1 − 3i) = −1.00205…
    let v = dn.tan(&cb((1, 0), (-3, 0)), None).unwrap().value();
    assert_eq!(v.im(), fb::<Down, 2>(-3, -1).repr());
    let v = up.tan(&cb((1, 0), (-3, 0)), None).unwrap().value();
    assert_eq!(v.im(), fb::<Up, 2>(-1, 0).repr());

    for p in PRECISIONS {
        let below: FBig<Down, 2> = fb(1, 0) - fb(1, -(p as isize)); // 1 − 2^−p
        let above: FBig<Up, 2> = fb(1, 0) + fb(1, 1 - p as isize); // 1 + 2^(1−p)
        let y = (p as i64 * 10, 0);
        // cos 2 < 0: just above 1
        let r = Context::<Down>::new(p).tan(&cb((1, 0), y), None).unwrap();
        assert_eq!(flags(&r).map(|f| f.1), Some(Rounding::NoOp));
        assert_eq!(r.value().im(), fb::<Down, 2>(1, 0).repr(), "Down above, p={p}");
        let r = Context::<Up>::new(p).tan(&cb((1, 0), y), None).unwrap();
        assert_eq!(flags(&r).map(|f| f.1), Some(Rounding::AddOne));
        assert_eq!(r.value().im(), above.repr(), "Up above, p={p}");
        let r = Context::<HalfEven>::new(p)
            .tan(&c((1, 0), y), None)
            .unwrap();
        assert_eq!(flags(&r).map(|f| f.1), Some(Rounding::NoOp));
        assert_eq!(r.value().im(), fb::<HalfEven, 2>(1, 0).repr(), "HalfEven above, p={p}");
        // cos 6 > 0: just below 1
        let r = Context::<Down>::new(p).tan(&cb((3, 0), y), None).unwrap();
        assert_eq!(r.value().im(), below.repr(), "Down below, p={p}");
        let r = Context::<Up>::new(p).tan(&cb((3, 0), y), None).unwrap();
        assert_eq!(flags(&r).map(|f| f.1), Some(Rounding::AddOne));
        assert_eq!(r.value().im(), fb::<Up, 2>(1, 0).repr(), "Up below, p={p}");
        let r = Context::<Zero>::new(p)
            .tan(&cb((1, 0), (-y.0, 0)), None)
            .unwrap();
        assert_eq!(r.value().im(), fb::<Zero, 2>(-1, 0).repr(), "Zero above, p={p}");
    }
    // the reported 53-bit case: Im tan(1 + 30i) under Down is 1, not 1 − 2⁻⁵³
    let v = Context::<Down>::new(53)
        .tan(&cb((1, 0), (30, 0)), None)
        .unwrap()
        .value();
    assert_eq!(v.im(), fb::<Down, 2>(1, 0).repr());
}

/// `tan_pi` saturates like `tan`: a large imaginary part returns promptly under every mode
/// (the directed modes used to climb to ~2π|y|/ln 2 working bits — minutes at |y| = 10⁵).
#[test]
fn tan_pi_with_large_imaginary_part() {
    for p in PRECISIONS {
        let below: FBig<Down, 2> = fb(1, 0) - fb(1, -(p as isize));
        let above: FBig<Up, 2> = fb(1, 0) + fb(1, 1 - p as isize);
        for y in [100000i64, 3000, p as i64] {
            // on the imaginary axis: tan_pi(iy) = i·tanh(πy), just below ±1
            let r = Context::<Down>::new(p)
                .tan_pi(&cb((0, 0), (y, 0)), None)
                .unwrap()
                .value();
            assert!(r.re().is_pos_zero());
            assert_eq!(r.im(), below.repr(), "Down tan_pi({y}i) p={p}");
            let r = Context::<Up>::new(p)
                .tan_pi(&cb((0, 0), (y, 0)), None)
                .unwrap()
                .value();
            assert_eq!(r.im(), fb::<Up, 2>(1, 0).repr(), "Up tan_pi({y}i) p={p}");
            let r = Context::<Zero>::new(p)
                .tan_pi(&cb((0, 0), (-y, 0)), None)
                .unwrap()
                .value();
            assert_eq!(r.im(), (-retag_below(&below)).repr(), "Zero tan_pi(-{y}i) p={p}");
            let r = Context::<HalfEven>::new(p)
                .tan_pi(&c((0, 0), (y, 0)), None)
                .unwrap()
                .value();
            assert_eq!(r.im(), fb::<HalfEven, 2>(1, 0).repr(), "HalfEven tan_pi({y}i) p={p}");
            // cos_pi(2·3/8) < 0: just above 1 in magnitude, with a tiny nonzero real part
            let r = Context::<Up>::new(p)
                .tan_pi(&cb((3, -3), (y, 0)), None)
                .unwrap()
                .value();
            assert_eq!(r.im(), above.repr(), "Up tan_pi(3/8 + {y}i) p={p}");
            assert!(!r.re().significand().is_zero());
            let r = Context::<Down>::new(p)
                .tan_pi(&cb((3, -3), (y, 0)), None)
                .unwrap()
                .value();
            assert_eq!(r.im(), fb::<Down, 2>(1, 0).repr(), "Down tan_pi(3/8 + {y}i) p={p}");
        }
    }

    fn retag_below(v: &FBig<Down, 2>) -> FBig<Zero, 2> {
        helpers::retag(v)
    }
}

/// `tan_pi` next to a real-axis pole with a tiny imaginary part is a huge finite value, not an
/// indeterminate form: `tan_pi(1/2 + iy) = i·coth(πy)`.
#[test]
fn tan_pi_next_to_a_pole() {
    for (p, k) in [
        (50usize, 44isize),
        (50, 60),
        (50, 30),
        (53, 44),
        (20, 44),
        (100, 300),
        (500, 44),
    ] {
        let z = c((1, -1), (1, -k));
        let r = Context::<HalfEven>::new(p).tan_pi(&z, None).unwrap();
        let v = r.value();
        assert!(v.re().is_pos_zero(), "Re tan_pi(1/2 + 2^-{k} i) at p={p}");
        let hi = Context::<HalfEven>::new(2 * p + 100)
            .tan_pi(&z, None)
            .unwrap()
            .value();
        assert_rounds_from(&v, &hi, p, &format!("tan_pi(1/2 + 2^-{k} i) at p={p}"));
        // ≈ 2^k/π
        let lo: F = fb(1, k - 2);
        let up: F = fb(1, k - 1);
        let im: F = helpers::retag(&v.into_parts().1);
        assert!(im > lo && im < up);
    }
    // the convenience layer on a 50-bit value does not panic
    let z = C::from_parts(
        F::from_parts(1.into(), -1).with_precision(50).value(),
        F::from_parts(1.into(), -44).with_precision(50).value(),
    );
    assert!(!z.tan_pi().im().significand().is_zero());
    // and under the directed modes
    let dn = Context::<Down>::new(50)
        .tan_pi(&cb((1, -1), (1, -44)), None)
        .unwrap()
        .value();
    let up = Context::<Up>::new(50)
        .tan_pi(&cb((1, -1), (1, -44)), None)
        .unwrap()
        .value();
    let (dn, up): (F, F) = (helpers::retag(&dn.into_parts().1), helpers::retag(&up.into_parts().1));
    assert!(dn < up);
}

/// On the real axis `tan_pi` keeps the real kernel's inexactness flag.
#[test]
fn tan_pi_on_the_real_axis_keeps_the_inexact_flag() {
    type C10 = CBig<HalfEven, 10>;
    let ctx = Context::<HalfEven>::new(5);
    let fctx = FloatContext::<HalfEven>::new(5);
    let x: FBig<HalfEven, 10> = fb(1, -1);
    let z = C10::from_parts(x.clone(), zero(false));
    let r = ctx.tan_pi(&z, None).unwrap();
    assert_eq!(flags(&r), Some((Rounding::AddOne, Rounding::NoOp)));
    assert_eq!(r.value().re(), fctx.tan_pi(x.repr(), None).unwrap().value().repr());
    // an exact value stays Exact: tan_pi(1/4) = 1
    let z = C10::from_parts(fb(25, -2), zero(false));
    let r = ctx.tan_pi(&z, None).unwrap();
    assert_eq!(flags(&r), None);
    assert_eq!(r.value().re(), fb::<HalfEven, 10>(1, 0).repr());
    for p in PRECISIONS {
        let z = C::from_parts(fb(3, -4), zero(true));
        let r = Context::<HalfEven>::new(p).tan_pi(&z, None).unwrap();
        assert!(flags(&r).is_some(), "tan(3π/16) is irrational, p={p}");
        assert!(r.value().im().is_neg_zero());
    }
}

/// The circular functions under the directed modes, against a higher-precision evaluation,
/// in every quadrant.
#[test]
fn circular_functions_directed_match_oracle() {
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
                ] {
                    let zh = retag_c(&z);
                    let name = |f: &str| format!("{} {f}({z:?}) p={p}", stringify!($mode));
                    assert_rounds_from(
                        &ctx.sin(&z, None).unwrap().value(),
                        &hi.sin(&zh, None).unwrap().value(),
                        p,
                        &name("sin"),
                    );
                    assert_rounds_from(
                        &ctx.cos(&z, None).unwrap().value(),
                        &hi.cos(&zh, None).unwrap().value(),
                        p,
                        &name("cos"),
                    );
                    assert_rounds_from(
                        &ctx.tan(&z, None).unwrap().value(),
                        &hi.tan(&zh, None).unwrap().value(),
                        p,
                        &name("tan"),
                    );
                    assert_rounds_from(
                        &ctx.atan(&z, None).unwrap().value(),
                        &hi.atan(&zh, None).unwrap().value(),
                        p,
                        &name("atan"),
                    );
                    assert_rounds_from(
                        &ctx.asin(&z, None).unwrap().value(),
                        &hi.asin(&zh, None).unwrap().value(),
                        p,
                        &name("asin"),
                    );
                    assert_rounds_from(
                        &ctx.acos(&z, None).unwrap().value(),
                        &hi.acos(&zh, None).unwrap().value(),
                        p,
                        &name("acos"),
                    );
                }
            }
        }};
    }
    check!(Down);
    check!(Up);
    check!(Zero);
}
