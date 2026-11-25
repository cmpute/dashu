//! Multiplication-family tests (`mul`, `sqr`, `mul_real`, `fma`, `norm`): each result part is the
//! exact sum of products rounded once, so the value, the `Exact`/`Inexact` tag and the rounding
//! direction are checked against an exact oracle (unlimited-precision `FBig` arithmetic is
//! lossless), including under cancellation.

use dashu_base::Approximation::{self, *};
use dashu_base::Sign;
use dashu_cmplx::{CBig, Context, FBig};
use dashu_float::round::mode::{Down, HalfAway, HalfEven, Up, Zero};
use dashu_float::round::{Round, Rounding};
use dashu_float::{Context as FloatContext, Repr};
use dashu_int::{IBig, Word};
use proptest::prelude::*;

/// The precisions every `CBig` change is exercised at.
const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

/// `sig · 2^exp` at unlimited precision (exact).
fn exact<R: Round>(sig: impl Into<IBig>, exp: isize) -> FBig<R, 2> {
    FBig::from_repr(Repr::new(sig.into(), exp), FloatContext::new(0))
}

/// A complex number from integer parts, both at precision `p`.
fn int_c<R: Round, const B: Word>(re: i128, im: i128, p: usize) -> CBig<R, B> {
    let part = |v: i128| FBig::<R, B>::from_repr(Repr::new(v.into(), 0), FloatContext::new(p));
    CBig::from_parts(part(re), part(im))
}

/// Unwrap a result into its two parts as (value, is_exact) — a part is exact iff the whole
/// result is `Exact` or the oracle below says so; the tag is compared separately.
fn parts<R: Round, const B: Word>(z: &CBig<R, B>) -> (FBig<R, B>, FBig<R, B>) {
    z.clone().into_parts()
}

/// Assert that `got` is the exact value `(re, im)` rounded once to `p` digits: same parts, and
/// `Exact` iff neither part needed rounding.
fn assert_rounded<R: Round>(
    got: Approximation<CBig<R, 2>, (Rounding, Rounding)>,
    re: FBig<R, 2>,
    im: FBig<R, 2>,
    p: usize,
    what: &str,
) {
    let (want_re, want_im) = (re.with_precision(p), im.with_precision(p));
    let want_exact = matches!((&want_re, &want_im), (Exact(_), Exact(_)));
    assert_eq!(matches!(got, Exact(_)), want_exact, "{what}: exactness tag, p={p}");
    if let Inexact(_, (re_adj, im_adj)) = &got {
        let adj = |r: &Approximation<FBig<R, 2>, Rounding>| match r {
            Exact(_) => Rounding::NoOp,
            Inexact(_, adj) => *adj,
        };
        assert_eq!(
            (*re_adj, *im_adj),
            (adj(&want_re), adj(&want_im)),
            "{what}: adjustments, p={p}"
        );
    }
    let (got_re, got_im) = parts(&got.value());
    assert_eq!(got_re.repr(), want_re.value().repr(), "{what}: real part, p={p}");
    assert_eq!(got_im.repr(), want_im.value().repr(), "{what}: imaginary part, p={p}");
}

#[test]
fn mul_and_sqr_real_part_cancellation_binary() {
    // (2^52+1 + 2^52 i)·(−(2^52−1) − 2^52 i): re = −(2^104 − 1) + 2^104 = 1 exactly.
    type C = CBig<HalfEven, 2>;
    let t = 1i128 << 52;
    let ctx = Context::<HalfEven>::new(53);
    let z: C = int_c(t + 1, t, 53);
    let w: C = int_c(-(t - 1), -t, 53);
    let r = ctx.mul(&z, &w).unwrap();
    assert!(matches!(r, Exact(_)), "both parts (1 and −2^105) are exact: {r:?}");
    let (re, im) = parts(&r.value());
    assert_eq!(re.repr(), &Repr::<2>::one());
    assert_eq!(im.repr(), &Repr::<2>::new((-1).into(), 105));
    assert_eq!((&z * &w).re(), &Repr::<2>::one());

    // (2^52 + (2^52−2) i)²: re = 2^104 − (2^52−2)² = 2^54 − 4, representable in 53 bits.
    let z: C = int_c(t, t - 2, 53);
    let (re, _) = parts(&ctx.sqr(&z).unwrap().value());
    assert_eq!(re, exact::<HalfEven>((1i128 << 54) - 4, 0));
    assert_eq!(z.sqr().re(), re.repr());
}

#[test]
fn mul_and_sqr_real_part_cancellation_decimal() {
    // 11-digit decimal operands whose exact real part fits in the precision.
    type C = CBig<HalfAway, 10>;
    for (x, y) in [(3162277660i128, 3162277661i128), (31622776601, 31622776602)] {
        let p = x.to_string().len();
        let z: C = int_c(x, y, p);
        let want = Repr::<10>::new((x * x - y * y).into(), 0);
        assert_eq!(z.sqr().re(), &want, "sqr of {x}+{y}i");
        assert_eq!((&z * &z).re(), &want, "mul of {x}+{y}i by itself");
    }
    for (x, y, u, v) in [
        (1000000001i128, 9999999990i128, 9999999992i128, 1000000001i128),
        (10000000001, 99999999990, 99999999992, 10000000001),
    ] {
        let p = x.to_string().len();
        let (z, w): (C, C) = (int_c(x, y, p), int_c(u, v, p));
        let want = Repr::<10>::new((x * u - y * v).into(), 0);
        assert_eq!((&z * &w).re(), &want, "({x}+{y}i)·({u}+{v}i)");
    }
}

#[test]
fn mul_zero_parts_follow_ieee_zero_sums() {
    // The imaginary part of (x+iy)(u+iv) is x·v + y·u; for real operands that is a sum of two
    // signed zeros, which is +0 unless both are −0 (and −0 under roundTowardNegative when the
    // signs differ) — in either operand order.
    type C = CBig<HalfEven, 2>;
    for p in PRECISIONS {
        let (z, w): (C, C) = (int_c(-4, 0, p), int_c(1, 0, p));
        for prod in [&z * &w, &w * &z] {
            assert_eq!(prod.re(), &Repr::<2>::new((-4).into(), 0), "p={p}");
            assert!(prod.im().is_pos_zero(), "p={p}: {prod:?}");
        }
        let (z, w): (CBig<Down, 2>, CBig<Down, 2>) = (int_c(-4, 0, p), int_c(1, 0, p));
        for prod in [&z * &w, &w * &z] {
            assert!(prod.im().is_neg_zero(), "Down, p={p}: {prod:?}");
        }

        // (+0 + 0i)(+0 − 0i): im = (+0)(−0) + (+0)(+0) = +0, re = (+0)(+0) − (+0)(−0) = +0
        let pos = || FBig::<HalfEven, 2>::from_repr(Repr::zero(), FloatContext::new(p));
        let neg = || FBig::<HalfEven, 2>::from_repr(Repr::neg_zero(), FloatContext::new(p));
        let prod = &C::from_parts(pos(), pos()) * &C::from_parts(pos(), neg());
        assert!(prod.re().is_pos_zero() && prod.im().is_pos_zero(), "p={p}: {prod:?}");
        // like-signed zero products keep their sign: (−0 + 0i)(+0 + 0i) has re = −0 − 0 = −0
        let prod = &C::from_parts(neg(), pos()) * &C::from_parts(pos(), pos());
        assert!(prod.re().is_neg_zero(), "p={p}: {prod:?}");
        assert_eq!(prod.precision(), p);
    }
}

#[test]
fn rounded_results_are_tagged_inexact() {
    // A truncated part is `Inexact(_, NoOp)`, never `Exact`.
    type C = CBig<HalfEven, 2>;
    let ctx = Context::<HalfEven>::new(5);
    // 3i · 10i = −30 fits in 5 bits; 3i · 11i = −33 does not
    let r = ctx
        .mul(&int_c(0, 3, 5), &int_c::<HalfEven, 2>(0, 10, 5))
        .unwrap();
    assert!(matches!(r, Exact(_)), "{r:?}");
    let r = ctx
        .mul(&int_c(0, 3, 5), &int_c::<HalfEven, 2>(0, 11, 5))
        .unwrap();
    assert_eq!(r, Inexact(int_c(-32, 0, 5), (Rounding::NoOp, Rounding::NoOp)));
    let r = ctx
        .mul_real(&int_c::<HalfEven, 2>(3, 0, 5), &exact(11, 0).with_precision(5).value())
        .unwrap();
    assert_eq!(r, Inexact(int_c(32, 0, 5), (Rounding::NoOp, Rounding::NoOp)));
    let r = ctx.sqr(&int_c::<HalfEven, 2>(33, 0, 6)).unwrap();
    assert_eq!(r, Inexact(int_c(1088, 0, 5), (Rounding::NoOp, Rounding::NoOp)));

    // norm: 3+4i → 25 is exact at 5 bits; 1 + 2^k i → 2^2k + 1 is not
    let n = ctx.norm(&int_c::<HalfEven, 2>(3, 4, 5)).unwrap();
    assert_eq!(n, Exact(exact(25, 0).with_precision(5).value()));
    for (k, p) in [(1usize, 1usize), (5, 1), (20, 20), (60, 100)] {
        let z: C = int_c(1, 1 << k, p.max(k + 1));
        let n = Context::<HalfEven>::new(p).norm(&z).unwrap();
        assert!(matches!(n, Inexact(..)), "norm(1 + 2^{k} i) at {p} bits: {n:?}");
        assert_eq!(n.value().repr(), &Repr::<2>::new(1.into(), 2 * k as isize));
    }

    // fma: i·i − z3 = −1 − z3, rounded up to 2 bits
    let ctx = Context::<HalfEven>::new(2);
    let i: C = int_c(0, 1, 2);
    for z3 in [-8i128, -4096, -8192] {
        let r = ctx.fma(&i, &i, &int_c(z3, 0, 2), Sign::Negative).unwrap();
        assert_eq!(
            r,
            Inexact(int_c(-z3, 0, 2), (Rounding::AddOne, Rounding::NoOp)),
            "fma(i, i, {z3}, −)"
        );
    }
}

#[test]
fn fma_keeps_a_fully_cancelled_result() {
    // (1 + 10^-k)² − (1 + 2·10^-k) = 10^-2k exactly, for real operands at k+1 digits.
    type C = CBig<HalfAway, 10>;
    type F = FBig<HalfAway, 10>;
    for (k, p) in [
        (10isize, 11usize),
        (19, 20),
        (49, 50),
        (99, 100),
        (499, 500),
    ] {
        let dec = |sig: IBig, exp: isize| F::from_repr(Repr::new(sig, exp), FloatContext::new(p));
        let a = dec(IBig::from(10).pow(k as usize) + 1, -k);
        let e = dec(IBig::from(10).pow(k as usize) + 2, -k);
        let z = C::from_parts(a.clone(), dec(0.into(), 0));
        let z3 = C::from_parts(e.clone(), dec(0.into(), 0));
        let r = z.fma(&z, &z3, Sign::Negative);
        assert_eq!(r.re(), &Repr::<10>::new(1.into(), -2 * k), "p={p}");
        assert!(r.im().is_pos_zero(), "p={p}");
        // … which is what the real FMA gives
        assert_eq!(r.re(), a.fma(&a, &-e, Sign::Positive).repr(), "p={p}");
    }
    // a non-cancelling neighbour from the same report
    let dec =
        |sig: i128, exp: isize| F::from_repr(Repr::new(sig.into(), exp), FloatContext::new(11));
    let zero = || dec(0, 0);
    let z1 = C::from_parts(dec(10000060183, 25), zero());
    let z2 = C::from_parts(dec(99999537265, 25), zero());
    let z3 = C::from_parts(dec(10000013912, 61), zero());
    let exact_re = IBig::from(10000060183i64) * IBig::from(99999537265i64)
        - IBig::from(10000013912i64) * IBig::from(10).pow(11);
    let want = F::from_repr(Repr::new(exact_re, 50), FloatContext::new(0)).with_precision(11);
    assert_eq!(z1.fma(&z2, &z3, Sign::Negative).re(), want.value().repr());
}

#[test]
fn fma_rounds_at_the_largest_operand_precision() {
    // The convenience `fma` rounds at the largest precision of its three operands — not at the
    // receiver's — like the binary operators and `FBig::fma`.
    type C = CBig<HalfAway, 10>;
    type F = FBig<HalfAway, 10>;
    let d = |s: &str, p: usize| s.parse::<F>().unwrap().with_precision(p).value();
    let a = C::from_parts(d("1.1", 2), d("0", 2));
    let b = C::from_parts(d("1.23456789", 20), d("0", 20));
    let c = C::from_parts(d("0.000001", 20), d("0", 20));
    let ab = a.fma(&b, &c, Sign::Positive);
    assert_eq!(ab.precision(), 20);
    assert_eq!(ab, &a * &b + &c);
    assert_eq!(ab, b.fma(&a, &c, Sign::Positive));
    let real = d("1.1", 2).fma(&d("1.23456789", 20), &d("0.000001", 20), Sign::Positive);
    assert_eq!(ab.re(), real.repr());

    // whichever operand carries the wide precision, the result is the context-layer one there
    for p in PRECISIONS {
        let narrow = |re, im| int_c::<HalfEven, 2>(re, im, 4);
        let wide = |re, im| int_c::<HalfEven, 2>(re, im, p);
        let ctx = Context::<HalfEven>::new(p);
        let (z1, z2, z3) = ((13, -11), (7, 9), (1 << 40, 3));
        for which in 0..3 {
            let pick = |i: usize, (re, im): (i128, i128)| {
                if i == which {
                    wide(re, im)
                } else {
                    narrow(re, im)
                }
            };
            let (z1, z2, z3) = (pick(0, z1), pick(1, z2), pick(2, z3));
            for sign in [Sign::Positive, Sign::Negative] {
                let r = z1.fma(&z2, &z3, sign);
                assert_eq!(r.precision(), p, "p={p} which={which}");
                assert_eq!(r, ctx.fma(&z1, &z2, &z3, sign).unwrap().value());
                assert_eq!(r, z2.fma(&z1, &z3, sign), "p={p} which={which}");
            }
        }
    }

    // an unlimited precision counts as the lowest
    let unlimited = int_c::<HalfEven, 2>(1 << 30, 1, 0);
    let limited = int_c::<HalfEven, 2>(3, 5, 20);
    assert_eq!(
        unlimited
            .fma(&unlimited, &limited, Sign::Positive)
            .precision(),
        20
    );
    assert_eq!(
        unlimited
            .fma(&unlimited, &unlimited, Sign::Positive)
            .precision(),
        0
    );

    // the cached wrapper mirrors it
    let cache = std::rc::Rc::new(core::cell::RefCell::new(dashu_float::ConstCache::new()));
    let (ca, cb, cc) = (
        a.clone().into_cached(cache.clone()),
        b.clone().into_cached(cache.clone()),
        c.clone().into_cached(cache),
    );
    assert_eq!(ca.fma(&cb, &cc, Sign::Positive).as_cbig(), &ab);
}

#[test]
fn sqr_respects_directed_rounding() {
    // z = 0 + yi: z² = −y² exactly; under Down the result must not exceed it (and must match
    // the general product).
    for (p, y) in [
        (20usize, exact::<Down>(-((1i128 << 19) + 1), 12)),
        (53, exact::<Down>(-((1i128 << 52) + 1), 12)),
        (100, exact::<Down>(-((1i128 << 99) + 1), 12)),
        (500, exact::<Down>(-((IBig::ONE << 499) + IBig::ONE), 12)),
    ] {
        let y = y.with_precision(p).value();
        let z = CBig::<Down, 2>::from_parts(FBig::ZERO.with_precision(p).value(), y.clone());
        let exact_re = -(exact::<Down>(y.repr().significand().clone(), y.repr().exponent()).sqr());
        let want = exact_re.clone().with_precision(p).value();
        assert!(want <= exact_re);
        assert_eq!(z.sqr().re(), want.repr(), "p={p}");
        assert_eq!((&z * &z).re(), want.repr(), "p={p}");
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn part_out_of_range_keeps_the_other_part() {
    // s·(1 + s·i) for s = 2^e far below the exponent range's square root: the imaginary part
    // s² underflows, the real part s is representable and must survive.
    type F = FBig<HalfEven, 2>;
    type C = CBig<HalfEven, 2>;
    let f = |sig: i32, exp: isize| F::from_repr(Repr::new(sig.into(), exp), FloatContext::new(53));
    for e in [-(1isize << 62) + 10, -(1isize << 62) - 10] {
        let s = f(1, e);
        let z = C::from_parts(f(1, 0), f(1, e));
        let prod = &s * &z;
        assert_eq!(prod.re(), s.repr(), "e={e}");
        assert_eq!(prod.im(), (&s * &s).repr(), "e={e}");
        // the full complex product keeps the representable part too
        let prod = &C::from_parts(s.clone(), f(0, 0)) * &z;
        assert_eq!(prod.re(), s.repr(), "e={e}");
        assert_eq!(prod.im(), (&s * &s).repr(), "e={e}");
    }
    // both parts out of range: the documented collapse to the complex infinity / zero
    let big = f(1, (1isize << 62) + 10);
    let z = C::from_parts(big.clone(), big.clone());
    let prod = &big * &z;
    assert!(prod.re().is_infinite() && prod.im().is_pos_zero());
}

#[test]
#[cfg(target_pointer_width = "64")]
fn product_out_of_range_inside_a_part() {
    // A single product leaving the exponent range does not erase the part it belongs to: the
    // part is still the exact sum rounded once.
    fn check<R: Round>(p: usize) {
        let f = |sig: i32, exp: isize| {
            FBig::<R, 2>::from_repr(Repr::new(sig.into(), exp), FloatContext::new(p))
        };
        let ctx = Context::<R>::new(p);
        let zero = CBig::from_parts(f(0, 0), f(0, 0));
        let one = exact::<R>(1, 0);
        let eps = exact::<R>(1, -(p as isize) - 5);
        for e in [
            -(1isize << 20),
            -(1 << 61),
            -(1 << 62) - 10,
            isize::MIN / 2 - (1 << 60),
        ] {
            // z = 1 + s·i with s = 2^e: z² = (1 − s²) + 2s·i, and s² is far below the range
            let z = CBig::from_parts(f(1, 0), f(1, e));
            let want_re = (one.clone() - eps.clone()).with_precision(p);
            let want_im = f(1, e + 1);
            for (name, r) in [
                ("sqr", ctx.sqr(&z)),
                ("mul", ctx.mul(&z, &z)),
                ("fma", ctx.fma(&z, &z, &zero, Sign::Positive)),
            ] {
                let r = r.unwrap_or_else(|err| panic!("{name} e={e} p={p}: {err:?}"));
                let flags = match &r {
                    Exact(_) => panic!("{name} e={e} p={p}: tagged exact"),
                    Inexact(_, flags) => *flags,
                };
                let r = r.value();
                assert_eq!(r.re(), want_re.value_ref().repr(), "{name} e={e} p={p}");
                assert_eq!(r.im(), want_im.repr(), "{name} e={e} p={p}");
                let want_flag = match &want_re {
                    Exact(_) => unreachable!(),
                    Inexact(_, flag) => *flag,
                };
                assert_eq!(flags, (want_flag, Rounding::NoOp), "{name} e={e} p={p}");
            }
            assert_eq!(z.sqr().re(), want_re.value_ref().repr());
            assert_eq!((&z * &z).re(), want_re.value_ref().repr());

            // (1 + s·i)(1 − s·i) = 1 + s² exactly real
            let r = ctx.mul(&z, &z.conj()).unwrap().value();
            let want = (one.clone() + eps.clone()).with_precision(p).value();
            assert_eq!(r.re(), want.repr(), "norm-like e={e} p={p}");
            assert!(r.im().significand().is_zero());
        }

        // products beyond the top of the range that cancel: (b + b·i)(b − b·i) has an exactly
        // zero imaginary part, and b·b − b·b + 5 = 5
        let b = f(3, (1 << 62) + 10);
        let z = CBig::from_parts(b.clone(), b.clone());
        let r = ctx.mul(&z, &z.conj()).unwrap().value();
        assert!(r.re().is_infinite() && r.re().sign() == Sign::Positive);
        assert!(r.im().significand().is_zero());
        let (z1, z2) = (CBig::from_parts(b.clone(), b.clone()), CBig::from_parts(b.clone(), b));
        let r = ctx.fma(&z1, &z2, &CBig::from_parts(f(5, 0), f(0, 0)), Sign::Positive);
        let r = r.unwrap().value();
        assert_eq!(r.re(), f(5, 0).repr());
        assert!(r.im().is_infinite());

        // a result below the range still underflows on its own
        let s = f(1, -(1 << 62) - 10);
        let z = CBig::from_parts(s.clone(), f(0, 0));
        let w = CBig::from_parts(s, f(1, 0));
        let r = ctx.mul(&z, &w).unwrap().value();
        assert!(r.re().is_pos_zero());
        assert_eq!(r.im(), f(1, -(1 << 62) - 10).repr());
    }
    for p in PRECISIONS {
        check::<HalfEven>(p);
        check::<HalfAway>(p);
        check::<Down>(p);
        check::<Up>(p);
        check::<Zero>(p);
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn astronomically_separated_addends_round_like_the_exact_sum() {
    // (2^E + i)(1 + i) = (2^E − 1) + (2^E + 1)i with E far beyond any materializable width: the
    // tiny addend only acts as a sticky bit.
    const E: isize = 1 << 50;
    fn check<R: Round>(re_sig: i128, re_exp: isize, im_sig: i128, im_exp: isize) {
        let p = 53;
        let f = |sig: i128, exp: isize| {
            FBig::<R, 2>::from_repr(Repr::new(sig.into(), exp), FloatContext::new(p))
        };
        let z = CBig::from_parts(f(1, E), f(1, 0));
        let w = CBig::from_parts(f(1, 0), f(1, 0));
        let r = Context::<R>::new(p).mul(&z, &w).unwrap();
        assert!(matches!(r, Inexact(..)));
        let r = r.value();
        let (re, im) = parts(&r);
        assert_eq!(re.repr(), &Repr::<2>::new(re_sig.into(), re_exp), "re");
        assert_eq!(im.repr(), &Repr::<2>::new(im_sig.into(), im_exp), "im");
        // norm(2^E + i) = 2^2E + 1
        let n = Context::<R>::new(p).norm(&z).unwrap();
        assert!(matches!(n, Inexact(..)));
        // fma with a third, even smaller addend
        let z3 = CBig::from_parts(f(1, -E), f(-1, -E));
        let r3 = Context::<R>::new(p)
            .fma(&z, &w, &z3, Sign::Positive)
            .unwrap();
        assert_eq!(r3.value(), r);
    }
    let below = (1i128 << 53) - 1; // 2^E − ulp/2-ish: the 53-bit predecessor of 2^E
    let above = (1i128 << 52) + 1; // the 53-bit successor of 2^E
    check::<HalfEven>(1, E, 1, E);
    check::<Zero>(below, E - 53, 1, E);
    check::<Down>(below, E - 53, 1, E);
    check::<Up>(1, E, above, E - 52);
}

fn operand() -> impl Strategy<Value = (i128, isize)> {
    // significands up to 100 bits with a modest exponent spread, so products both cancel and
    // straddle the rounding position at every tested precision
    prop_oneof![
        (any::<i64>().prop_map(i128::from), -70isize..70),
        (-(1i128 << 100)..(1i128 << 100), -70isize..70),
        (-4i128..4, -70isize..70),
    ]
}

/// Check `mul`, `sqr`, `fma` and `norm` against exact arithmetic under rounding mode `R`.
fn check_against_exact<R: Round>(ops: [(i128, isize); 6], subtract: bool) {
    let [x, y, u, v, e, f] = ops.map(|(sig, exp)| exact::<R>(sig, exp));
    let sign = if subtract {
        Sign::Negative
    } else {
        Sign::Positive
    };
    for p in PRECISIONS {
        let ctx = Context::<R>::new(p);
        // the operands keep their full width (wider than `p` at the small precisions)
        let wide = |re: &FBig<R, 2>, im: &FBig<R, 2>| {
            CBig::from_parts(
                re.clone().with_precision(200).value(),
                im.clone().with_precision(200).value(),
            )
        };
        let (z, w, z3) = (wide(&x, &y), wide(&u, &v), wide(&e, &f));

        let (re, im) = (&x * &u - &y * &v, &x * &v + &y * &u);
        assert_rounded(ctx.mul(&z, &w).unwrap(), re.clone(), im.clone(), p, "mul");
        assert_rounded(ctx.sqr(&z).unwrap(), &x * &x - &y * &y, &x * &y + &x * &y, p, "sqr");
        let (fre, fim) = if subtract {
            (re - &e, im - &f)
        } else {
            (re + &e, im + &f)
        };
        assert_rounded(ctx.fma(&z, &w, &z3, sign).unwrap(), fre, fim, p, "fma");

        let norm = (&x * &x + &y * &y).with_precision(p);
        let got = ctx.norm(&z).unwrap();
        assert_eq!(matches!(got, Exact(_)), matches!(norm, Exact(_)), "norm tag, p={p}");
        assert_eq!(got.value().repr(), norm.value().repr(), "norm, p={p}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn mul_family_is_correctly_rounded(ops in [operand(), operand(), operand(), operand(), operand(), operand()], subtract: bool) {
        check_against_exact::<HalfEven>(ops, subtract);
        check_against_exact::<HalfAway>(ops, subtract);
        check_against_exact::<Zero>(ops, subtract);
        check_against_exact::<Down>(ops, subtract);
        check_against_exact::<Up>(ops, subtract);
    }

    #[test]
    fn mul_cancelling_real_part_is_correctly_rounded(x in operand(), y in operand(), d in -3i128..4) {
        // (x + iy)(y' + ix) with y' = y + d·ulp: the real part x·y' − y·x nearly cancels
        let ops = [x, y, (y.0 + d, y.1), x, (1, 0), (0, 0)];
        check_against_exact::<HalfEven>(ops, false);
        check_against_exact::<Down>(ops, true);
    }
}
