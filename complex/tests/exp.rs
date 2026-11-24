//! Fixed regression cases for the complex exponential and the power family (`exp`, `powi`,
//! `powf`): special values, exactness flags, exactly-representable results and range errors.

mod helpers;

use dashu_base::Sign;
use dashu_cmplx::round::mode::{Down, HalfAway, HalfEven, Up, Zero};
use dashu_cmplx::{CBig, Context, FBig, FpError, Rounding};
use dashu_int::IBig;
use helpers::{assert_rounds_from, cb, fb, flags, retag, retag_c, zero};

const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

type C = CBig<HalfEven, 2>;

/// A base-2 `HalfEven` value `re.0·2^re.1 + i·im.0·2^im.1` (unlimited precision).
fn c(re: (i64, isize), im: (i64, isize)) -> C {
    cb(re, im)
}

/// `exp` of a zero is `1` at the context's precision (not the unlimited precision of a bare
/// constant, on which `ln` would panic), and the imaginary zero keeps its sign:
/// `exp(conj z) = conj(exp z)`.
#[test]
fn exp_of_zero_keeps_precision_and_zero_sign() {
    for p in PRECISIONS {
        for re_neg in [false, true] {
            for im_neg in [false, true] {
                let z = CBig::<HalfEven, 2>::from_parts(
                    zero(re_neg).with_precision(p).value(),
                    zero(im_neg).with_precision(p).value(),
                );
                let r = Context::<HalfEven>::new(p).exp(&z, None).unwrap();
                assert_eq!(flags(&r), None, "exp(0) is exact");
                let e = z.exp();
                assert_eq!(e.precision(), p, "exp(0) must carry the operand's precision");
                assert_eq!(e.re(), fb::<HalfEven, 2>(1, 0).repr());
                assert!(e.im().significand().is_zero());
                assert_eq!(e.im().is_neg_zero(), im_neg, "exp(±0 {im_neg:?}) imaginary zero sign");
                // the result is usable: ln(exp(0)) = 0
                assert!(e.ln().re().significand().is_zero());
            }
        }
    }
    // a zero produced by cancellation behaves the same
    let a: FBig<HalfEven, 2> = fb(3, 0).with_precision(10).value();
    let z = CBig::from_parts(a.clone() - a.clone(), a.clone() - a);
    assert_eq!(z.exp().precision(), 10);
    assert!(z.exp().ln().is_zero());
}

/// An argument wider than the working precision must not be rounded before it reaches `exp`:
/// the rounding error of `1.19e15` held to 15 bits is ~2³⁵, and `e^(2³⁵)` times the result is
/// an error bound that used to be aligned against the result bit for bit (an out-of-memory
/// panic at precision 1, and tens of gigabytes at any low precision).
#[test]
fn exp_of_wide_large_real_at_low_precision() {
    let x: FBig<Zero, 2> = fb(278130, 32); // ≈ 1.19e15, an 18-bit significand
    let z = CBig::from_parts(x.clone(), FBig::ZERO);
    for p in [1usize, 2, 5, 20] {
        let real = dashu_float::Context::<Zero>::new(p)
            .exp(x.repr(), None)
            .unwrap()
            .value();
        let r = Context::<Zero>::new(p).exp(&z, None).unwrap();
        // e^x is irrational: the real part is rounded, the imaginary part an exact zero
        assert_eq!(flags(&r), Some((Rounding::NoOp, Rounding::NoOp)), "p={p}");
        let c = r.value();
        assert_eq!(c.re(), real.repr(), "p={p}");
        assert!(c.im().is_pos_zero(), "p={p}");
    }
}

/// `z⁰ = 1` carries the operand's precision like every other power, for `powi` and `powf`.
#[test]
fn zeroth_power_keeps_precision() {
    for p in PRECISIONS {
        for z in [cb::<HalfEven, 2>((3, 0), (-5, -2)), cb((0, 0), (0, 0))] {
            let (re, im) = z.into_parts();
            let z = CBig::from_parts(re.with_precision(p).value(), im.with_precision(p).value());
            let one = z.powi(IBig::ZERO);
            assert_eq!(one.precision(), p);
            assert!(one == C::ONE);
            // usable by the transcendental functions (they reject unlimited precision)
            assert!(one.sqrt() == C::ONE);

            let w0 = CBig::from_parts(
                zero::<HalfEven, 2>(false).with_precision(p).value(),
                zero::<HalfEven, 2>(false).with_precision(p).value(),
            );
            let one = z.powf(&w0);
            assert_eq!(one.precision(), p);
            assert!(one == C::ONE);
            assert_eq!(one.exp().precision(), p);
        }
    }
}

/// A non-negative integer power of an unlimited-precision value is exact, like `z * z`.
#[test]
fn powi_at_unlimited_precision_is_exact() {
    assert!(C::I.powi(2.into()) == C::NEG_ONE);
    assert!(C::I.powi(4.into()) == C::ONE);
    assert!(C::ONE.powi(1.into()) == C::ONE);
    let z: C = cb((3, 0), (4, 0));
    assert_eq!(z.precision(), 0);
    // (3 + 4i)³ = −117 + 44i, (3 + 4i)⁵ = −237 − 3116i
    let cube = z.powi(3.into());
    assert_eq!(cube.precision(), 0);
    assert!(cube == c((-117, 0), (44, 0)));
    assert!(z.powi(5.into()) == c((-237, 0), (-3116, 0)));
    // agrees with repeated multiplication on a non-integer value
    let w: C = cb((5, -3), (-7, -2));
    assert!(w.powi(3.into()) == &(&w * &w) * &w);
    // the context layer reports it exact
    let r = Context::<Down>::new(0)
        .powi(&cb::<Down, 2>((3, 0), (4, 0)), 3.into())
        .unwrap();
    assert_eq!(flags(&r), None);
}

/// A reciprocal is not exactly representable in general, so a negative power still needs a
/// precision.
#[test]
#[should_panic(expected = "precision cannot be 0")]
fn powi_negative_at_unlimited_precision_panics() {
    let _ = CBig::<HalfEven, 2>::I.powi((-1).into());
}

/// `(t·(1 − i))ⁿ` lies on the same exact lattice as `(t·(1 + i))ⁿ` (its conjugate): one part of
/// an even power is exactly zero, which the generic squaring chain can never certify once the
/// chain is inexact (a negative exponent). `(3 − 3i)⁻² = i/18` used to retry forever.
#[test]
fn powi_of_anti_diagonal_base() {
    // (3 ± 3i)^(−2) = ∓i/18; at 2 bits 1/18 rounds to 1/16
    let ctx = Context::<HalfEven>::new(2);
    let diag = ctx.powi(&cb((3, 0), (3, 0)), (-2).into()).unwrap().value();
    let anti = ctx.powi(&cb((3, 0), (-3, 0)), (-2).into()).unwrap().value();
    assert!(diag.re().significand().is_zero() && anti.re().significand().is_zero());
    assert_eq!(diag.im(), fb::<HalfEven, 2>(-1, -4).repr());
    assert_eq!(anti.im(), fb::<HalfEven, 2>(1, -4).repr());

    // every orientation of the diagonal, against the conjugate / negation of the (1 + i) case,
    // under the nearest mode (which is symmetric in sign) across the precision sweep
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        for n in [-7i32, -4, -2, -1, 1, 2, 3, 6, 200, -200] {
            let base = ctx.powi(&c((3, 0), (3, 0)), n.into()).unwrap().value();
            let conj = ctx.powi(&cb((3, 0), (-3, 0)), n.into()).unwrap().value();
            assert!(conj == base.conj(), "(3-3i)^{n} at p={p}");
            let neg = ctx.powi(&cb((-3, 0), (3, 0)), n.into()).unwrap().value();
            let want = if n % 2 == 0 {
                base.conj()
            } else {
                -base.conj()
            };
            assert!(neg == want, "(-3+3i)^{n} at p={p}");
        }
    }

    // directed modes: (3 − 3i)⁻² = i/18 exactly, so Down and Up bracket 1/18 and the real part
    // is an exact zero
    for p in PRECISIONS {
        let z_dn: CBig<Down, 2> = cb((3, 0), (-3, 0));
        let z_up: CBig<Up, 2> = cb((3, 0), (-3, 0));
        let dn = Context::<Down>::new(p)
            .powi(&z_dn, (-2).into())
            .unwrap()
            .value();
        let up = Context::<Up>::new(p)
            .powi(&z_up, (-2).into())
            .unwrap()
            .value();
        assert!(dn.re().significand().is_zero() && up.re().significand().is_zero());
        let eighteen = fb::<HalfEven, 2>(18, 0);
        let dn_im = retag::<Down, HalfEven, 2>(&dn.into_parts().1);
        let up_im = retag::<Up, HalfEven, 2>(&up.into_parts().1);
        // exact products at unlimited precision
        assert!(&dn_im * &eighteen < fb::<HalfEven, 2>(1, 0), "Down below 1/18 at p={p}");
        assert!(&up_im * &eighteen > fb::<HalfEven, 2>(1, 0), "Up above 1/18 at p={p}");
    }
}

/// An overflowing / underflowing power of a *real* base reports the sign of the true result
/// (negative for a negative base and an odd exponent), like the real `powi`.
#[test]
fn powi_real_base_range_error_sign() {
    let big = IBig::from(4611686018427387905i64); // 2⁶² + 1, odd
    let ctx = Context::<HalfEven>::new(53);
    let fctx = dashu_float::Context::<HalfEven>::new(53);
    for (base, n) in [
        ((-1i64, -2isize), big.clone()),
        ((-1, 2), -big.clone()),
        ((1, -2), big.clone()),
        ((-1, 2), big.clone()),
        ((1, 2), big.clone()),
        ((-1, -2), big.clone() + IBig::ONE), // even exponent: positive
    ] {
        let x: FBig<HalfEven, 2> = fb(base.0, base.1);
        let z = CBig::from_parts(x.clone(), FBig::ZERO);
        let real = fctx.powi(x.repr(), n.clone());
        let complex = ctx.powi(&z, n.clone());
        match (real, complex) {
            (Err(a), Err(b)) => assert_eq!(a, b, "base {base:?}"),
            (a, b) => panic!("expected a range error from both layers, got {a:?} / {b:?}"),
        }
    }
    // the convenience layer turns the negative underflow into a −0 real part
    let z = CBig::<HalfEven, 2>::from_parts(fb(-1, -2).with_precision(53).value(), FBig::ZERO);
    let r = z.powi(big.clone());
    assert!(r.re().is_neg_zero());
    assert_eq!(ctx.powi(&z, big), Err(FpError::Underflow(Sign::Negative)));
}

/// A rounded reciprocal is `Inexact`, with the same flags `inv` reports: `(16 + i)⁻¹ =
/// (16 − i)/257` has no finite binary expansion, whatever the precision.
#[test]
fn powi_minus_one_flags_match_inv() {
    for p in [1usize, 2, 20, 50, 100, 500] {
        let ctx = Context::<HalfEven>::new(p);
        for z in [c((16, 0), (1, 0)), c((17, 0), (1, 0)), c((3, 0), (-7, 2))] {
            let pow = ctx.powi(&z, (-1).into()).unwrap();
            let inv = ctx.inv(&z).unwrap();
            assert!(flags(&pow).is_some(), "powi(z, -1) at p={p} must be Inexact");
            assert_eq!(flags(&pow), flags(&inv), "p={p}");
            assert!(pow.value() == inv.value(), "p={p}");
        }
    }
    let r = Context::<HalfEven>::new(1)
        .powi(&c((16, 0), (1, 0)), (-1).into())
        .unwrap();
    assert_eq!(flags(&r), Some((Rounding::AddOne, Rounding::SubOne)));
    // an exactly representable reciprocal stays Exact: (1 + i)⁻¹ = 1/2 − i/2, (4i)⁻¹ = −i/4
    for z in [cb((1, 0), (1, 0)), cb((0, 0), (4, 0))] {
        let r = Context::<HalfEven>::new(20)
            .powi::<2>(&z, (-1).into())
            .unwrap();
        assert_eq!(flags(&r), None);
    }
}

/// `0^w = 0` for `Re w > 0`; a zero base with `Re w ≤ 0` (and `w ≠ 0`) has no value.
#[test]
fn powf_of_zero_base() {
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        let z0: C = cb((0, 0), (0, 0));
        for w in [
            cb((1, 0), (0, 0)),
            cb((2, 0), (0, 0)),
            cb((1, -1), (0, 0)),
            cb((3, -2), (-5, 0)),
        ] {
            let r = ctx.powf(&z0, &w, None).unwrap();
            assert_eq!(flags(&r), None);
            let v = r.value();
            assert!(v.is_zero(), "0^w for Re w > 0 at p={p}");
            assert_eq!(v.precision(), p);
        }
        assert!(ctx.powf(&z0, &cb((0, 0), (0, 0)), None).unwrap().value() == C::ONE);
        for w in [
            cb((-1, 0), (0, 0)),
            cb((-1, -1), (1, 0)),
            cb((0, 0), (1, 0)),
        ] {
            assert_eq!(ctx.powf(&z0, &w, None), Err(FpError::Indeterminate));
        }
    }
    // the convenience layer does not panic
    let mk = |v: i64| fb::<HalfEven, 2>(v, 0).with_precision(53).value();
    let z0 = C::from_parts(mk(0), mk(0));
    assert!(z0.powf(&C::from_parts(mk(1), mk(0))).is_zero());
}

/// An integer exponent is `powi`: exact results stay exact (`i¹ = i`, `i² = −1`), and a result
/// that is exactly a rounding tie is rounded by the mode instead of being approached forever
/// through `exp(w·log z)`.
#[test]
fn powf_with_integer_exponent_is_powi() {
    let ctx = Context::<HalfEven>::new(53);
    let i = cb::<HalfEven, 2>((0, 0), (1, 0));
    let r = ctx.powf(&i, &cb((1, 0), (0, 0)), None).unwrap();
    assert_eq!(flags(&r), None);
    assert!(r.value() == i);
    let r = ctx.powf(&i, &cb((2, 0), (0, 0)), None).unwrap();
    assert_eq!(flags(&r), None);
    assert!(r.value() == C::NEG_ONE);

    // (28 + 64i)⁵ = 1466866688 − 784777216i: the real part has 21 significant bits, a tie at 20;
    // (4 + 1196i)⁶ has a 41-bit imaginary part, a tie at 40
    for (z, n, p) in [
        (cb::<HalfEven, 2>((28, 0), (64, 0)), 5i64, 20usize),
        (cb((28, 0), (64, 0)), 5, 21),
        (cb((28, 0), (65, 0)), 5, 20),
        (cb((4, 0), (1196, 0)), 6, 40),
        (cb((4, 0), (1196, 0)), 6, 41),
        (cb((-3, 0), (2, -1)), -3, 50),
    ] {
        let ctx = Context::<HalfEven>::new(p);
        let powi = ctx.powi(&z, n.into()).unwrap();
        let powf = ctx.powf(&z, &cb((n, 0), (0, 0)), None).unwrap();
        assert_eq!(flags(&powf), flags(&powi), "{n} at p={p}");
        assert!(powf.value() == powi.value(), "{n} at p={p}");
    }
    let r = Context::<HalfEven>::new(20)
        .powf(&cb((28, 0), (64, 0)), &cb((5, 0), (0, 0)), None)
        .unwrap()
        .value();
    assert_eq!(r.re(), fb::<HalfEven, 2>(179061, 13).repr());
    assert_eq!(r.im(), fb::<HalfEven, 2>(-47899, 14).repr());

    // base 10: the exponent 2 written as 20·10⁻¹ is the same integer
    let ctx = Context::<HalfAway>::new(7);
    let z: CBig<HalfAway, 10> = cb((12, -1), (5, -1));
    let two = CBig::from_parts(FBig::from_parts(20.into(), -1), FBig::ZERO);
    let r = ctx.powf(&z, &two, None).unwrap();
    assert_eq!(flags(&r), None); // (1.2 + 0.5i)² = 1.19 + 1.2i
    assert!(r.value() == cb::<HalfAway, 10>((119, -2), (12, -1)));
}

/// A real exponent `m/2ᵏ` goes through exact-when-exact operations (square roots, then the
/// integer chain): `(−1)^½ = i` used to retry forever on its exactly-zero real part.
#[test]
fn powf_with_dyadic_exponent() {
    macro_rules! exact_cases {
        ($mode:ty) => {{
            for p in PRECISIONS {
                let ctx = Context::<$mode>::new(p);
                let half = cb::<$mode, 2>((1, -1), (0, 0));
                // (−1)^½ = i, 4^½ = 2, (−4)^(3/2) = −8i, (2i)^½ = 1 + i, 16^(−¼) = ½
                for (z, w, want) in [
                    (cb((-1, 0), (0, 0)), half.clone(), cb::<$mode, 2>((0, 0), (1, 0))),
                    (cb((4, 0), (0, 0)), half.clone(), cb((2, 0), (0, 0))),
                    (cb((-4, 0), (0, 0)), cb((3, -1), (0, 0)), cb((0, 0), (-8, 0))),
                    (cb((0, 0), (2, 0)), half.clone(), cb((1, 0), (1, 0))),
                    (cb((16, 0), (0, 0)), cb((-1, -2), (0, 0)), cb((1, -1), (0, 0))),
                ] {
                    let r = ctx.powf(&z, &w, None).unwrap();
                    assert_eq!(flags(&r), None, "{} p={p}: {z:?}^{w:?}", stringify!($mode));
                    assert!(r.value() == want, "{} p={p}: {z:?}^{w:?}", stringify!($mode));
                }
                // an inexact root keeps its exact zero part: (−2)^½ = i·√2
                let r = ctx.powf(&cb((-2, 0), (0, 0)), &half, None).unwrap();
                let root2 = dashu_float::Context::<$mode>::new(p)
                    .sqrt(fb::<$mode, 2>(2, 0).repr())
                    .unwrap()
                    .value();
                assert!(flags(&r).is_some());
                let v = r.value();
                assert!(v.re().is_pos_zero());
                assert_eq!(v.im(), root2.repr());
            }
        }};
    }
    exact_cases!(HalfEven);
    exact_cases!(Down);
    exact_cases!(Up);
    exact_cases!(Zero);

    // z^½ is the principal square root, for a generic z in every quadrant
    for p in PRECISIONS {
        let ctx = Context::<HalfEven>::new(p);
        for z in [
            c((3, 0), (5, 0)),
            c((-3, 0), (5, 0)),
            c((-3, 0), (-5, 0)),
            c((7, -3), (-1, 0)),
        ] {
            let pow = ctx.powf(&z, &cb((1, -1), (0, 0)), None).unwrap();
            let root = ctx.sqrt(&z).unwrap();
            assert!(pow.value() == root.value(), "sqrt as a power at p={p}");
        }
    }

    // base 10: 0.5 = 5·10⁻¹ is the same half
    let ctx = Context::<HalfAway>::new(16);
    let half: CBig<HalfAway, 10> = cb((5, -1), (0, 0));
    let r = ctx.powf(&cb((-1, 0), (0, 0)), &half, None).unwrap();
    assert_eq!(flags(&r), None);
    assert!(r.value() == cb::<HalfAway, 10>((0, 0), (1, 0)));
    let r = ctx.powf(&cb((625, -2), (0, 0)), &half, None).unwrap();
    assert_eq!(flags(&r), None);
    assert!(r.value() == cb::<HalfAway, 10>((25, -1), (0, 0)));
}

/// The general (non-dyadic) path agrees with a higher-precision evaluation under every mode,
/// and a positive real base with a real exponent keeps an exactly-zero imaginary part.
#[test]
fn powf_general_matches_oracle() {
    macro_rules! check {
        ($mode:ty) => {{
            for p in PRECISIONS {
                let ctx = Context::<$mode>::new(p);
                let hi = Context::<HalfEven>::new(p + 80);
                for (z, w) in [
                    (cb::<$mode, 2>((2, 0), (0, 0)), cb::<$mode, 2>((5, -4), (0, 0))),
                    (cb((3, 0), (-4, 0)), cb((5, -4), (1, -3))),
                    (cb((-7, -2), (1, 0)), cb((-11, -5), (3, -1))),
                ] {
                    // 5/16 is dyadic; perturb the exponent off the dyadic grid
                    let w = &w + &cb::<$mode, 2>((1, -70), (0, 0));
                    let got = ctx.powf(&z, &w, None).unwrap().value();
                    let want = hi.powf(&retag_c(&z), &retag_c(&w), None).unwrap().value();
                    assert_rounds_from(
                        &got,
                        &want,
                        p,
                        &format!("{} powf p={p}", stringify!($mode)),
                    );
                }
                let got = ctx
                    .powf(&cb::<$mode, 2>((2, 0), (0, 0)), &cb((1, -70), (0, 0)), None)
                    .unwrap()
                    .value();
                assert!(got.im().significand().is_zero(), "real^real has a zero imaginary part");
            }
        }};
    }
    check!(HalfEven);
    check!(Down);
    check!(Up);
}
