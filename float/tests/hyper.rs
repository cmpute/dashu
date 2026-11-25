//! Tests for the hyperbolic functions: fixture values, IEEE-754 special values,
//! identities, and domain handling.

use core::str::FromStr;

use dashu_base::Sign;
use dashu_float::ops::Abs;
use dashu_float::{round::mode, Context, DBig, FBig, Repr};
use dashu_int::IBig;

#[test]
fn test_values() {
    // Low-precision fixtures, verified against mpmath
    // (sinh(0.5)=0.5210953054937474, cosh(0.5)=1.1276259652063807, …).
    let x = DBig::from_str("0.5000000").unwrap();
    assert_eq!(x.sinh().to_string(), "0.52109531");
    assert_eq!(x.cosh().to_string(), "1.127626");
    assert_eq!(x.tanh().to_string(), "0.46211716");
    assert_eq!(x.asinh().to_string(), "0.48121183");
    assert_eq!(x.atanh().to_string(), "0.54930614");
    let two = DBig::from_str("2.000000").unwrap();
    assert_eq!(two.acosh().to_string(), "1.316958");
}

#[test]
fn test_signed_zeros() {
    let ctx = Context::<mode::HalfEven>::new(30);
    // odd functions preserve the sign of zero: sinh/tanh/asinh/atanh(−0) = −0
    assert!(ctx
        .sinh::<10>(&Repr::<10>::zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_pos_zero());
    assert!(ctx
        .sinh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    assert!(ctx
        .tanh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    assert!(ctx
        .asinh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    assert!(ctx
        .atanh::<10>(&Repr::<10>::neg_zero(), None)
        .unwrap()
        .value()
        .repr()
        .is_neg_zero());
    // even function: cosh(±0) = 1
    assert_eq!(
        ctx.cosh::<10>(&Repr::<10>::neg_zero(), None)
            .unwrap()
            .value(),
        DBig::from_str("1").unwrap()
    );
}

/// Exact results at zero do not need a precision to round to: a zero carrying unlimited
/// precision (`FBig::ZERO`, `try_from(0.0)`) used to panic with "precision cannot be 0".
#[test]
fn test_exact_results_at_unlimited_precision_zero() {
    type F = FBig<mode::HalfEven, 2>;
    for zero in [F::ZERO, F::try_from(0.0f64).unwrap()] {
        assert_eq!(zero.precision(), 0);
        assert_eq!(zero.sinh(), F::ZERO);
        assert_eq!(zero.cosh(), F::ONE);
        assert_eq!(zero.sinh_cosh(), (F::ZERO, F::ONE));
        assert_eq!(zero.sinh_pi(), F::ZERO);
        assert_eq!(zero.cosh_pi(), F::ONE);
        assert_eq!(zero.sinh_cosh_pi(), (F::ZERO, F::ONE));
        assert_eq!(zero.tanh(), F::ZERO);
        assert_eq!(zero.asinh(), F::ZERO);
        assert_eq!(zero.atanh(), F::ZERO);
    }
    let neg_zero = F::try_from(-0.0f64).unwrap();
    assert!(neg_zero.sinh().repr().is_neg_zero());
    assert!(neg_zero.sinh_pi().repr().is_neg_zero());
    assert!(neg_zero.tanh().repr().is_neg_zero());
    assert!(neg_zero.asinh().repr().is_neg_zero());
    assert!(neg_zero.atanh().repr().is_neg_zero());
    // acosh(1) = 0 is exact as well
    assert_eq!(F::ONE.acosh(), F::ZERO);
}

/// Tiny arguments, down to exponents in the lower half of the `isize` range: `sinh`, `tanh`,
/// `asinh` and `atanh` are the float next to `x` on a known side, `cosh` the float next to 1 —
/// decided directly, under every mode and with the right flag, instead of by a working
/// precision of `2·|log2 x|` bits (or an underflowing `x²`).
#[test]
fn test_tiny_argument() {
    use dashu_base::Approximation::Inexact;
    use dashu_float::round::Rounding::{AddOne, NoOp};
    let p = 53usize;
    let ones = (IBig::ONE << p) - IBig::ONE;
    for e in [
        -1000isize,
        -1_000_000,
        isize::MIN / 4,
        -(isize::MAX / 2) - 10,
    ] {
        let x = Repr::<2>::new(IBig::ONE, e);
        let same = || FBig::<mode::Down, 2>::from_parts(1.into(), e);
        let inside = || FBig::<mode::Down, 2>::from_parts(ones.clone(), e - p as isize);
        let down = Context::<mode::Down>::new(p);
        // sinh and atanh move away from zero, tanh and asinh towards it
        assert_eq!(down.sinh(&x, None), Ok(Inexact(same(), NoOp)), "e = {e}");
        assert_eq!(down.atanh(&x, None), Ok(Inexact(same(), NoOp)));
        assert_eq!(down.tanh(&x, None), Ok(Inexact(inside(), NoOp)));
        assert_eq!(down.asinh(&x, None), Ok(Inexact(inside(), NoOp)));
        assert_eq!(down.cosh(&x, None), Ok(Inexact(FBig::ONE, NoOp)));
        assert_eq!(down.cosh_pi(&x, None), Ok(Inexact(FBig::ONE, NoOp)));
        assert_eq!(down.sinh_cosh(&x, None), (down.sinh(&x, None), down.cosh(&x, None)));
        assert_eq!(down.sinh_cosh_pi(&x, None), (down.sinh_pi(&x, None), down.cosh_pi(&x, None)));

        let up = Context::<mode::Up>::new(p);
        let above_one = FBig::<mode::Up, 2>::from_parts((IBig::ONE << (p - 1)) + 1, 1 - p as isize);
        assert_eq!(up.cosh(&x, None), Ok(Inexact(above_one.clone(), AddOne)));
        assert_eq!(up.cosh_pi(&x, None), Ok(Inexact(above_one, AddOne)));
        let next = FBig::<mode::Up, 2>::from_parts((IBig::ONE << (p - 1)) + 1, e + 1 - p as isize);
        assert_eq!(up.sinh(&x, None), Ok(Inexact(next, AddOne)));
        assert_eq!(up.tanh(&x, None), Ok(Inexact(FBig::from_parts(1.into(), e), AddOne)));

        // sinh(π·x) = π·x·(1 + tiny): the digits of π, at the argument's exponent
        let pi = Context::<mode::Down>::new(p).pi::<2>(None).value();
        let (sig, exp) = pi.into_repr().into_parts();
        let want = FBig::<mode::Down, 2>::from_parts(sig, exp + e);
        assert_eq!(down.sinh_pi(&x, None).unwrap().value(), want, "sinh_pi, e = {e}");
    }
}

/// Right at the bottom of the exponent range the neighbours of `x` have no representable last
/// place: the functions that return `x ± tiny` report an underflow, `cosh` is the float next
/// to 1.
#[test]
fn test_argument_at_bottom_of_exponent_range() {
    use dashu_base::Approximation::Inexact;
    use dashu_float::round::Rounding::NoOp;
    use dashu_float::FpError;
    let x = Repr::<2>::new(IBig::ONE, isize::MIN + 10);
    let neg = -x.clone();
    let down = Context::<mode::Down>::new(53);
    assert_eq!(down.sinh(&x, None), Err(FpError::Underflow(Sign::Positive)));
    assert_eq!(down.tanh(&neg, None), Err(FpError::Underflow(Sign::Negative)));
    assert_eq!(down.asinh(&x, None), Err(FpError::Underflow(Sign::Positive)));
    assert_eq!(down.atanh(&neg, None), Err(FpError::Underflow(Sign::Negative)));
    assert_eq!(down.cosh(&x, None), Ok(Inexact(FBig::ONE, NoOp)));
    let (s, c) = down.sinh_cosh(&x, None);
    assert_eq!(s, Err(FpError::Underflow(Sign::Positive)));
    assert_eq!(c, Ok(Inexact(FBig::ONE, NoOp)));
}

/// `asinh(x) = x − x³/6 + 3x⁵/40 − …`, and for `x = c·3^-100` in base 3 the first two terms
/// add up to an exact midpoint of two 255-digit floats (`3^54/2` is a half-integer). The result
/// is that midpoint plus `3x⁵/40`, a near-tie that takes about twice the precision to resolve —
/// and that the round-to-nearest modes could not resolve at any working precision: the loop did
/// not return (the half-ulp bound of an odd base stops `3^-129` ulps short of the midpoint).
#[test]
fn test_asinh_near_tie_in_odd_base() {
    use dashu_base::Approximation::Inexact;
    use dashu_float::round::Rounding::AddOne;
    let three = IBig::from(3);
    // (c, the half-integer part c³·3^k/2 that x³/6 removes, the exponent of the last place)
    let cases = [
        (1, three.pow(54), -355isize),
        (7, IBig::from(343) * three.pow(52), -353),
    ];
    for (c, odd, last) in cases {
        let x = Repr::<3>::new(IBig::from(c), -100);
        // in units of the last place: x − x³/6 = c·3^(−100 − last) − odd/2
        let below = IBig::from(c) * three.pow((-100 - last) as usize) - (&odd + IBig::ONE) / 2;
        let above = &below + IBig::ONE;
        let f = |sig: &IBig| FBig::<mode::HalfEven, 3>::from_parts(sig.clone(), last);
        // just above the midpoint: the nearest modes round up, towards zero truncates
        assert_eq!(
            Context::<mode::HalfEven>::new(255).asinh(&x, None),
            Ok(Inexact(f(&above), AddOne)),
            "c = {c}"
        );
        assert_eq!(
            Context::<mode::HalfAway>::new(255)
                .asinh(&x, None)
                .unwrap()
                .value()
                .repr(),
            f(&above).repr()
        );
        assert_eq!(
            Context::<mode::Down>::new(255)
                .asinh(&x, None)
                .unwrap()
                .value()
                .repr(),
            f(&below).repr()
        );
        // odd: the mirror image, with the flag of a negative value
        let neg = Context::<mode::HalfEven>::new(255)
            .asinh(&-x.clone(), None)
            .unwrap();
        assert_eq!(neg.value_ref().repr(), (-f(&above)).repr());
        // one digit more or less and it is an ordinary rounding
        for p in [200usize, 254, 256] {
            let narrow = Context::<mode::HalfEven>::new(p).asinh(&x, None).unwrap();
            let wide = Context::<mode::HalfEven>::new(700).asinh(&x, None).unwrap();
            assert_eq!(narrow.value(), wide.value().with_precision(p).value(), "p = {p}");
        }
    }
}

/// The `Rounding` flag of a `tanh` that saturates next to `±1` is the adjustment applied to
/// the truncated significand, as everywhere else: `tanh(13)` at 10 digits towards zero is the
/// truncation `0.9999999999` itself — `NoOp`, like `tanh(12)` one step below the saturation
/// shortcut — and a result carried up to `±1` is `AddOne` / `SubOne`.
#[test]
fn test_tanh_saturation_flag() {
    use dashu_base::Approximation::Inexact;
    use dashu_float::round::Rounding::{AddOne, NoOp, SubOne};
    type Z = FBig<mode::Zero, 10>;
    let nines = Z::from_str("0.9999999999").unwrap();
    let ctx = Context::<mode::Zero>::new(10);
    for s in ["12", "13", "100", "1e30"] {
        let x = Z::from_str(s).unwrap();
        assert_eq!(ctx.tanh(x.repr(), None), Ok(Inexact(nines.clone(), NoOp)), "tanh({s})");
        let neg = -x;
        assert_eq!(ctx.tanh(neg.repr(), None), Ok(Inexact(-nines.clone(), NoOp)), "tanh(-{s})");
    }
    // 400 digits: 461.0 is below the shortcut's threshold, 461.4 above it
    let ctx = Context::<mode::Zero>::new(400);
    let nines = Z::from_str(&format!("0.{}", "9".repeat(400))).unwrap();
    for s in ["461.0", "461.4"] {
        let x = Z::from_str(s).unwrap();
        assert_eq!(ctx.tanh(x.repr(), None), Ok(Inexact(nines.clone(), NoOp)), "tanh({s})");
    }
    // the modes that carry the magnitude up to 1 (1 − tanh(12.5) = 2.8e-11, below half an ulp;
    // 12.5 is still on the Ziv path, 13 past the shortcut's threshold)
    type H = FBig<mode::HalfEven, 10>;
    let ctx = Context::<mode::HalfEven>::new(10);
    for s in ["12.5", "13", "100"] {
        let x = H::from_str(s).unwrap();
        assert_eq!(ctx.tanh(x.repr(), None), Ok(Inexact(H::ONE, AddOne)), "tanh({s})");
        let neg = -x;
        assert_eq!(ctx.tanh(neg.repr(), None), Ok(Inexact(H::NEG_ONE, SubOne)), "tanh(-{s})");
    }
    // base 2 at each of the standard precisions, both directed modes
    for p in [20usize, 50, 100, 500] {
        let x = Repr::<2>::new(IBig::from(1755648), 0);
        let ones = FBig::from_parts((IBig::ONE << p) - IBig::ONE, -(p as isize));
        let down = Context::<mode::Down>::new(p);
        assert_eq!(down.tanh(&x, None), Ok(Inexact(ones, NoOp)), "p = {p}");
        assert_eq!(down.tanh(&-x.clone(), None), Ok(Inexact(FBig::NEG_ONE, SubOne)));
        let up = Context::<mode::Up>::new(p);
        assert_eq!(up.tanh(&x, None), Ok(Inexact(FBig::ONE, AddOne)));
    }
}

/// `sinh_pi` / `cosh_pi` of a large argument at a low precision: the exponent `π·x` formed at
/// the working precision was uncertain by whole units (`x = 2^45` at 1 bit: by `2^39`), and the
/// resulting radius — astronomically larger than the value — ran the certification out of
/// memory. The working precision now covers the argument's integer digits, and an argument
/// past the exponent range is reported as an overflow at once.
#[test]
fn test_sinh_cosh_pi_large_argument() {
    use dashu_float::FpError;
    for e in [30isize, 38, 45, 60] {
        let x = Repr::<2>::new(IBig::from(1), e);
        // the reference: 100 bits, truncated once more (truncations compose)
        let wide = Context::<mode::Zero>::new(100);
        let (sh_wide, ch_wide) = wide.sinh_cosh_pi(&x, None);
        let (sh_wide, ch_wide) = (sh_wide.unwrap().value(), ch_wide.unwrap().value());
        // sinh(π·2^e) = 2^(π·2^e/ln 2 − 1): the leading bit is in the right place
        let lead = (sh_wide.repr().exponent() + sh_wide.repr().digits() as isize) as f64;
        let want = core::f64::consts::PI * 2f64.powi(e as i32) / core::f64::consts::LN_2;
        assert!((lead - want).abs() <= want * 1e-15 + 2.0, "2^{e}: {lead} vs {want}");
        for p in [1usize, 5, 20, 50] {
            let ctx = Context::<mode::Zero>::new(p);
            let sh = ctx.sinh_pi(&x, None).unwrap().value();
            let ch = ctx.cosh_pi(&x, None).unwrap().value();
            assert_eq!(sh, sh_wide.clone().with_precision(p).value(), "sinh_pi(2^{e}) at {p}");
            assert_eq!(ch, ch_wide.clone().with_precision(p).value(), "cosh_pi(2^{e}) at {p}");
            let (sh2, ch2) = ctx.sinh_cosh_pi(&x, None);
            assert_eq!((sh2.unwrap().value(), ch2.unwrap().value()), (sh.clone(), ch));
            let neg = ctx.sinh_pi(&-x.clone(), None).unwrap().value();
            assert_eq!(neg, -sh);
        }
    }
    // past the exponent range, however far: an overflow, not an attempt to reduce the argument
    for e in [62isize, 70, 100_000, 1 << 40, isize::MAX - 10] {
        let x = Repr::<2>::new(IBig::from(1), e);
        let neg = -x.clone();
        for p in [1usize, 53] {
            let ctx = Context::<mode::Zero>::new(p);
            let pos_overflow = Err(FpError::Overflow(Sign::Positive));
            let neg_overflow = Err(FpError::Overflow(Sign::Negative));
            assert_eq!(ctx.sinh_pi(&x, None), pos_overflow, "sinh_pi(2^{e})");
            assert_eq!(ctx.sinh_pi(&neg, None), neg_overflow);
            assert_eq!(ctx.cosh_pi(&neg, None), pos_overflow);
            assert_eq!(ctx.sinh_cosh_pi(&neg, None), (neg_overflow.clone(), pos_overflow.clone()));
            if e >= 100 {
                // sinh/cosh themselves overflow from 2^63·ln 2 on
                assert_eq!(ctx.sinh(&neg, None), neg_overflow);
                assert_eq!(ctx.cosh(&x, None), pos_overflow);
                assert_eq!(ctx.sinh_cosh(&x, None), (pos_overflow.clone(), pos_overflow));
            }
        }
    }
}

/// `asinh` / `acosh` of a huge argument are `ln(2|x|)` to within `1/(2x²)`. The textbook
/// compositions used to be formed regardless — `x² + 1` and `1 + (x + …)` aligned a unit
/// against the argument digit for digit, and ran out of memory from `2^(2^40)` on.
#[test]
fn test_asinh_acosh_huge_argument() {
    type F = FBig<mode::HalfEven, 2>;
    for e in [
        1isize << 20,
        1 << 40,
        isize::MAX / 2 - 100,
        isize::MAX - 100,
    ] {
        for p in [5usize, 20, 50, 100, 500] {
            for sig in [1, 5] {
                let x = F::from_parts(IBig::from(sig), e).with_precision(p).value();
                let ln_2x = F::from_parts(IBig::from(2 * sig), e)
                    .with_precision(p)
                    .value()
                    .ln();
                assert_eq!(x.asinh(), ln_2x, "asinh({sig}·2^{e}) at {p} bits");
                assert_eq!(x.acosh(), ln_2x, "acosh({sig}·2^{e}) at {p} bits");
                assert_eq!((-x).asinh(), -ln_2x, "asinh(-{sig}·2^{e}) at {p} bits");
            }
        }
    }
    // Around the magnitude where the asymptotic form takes over (x² past the working
    // precision), every mode agrees with the same call at a precision wide enough to stay on
    // the general composition, rounded once more.
    fn check<R: dashu_float::round::ErrorBounds>(sig: i32, e: isize, p: usize) {
        let x = Repr::<2>::new(IBig::from(sig), e);
        let wide = Context::<mode::HalfEven>::new(p + 2 * e as usize + 200);
        let narrow = Context::<R>::new(p);
        let reround = |v: FBig<mode::HalfEven, 2>| {
            FBig::<R, 2>::from_repr(v.into_repr(), Context::new(0))
                .with_precision(p)
                .value()
        };
        let asinh = narrow.asinh(&x, None).unwrap().value();
        assert_eq!(asinh, reround(wide.asinh(&x, None).unwrap().value()), "asinh {sig}·2^{e} @{p}");
        let neg = narrow.asinh(&-x.clone(), None).unwrap().value();
        assert_eq!(neg, reround(wide.asinh(&-x.clone(), None).unwrap().value()));
        if sig > 0 {
            let acosh = narrow.acosh(&x, None).unwrap().value();
            assert_eq!(
                acosh,
                reround(wide.acosh(&x, None).unwrap().value()),
                "acosh {sig}·2^{e} @{p}"
            );
        }
    }
    for p in [20usize, 50, 100, 500] {
        for e in [p / 2, p / 2 + 20, p / 2 + 40, p, 2 * p] {
            for sig in [1, 3, 7] {
                check::<mode::HalfEven>(sig, e as isize, p);
                check::<mode::Down>(sig, e as isize, p);
                check::<mode::Up>(sig, e as isize, p);
                check::<mode::Zero>(sig, e as isize, p);
            }
        }
    }
}

#[test]
fn test_infinities() {
    let ctx = Context::<mode::HalfEven>::new(30);
    let one = DBig::from_str("1").unwrap();

    // sinh(±inf) = ±inf
    let r = ctx
        .sinh::<10>(&Repr::<10>::infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);
    let r = ctx
        .sinh::<10>(&Repr::<10>::neg_infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Negative);

    // cosh(±inf) = +inf
    let r = ctx
        .cosh::<10>(&Repr::<10>::neg_infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);

    // tanh(±inf) = ±1 (finite values, not infinities)
    assert!(!ctx
        .tanh::<10>(&Repr::<10>::infinity(), None)
        .unwrap()
        .value()
        .repr()
        .is_infinite());
    assert_eq!(
        ctx.tanh::<10>(&Repr::<10>::infinity(), None)
            .unwrap()
            .value(),
        one.clone()
    );
    assert_eq!(
        ctx.tanh::<10>(&Repr::<10>::neg_infinity(), None)
            .unwrap()
            .value(),
        -one
    );

    // asinh(±inf) = ±inf
    let r = ctx
        .asinh::<10>(&Repr::<10>::neg_infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Negative);

    // acosh(+inf) = +inf
    let r = ctx
        .acosh::<10>(&Repr::<10>::infinity(), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);
}

#[test]
fn test_pythagorean_identity() {
    let x = DBig::from_str("1.234").unwrap().with_precision(30).value();
    let s = x.sinh();
    let c = x.cosh();
    let resid = (&c * &c - &s * &s - DBig::ONE).abs();
    assert!(resid < DBig::from_parts(IBig::from(1), -27), "cosh²-sinh²-1 = {resid}");
}

#[test]
fn test_round_trips() {
    let x = DBig::from_str("1.5").unwrap().with_precision(30).value();
    let tol = DBig::from_parts(IBig::from(1), -27);
    assert!((x.sinh().asinh() - &x).abs() < tol);
    assert!((x.cosh().acosh() - &x).abs() < tol);
    assert!((x.tanh().atanh() - &x).abs() < tol);
}

#[test]
fn test_acosh_one_is_zero() {
    let ctx = Context::<mode::HalfEven>::new(30);
    let r = ctx.acosh::<10>(&Repr::<10>::one(), None).unwrap().value();
    assert!(r.repr().is_pos_zero());
}

/// `acosh` just above 1 under a context narrower than the argument: `x − 1` is all rounding
/// error at the first working precision, which used to trip a debug assertion in the ball
/// square root (`1 + 5/4096` at 1 bit, `1 + 11e-43` at 11 digits).
#[test]
fn test_acosh_just_above_one_wide_argument() {
    let x = Repr::<2>::new(IBig::from(4101), -12); // 1 + 5/4096, acosh ≈ 0.0494
    let r = Context::<mode::HalfEven>::new(1)
        .acosh(&x, None)
        .unwrap()
        .value();
    assert_eq!(r.repr(), &Repr::<2>::new(IBig::ONE, -4));
    let r = Context::<mode::HalfEven>::new(2)
        .acosh(&x, None)
        .unwrap()
        .value();
    assert_eq!(r.repr(), &Repr::<2>::new(IBig::from(3), -6));

    // 1 + 11e-43: acosh(1 + δ) = √(2δ)·(1 − δ/12 + …) = √22·1e-21.5
    let sig: IBig = "10000000000000000000000000000000000000000011"
        .parse()
        .unwrap();
    let x = Repr::<10>::new(sig, -43);
    let r11 = Context::<mode::HalfEven>::new(11)
        .acosh(&x, None)
        .unwrap()
        .value();
    let r12 = Context::<mode::HalfEven>::new(12)
        .acosh(&x, None)
        .unwrap()
        .value();
    assert_eq!(r12.repr(), &Repr::<10>::new(IBig::from(148323969742u64), -32));
    assert_eq!(r11.repr(), &Repr::<10>::new(IBig::from(14832396974u64), -31));

    // a small sweep against a precision that holds the argument exactly, rounded once more
    for p in [20usize, 50, 100] {
        for k in [p + 10, 2 * p + 1, 3 * p - 1] {
            for m in [1u8, 3, 5, 7] {
                let x = Repr::<2>::new((IBig::ONE << k) + IBig::from(m), -(k as isize));
                let narrow = Context::<mode::HalfEven>::new(p)
                    .acosh(&x, None)
                    .unwrap()
                    .value();
                let wide = Context::<mode::HalfEven>::new(p + k + 80)
                    .acosh(&x, None)
                    .unwrap()
                    .value();
                assert_eq!(narrow, wide.with_precision(p).value(), "p={p} k={k} m={m}");
            }
        }
    }
}

#[test]
fn test_atanh_one_is_infinity() {
    let ctx = Context::<mode::HalfEven>::new(30);
    let r = ctx.atanh::<10>(&Repr::<10>::one(), None).unwrap().value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Positive);
    let r = ctx
        .atanh::<10>(&Repr::new(IBig::from(-1), 0), None)
        .unwrap()
        .value();
    assert!(r.repr().is_infinite() && r.repr().sign() == Sign::Negative);

    // the pole is an exact row: it needs no precision to round to, so the unlimited-precision
    // constants reach it too (they used to panic with "precision cannot be 0")
    type F = FBig<mode::HalfEven, 2>;
    assert_eq!(F::ONE.precision(), 0);
    assert_eq!(F::ONE.atanh(), F::INFINITY);
    assert_eq!(F::NEG_ONE.atanh(), F::NEG_INFINITY);
    assert_eq!(DBig::ONE.atanh(), DBig::INFINITY);
    assert_eq!(DBig::NEG_ONE.atanh(), DBig::NEG_INFINITY);
}

/// Anything but the exact rows still needs a precision.
#[test]
#[should_panic(expected = "precision cannot be 0")]
fn test_atanh_unlimited_precision_inexact_panics() {
    let half = FBig::<mode::HalfEven, 2>::from_parts(1.into(), -1)
        .with_precision(0)
        .value();
    let _ = half.atanh();
}

#[test]
#[should_panic]
fn test_acosh_out_of_domain_panics() {
    let _ = DBig::from_str("0.5").unwrap().acosh();
}

#[test]
#[should_panic]
fn test_atanh_out_of_domain_panics() {
    let _ = DBig::from_str("2").unwrap().atanh();
}
