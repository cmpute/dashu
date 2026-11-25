use core::{
    fmt::Debug,
    ops::{Add, AddAssign, Sub, SubAssign},
};
use dashu_base::Approximation::*;
use dashu_float::{round::Rounding::*, Context};

mod helper_macros;

/// Test a + b = c in various ways.
fn test_add<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Add<T, Output = T>,
    T: Add<&'a T, Output = T>,
    &'a T: Add<T, Output = T>,
    &'a T: Add<&'a T, Output = T>,
    T: AddAssign<T>,
    T: AddAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a + b, *c);
    assert_eq!(a.clone() + b, *c);
    assert_eq!(a + b.clone(), *c);
    assert_eq!(a.clone() + b.clone(), *c);

    let mut x = a.clone();
    x += b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x += b.clone();
    assert_eq!(x, *c);
}

/// Test a - b = c in various ways.
fn test_sub<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Sub<T, Output = T>,
    T: Sub<&'a T, Output = T>,
    &'a T: Sub<T, Output = T>,
    &'a T: Sub<&'a T, Output = T>,
    T: SubAssign<T>,
    T: SubAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a - b, *c);
    assert_eq!(a.clone() - b, *c);
    assert_eq!(a - b.clone(), *c);
    assert_eq!(a.clone() - b.clone(), *c);

    let mut x = a.clone();
    x -= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x -= b.clone();
    assert_eq!(x, *c);
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_add_binary() {
    // cases without rounding
    let exact_cases = [
        (fbig!(0), fbig!(0), fbig!(0)),
        (fbig!(0), fbig!(1), fbig!(1)),
        (fbig!(0), fbig!(-1), fbig!(-1)),
        (fbig!(1), fbig!(-1), fbig!(0)),
        (fbig!(0x001), fbig!(0x100), fbig!(0x101)),
        (fbig!(0x00001p8), fbig!(0x00001p-8), fbig!(0x10001p-8)),
        (fbig!(0x123p2), fbig!(-0x123p2), fbig!(0)),
        (fbig!(0x123p2), fbig!(-0x023p2), fbig!(0x1p10)),
        (fbig!(0x123p2), fbig!(-0x234p-2), fbig!(0xffcp-2)),
        (fbig!(0x100), fbig!(-0x001p-1), fbig!(0x1ffp-1)),
        (fbig!(0x100), fbig!(0x001p-1), fbig!(0x201p-1)),
        (fbig!(0xff), fbig!(0x01), fbig!(0x1p8)),
    ];
    for (a, b, c) in &exact_cases {
        test_add(a, b, c);
        test_add(b, a, c);
        test_sub(c, a, b);
        test_sub(c, b, a);

        let context = Context::max(a.context(), b.context());
        match (
            context.add(a.repr(), b.repr()),
            context.sub(c.repr(), a.repr()),
            context.sub(c.repr(), b.repr()),
        ) {
            (Ok(Exact(vc)), Ok(Exact(vb)), Ok(Exact(va))) => {
                assert_eq!(va, *a);
                assert_eq!(vb, *b);
                assert_eq!(vc, *c);
            }
            _ => panic!("the result should be exact!"),
        }
    }

    // cases with rounding
    let inexact_cases = [
        (fbig!(0x100), fbig!(0x1p-10), fbig!(0x100), NoOp),
        (fbig!(0x100), fbig!(0x1p-100), fbig!(0x100), NoOp),
        // (rounding toward zero never adjusts the truncated significand, also when the
        // truncation comes from an effective subtraction)
        (fbig!(0x100), fbig!(-0x1p-10), fbig!(0xfffp-4), NoOp),
        (fbig!(0x100), fbig!(-0x1p-100), fbig!(0xfffp-4), NoOp),
        (fbig!(0xff), fbig!(0x1p-1), fbig!(0xff), NoOp),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_add(a, b, c);
        test_add(b, a, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).add(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_add_decimal() {
    // cases without rounding
    let exact_cases = [
        (dbig!(0), dbig!(0), dbig!(0)),
        (dbig!(0), dbig!(1), dbig!(1)),
        (dbig!(0), dbig!(-1), dbig!(-1)),
        (dbig!(1), dbig!(-1), dbig!(0)),
        (dbig!(1), dbig!(100), dbig!(101)),
        (dbig!(00001e2), dbig!(00001e-2), dbig!(10001e-2)),
        (dbig!(123e2), dbig!(-123e2), dbig!(0)),
        (dbig!(995), dbig!(005), dbig!(100e1)),
        // effective subtractions whose smaller operand reaches the precision
        // window edge: formed exactly at full width, the result carries one guard
        // digit (precision+1) and is exact — see AGENTS.md on the guard digit.
        (dbig!(100), dbig!(-12e-2), dbig!(9988e-2)),
        (dbig!(100), dbig!(-15e-2), dbig!(9985e-2)),
        (dbig!(100), dbig!(-18e-2), dbig!(9982e-2)),
    ];

    for (a, b, c) in &exact_cases {
        test_add(a, b, c);
        test_add(b, a, c);
        test_sub(c, a, b);
        test_sub(c, b, a);

        let context = Context::max(a.context(), b.context());
        match (
            context.add(a.repr(), b.repr()),
            context.sub(c.repr(), a.repr()),
            context.sub(c.repr(), b.repr()),
        ) {
            (Ok(Exact(vc)), Ok(Exact(vb)), Ok(Exact(va))) => {
                assert_eq!(va, *a);
                assert_eq!(vb, *b);
                assert_eq!(vc, *c);
            }
            _ => panic!("the result should be exact!"),
        }
    }

    // cases with rounding
    let inexact_cases = [
        (dbig!(100), dbig!(2e-1), dbig!(100), NoOp),
        (dbig!(100), dbig!(5e-1), dbig!(101), AddOne),
        (dbig!(100), dbig!(8e-1), dbig!(101), AddOne),
        (dbig!(100), dbig!(12e-1), dbig!(101), NoOp),
        (dbig!(100), dbig!(15e-1), dbig!(102), AddOne),
        (dbig!(100), dbig!(18e-1), dbig!(102), AddOne),
        (dbig!(100), dbig!(1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(1e-100), dbig!(100), NoOp),
        // effective subtractions: the flag is relative to the truncated exact value
        // (99.98 -> 99.9|8 rounds up to 100.0), as for every other operation
        (dbig!(100), dbig!(-2e-2), dbig!(100), AddOne),
        (dbig!(100), dbig!(-5e-2), dbig!(100), AddOne),
        (dbig!(100), dbig!(-8e-2), dbig!(999e-1), NoOp),
        (dbig!(100), dbig!(-1e-10), dbig!(100), AddOne),
        (dbig!(100), dbig!(-1e-100), dbig!(100), AddOne),
        (dbig!(995), dbig!(8), dbig!(100e1), NoOp),
        (dbig!(995), dbig!(10), dbig!(101e1), AddOne),
        (dbig!(995), dbig!(13), dbig!(101e1), AddOne),
        (dbig!(999), dbig!(2e-1), dbig!(999), NoOp),
        (dbig!(999), dbig!(5e-1), dbig!(1e3), AddOne),
        (dbig!(999), dbig!(8e-1), dbig!(1e3), AddOne),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_add(a, b, c);
        test_add(b, a, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).add(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_sub_binary() {
    let inexact_cases = [
        (fbig!(0x100), fbig!(0x1p-10), fbig!(0xfffp-4), NoOp),
        (fbig!(0x100), fbig!(0x1p-100), fbig!(0xfffp-4), NoOp),
        (fbig!(0x100), fbig!(-0x1p-10), fbig!(0x100), NoOp),
        (fbig!(0x100), fbig!(-0x1p-100), fbig!(0x100), NoOp),
        (fbig!(0xff), fbig!(-0x1p-1), fbig!(0xff), NoOp),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_sub(a, b, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).sub(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_sub_decimal() {
    // in these cases, the output significand can have one more digit than the precision
    let exact_cases = [
        (dbig!(100), dbig!(2e-1), dbig!(998e-1)),
        (dbig!(101), dbig!(2e-1), dbig!(1008e-1)),
        (dbig!(101), dbig!(5e-1), dbig!(1005e-1)),
        (dbig!(101), dbig!(8e-1), dbig!(1002e-1)),
        (dbig!(101), dbig!(12e-1), dbig!(998e-1)),
        (dbig!(102), dbig!(2e-1), dbig!(1018e-1)),
        (dbig!(100e1), dbig!(8), dbig!(992)),
        (dbig!(101e1), dbig!(13), dbig!(997)),
        (dbig!(999), dbig!(2e-1), dbig!(9988e-1)),
    ];

    for (a, b, c) in &exact_cases {
        test_sub(a, b, c);

        if let Ok(Exact(v)) = Context::max(a.context(), b.context()).sub(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(100), dbig!(1e-10), dbig!(100), AddOne),
        (dbig!(100), dbig!(1e-100), dbig!(100), AddOne),
        (dbig!(100), dbig!(-2e-2), dbig!(100), NoOp),
        (dbig!(100), dbig!(-5e-2), dbig!(100), NoOp),
        (dbig!(999e-1), dbig!(-8e-2), dbig!(100), AddOne),
        (dbig!(999e-1), dbig!(-12e-2), dbig!(100), NoOp),
        (dbig!(999e-1), dbig!(-15e-2), dbig!(100), NoOp),
        (dbig!(998e-1), dbig!(-18e-2), dbig!(100), AddOne),
        (dbig!(100), dbig!(-1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(-1e-100), dbig!(100), NoOp),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_sub(a, b, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).sub(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_add_by_inf() {
    let _ = dashu_float::DBig::ONE + dashu_float::DBig::INFINITY;
}

#[test]
#[should_panic]
fn test_sub_by_inf() {
    let _ = dashu_float::DBig::ONE - dashu_float::DBig::INFINITY;
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_add_sub_unlimited_precision() {
    let a = fbig!(0xffff).with_precision(0).unwrap();
    let b = fbig!(-0xffff).with_precision(0).unwrap();
    test_add(&a, &a, &fbig!(0x1fffe));
    test_add(&a, &b, &fbig!(0x0));
    test_add(&b, &b, &fbig!(-0x1fffe));

    let a = dbig!(9999).with_precision(0).unwrap();
    let b = dbig!(-9999).with_precision(0).unwrap();
    test_add(&a, &a, &dbig!(19998));
    test_add(&a, &b, &dbig!(0));
    test_add(&b, &b, &dbig!(-19998));

    let a = fbig!(0x1p16).with_precision(0).unwrap();
    let b = fbig!(0x1p-16).with_precision(0).unwrap();
    test_add(&a, &fbig!(0x1p-16), &a);
    test_add(&a, &b, &fbig!(0x100000001p-16));

    let a = dbig!(1e4).with_precision(0).unwrap();
    let b = dbig!(1e-4).with_precision(0).unwrap();
    test_add(&a, &dbig!(1e-4), &a);
    test_add(&a, &b, &dbig!(100000001e-4));
}

#[test]
fn test_add_sub_with_rounding() {
    use dashu_float::round::mode::*;

    // rhs is about the magnitude of lhs.ulp()
    let a = fbig!(0x01).with_rounding::<Zero>();
    let b = fbig!(0x1p - 8).with_rounding::<Zero>();
    let c = fbig!(0xffp - 8).with_rounding::<Zero>();
    test_add(&a, &b, &a);
    test_sub(&a, &b, &c);

    let a = fbig!(0x01).with_rounding::<Up>();
    let b = fbig!(0x1p - 8).with_rounding::<Up>();
    let c = fbig!(0x81p - 7).with_rounding::<Up>();
    let d = fbig!(0xffp - 8).with_rounding::<Up>();
    test_add(&a, &b, &c);
    test_sub(&a, &b, &d);

    let a = fbig!(0x01).with_rounding::<Down>();
    let b = fbig!(0x1p - 8).with_rounding::<Down>();
    let c = fbig!(0xffp - 8).with_rounding::<Down>();
    test_add(&a, &b, &a);
    test_sub(&a, &b, &c);

    let a = fbig!(0x01).with_rounding::<HalfAway>();
    let b = fbig!(0x1p - 8).with_rounding::<HalfAway>();
    let c = fbig!(0x81p - 7).with_rounding::<HalfAway>();
    let d = fbig!(0xffp - 8).with_rounding::<HalfAway>();
    test_add(&a, &b, &c);
    test_sub(&a, &b, &d);

    let a = fbig!(0x01).with_rounding::<HalfEven>();
    let b = fbig!(0x1p - 8).with_rounding::<HalfEven>();
    let c = fbig!(0xffp - 8).with_rounding::<HalfEven>();
    test_add(&a, &b, &a);
    test_sub(&a, &b, &c);

    // rhs much less than lhs
    let a = fbig!(0x01).with_rounding::<Zero>();
    let b = fbig!(0x1p - 100).with_rounding::<Zero>();
    let c = fbig!(0xffp - 8).with_rounding::<Zero>();
    test_add(&a, &b, &a);
    test_sub(&a, &b, &c);

    let a = fbig!(0x01).with_rounding::<Up>();
    let b = fbig!(0x1p - 100).with_rounding::<Up>();
    let c = fbig!(0x81p - 7).with_rounding::<Up>();
    test_add(&a, &b, &c);
    test_sub(&a, &b, &a);

    let a = fbig!(0x01).with_rounding::<Down>();
    let b = fbig!(0x1p - 100).with_rounding::<Down>();
    let c = fbig!(0xffp - 8).with_rounding::<Down>();
    test_add(&a, &b, &a);
    test_sub(&a, &b, &c);
}

/// `Context::sub(0, b) = -b` must be rounded with the context's mode applied to `-b` directly.
///
/// An earlier implementation rounded `b` first and then negated. That is only valid for the
/// sign-symmetric modes (Zero, Away, HalfEven, HalfAway); for the asymmetric directed modes
/// `Up` (toward +∞) and `Down` (toward −∞), `round(-x) != -round(x)`, so `0 - b` landed one
/// ULP off. These cases pin the correct rounding for both directions and both signs.
#[test]
fn test_sub_zero_operand_directed_rounding() {
    use dashu_float::round::mode::*;
    use dashu_float::{Context, Repr};
    use dashu_int::IBig;

    let zero = Repr::<10>::new(IBig::from(0), 0);
    let repr = |sig: i64, exp: isize| Repr::<10>::new(IBig::from(sig), exp);

    // b = -1234 → 0 - b = +1234, rounded to precision 2:
    //   Up   (toward +∞): 1300     Down (toward −∞): 1200
    let b = repr(-1234, 0);
    assert_eq!(Context::<Up>::new(2).sub(&zero, &b).unwrap().value().repr(), &repr(13, 2));
    assert_eq!(
        Context::<Down>::new(2)
            .sub(&zero, &b)
            .unwrap()
            .value()
            .repr(),
        &repr(12, 2)
    );

    // b = +1234 → 0 - b = -1234, rounded to precision 2:
    //   Up   (toward +∞): -1200    Down (toward −∞): -1300
    let b = repr(1234, 0);
    assert_eq!(Context::<Up>::new(2).sub(&zero, &b).unwrap().value().repr(), &repr(-12, 2));
    assert_eq!(
        Context::<Down>::new(2)
            .sub(&zero, &b)
            .unwrap()
            .value()
            .repr(),
        &repr(-13, 2)
    );

    // The symmetric `Away` mode is unaffected by the negation order: ±1234 → ±1300.
    let b = repr(-1234, 0);
    assert_eq!(
        Context::<Away>::new(2)
            .sub(&zero, &b)
            .unwrap()
            .value()
            .repr(),
        &repr(13, 2)
    );
    let b = repr(1234, 0);
    assert_eq!(
        Context::<Away>::new(2)
            .sub(&zero, &b)
            .unwrap()
            .value()
            .repr(),
        &repr(-13, 2)
    );
}

// ---------------------------------------------------------------------------
// Regressions for the addition / subtraction reports of the Hegel triage
// (see triage/float-arith.md).
// ---------------------------------------------------------------------------

mod triage {
    use core::cmp::Ordering;
    use dashu_base::{AbsOrd, Approximation::*, Sign};
    use dashu_float::{
        round::{mode, Round, Rounding},
        Context, FBig, FpError, Repr, Word,
    };
    use dashu_int::IBig;

    fn repr<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(sig.into(), exp)
    }

    /// The `Rounding` flag of an inexact sum is the adjustment applied to the exact value
    /// *truncated toward zero* — the convention of every other operation: `NoOp` when the
    /// result is smaller in magnitude than the exact sum, `AddOne` / `SubOne` when a positive /
    /// negative result is larger in magnitude. When the operands partly cancelled, the flag
    /// used to be relative to an intermediate significand instead, so that e.g. `Away` reported
    /// `NoOp` for a result it had rounded away from zero, and `Zero` reported `SubOne`.
    #[test]
    fn test_add_sub_rounding_flag_direction() {
        fn check<R: Round, const B: Word>() {
            let exact_ctx = Context::<R>::new(0);
            let sigs = [1i128, 3, 7, 11, 999, (1 << 60) - 1, 123456789012345678];
            let gaps = [
                0isize, 1, 2, 3, 5, 19, 21, 49, 52, 60, 99, 130, 499, 503, 2000,
            ];
            for precision in [1usize, 2, 20, 50, 53, 100, 500] {
                let ctx = Context::<R>::new(precision);
                for a_sig in sigs {
                    for b_sig in sigs {
                        for gap in gaps {
                            for (a_sign, b_sign) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                                let a = repr::<B>(a_sign * a_sig, gap);
                                let b = repr::<B>(b_sign * b_sig, 0);
                                for (x, y) in [(&a, &b), (&b, &a)] {
                                    let cases = [
                                        (ctx.add(x, y), exact_ctx.add(x, y)),
                                        (ctx.sub(x, y), exact_ctx.sub(x, y)),
                                    ];
                                    for (got, exact) in cases {
                                        let exact = exact.unwrap().value();
                                        check_flag(got.unwrap(), &exact, precision);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        fn check_flag<R: Round, const B: Word>(
            got: dashu_float::round::Rounded<FBig<R, B>>,
            exact: &FBig<R, B>,
            precision: usize,
        ) {
            match got {
                Exact(v) => assert_eq!(v.repr(), exact.repr()),
                Inexact(v, flag) => {
                    // one guard digit at most (see `test_sub_keeps_one_guard_digit`)
                    assert!(v.repr().digits() <= precision + 1);
                    let expected = match (v.abs_cmp(exact), v.sign()) {
                        (Ordering::Less, _) => Rounding::NoOp,
                        (Ordering::Greater, Sign::Positive) => Rounding::AddOne,
                        (Ordering::Greater, Sign::Negative) => Rounding::SubOne,
                        (Ordering::Equal, _) => panic!("inexact flag on an exact result"),
                    };
                    assert_eq!(
                        flag,
                        expected,
                        "exact {:?}, got {:?} at precision {precision}",
                        exact.repr(),
                        v.repr()
                    );
                }
            }
        }

        check::<mode::Zero, 2>();
        check::<mode::Away, 2>();
        check::<mode::Up, 2>();
        check::<mode::Down, 10>();
        check::<mode::HalfEven, 2>();
        check::<mode::HalfAway, 10>();
        check::<mode::Away, 10>();

        // the reported cases
        let ctx = Context::<mode::Away>::new(1);
        let (one, four) = (repr::<2>(1, 0), repr::<2>(1, 2));
        let got = ctx.sub(&one, &four).unwrap(); // -3 rounds away from zero to -4
        assert_eq!(got, Inexact(FBig::from_repr(repr(-1, 2), ctx), Rounding::SubOne));
        let got = ctx.sub(&four, &one).unwrap(); // 3 rounds away from zero to 4
        assert_eq!(got, Inexact(FBig::from_repr(repr(1, 2), ctx), Rounding::AddOne));
        let ctx = Context::<mode::Away>::new(53);
        let got = ctx.sub(&repr::<2>(1, 60), &one).unwrap(); // 2^60 - 1 rounds to 2^60
        assert_eq!(got, Inexact(FBig::from_repr(repr(1, 60), ctx), Rounding::AddOne));
    }

    /// An effective subtraction keeps one guard digit: its result can carry `precision + 1`
    /// digits (the documented exception to the precision limit, see `Repr`), and is reported
    /// `Exact` when those digits hold the difference exactly.
    #[test]
    fn test_sub_keeps_one_guard_digit() {
        let ctx = Context::<mode::Zero>::new(1);
        // 1 - 20 = -19: two digits at precision 1
        let got = ctx.sub(&repr::<10>(1, 0), &repr::<10>(2, 1)).unwrap();
        assert_eq!(got, Exact(FBig::from_repr(repr(-19, 0), ctx)));
        // 11 - 200 = -189, truncated to two digits
        let got = ctx.sub(&repr::<10>(11, 0), &repr::<10>(2, 2)).unwrap();
        assert_eq!(got, Inexact(FBig::from_repr(repr(-18, 1), ctx), Rounding::NoOp));
        // an addition has no guard digit
        let got = ctx.add(&repr::<10>(1, 0), &repr::<10>(2, 1)).unwrap();
        assert_eq!(got, Inexact(FBig::from_repr(repr(2, 1), ctx), Rounding::NoOp));
        // re-rounding gives the plain one-digit value
        let diff = FBig::<mode::Zero, 10>::from_repr(repr(-19, 0), Context::new(0));
        assert_eq!(diff.with_precision(1).value().repr(), &repr(-1, 1));
    }

    /// The operands' exponents can be further apart than `isize::MAX`: the distance is
    /// unsigned. `1·2^1 + 1·2^(isize::MIN+1)` used to panic with "attempt to subtract with
    /// overflow" in debug builds.
    #[test]
    fn test_add_sub_exponent_gap_beyond_isize() {
        fn check<R: Round, const B: Word>() {
            for precision in [20usize, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                let tiny_exps = [isize::MIN + 1, isize::MIN + 2, isize::MIN / 2];
                for (big_exp, tiny_exp) in [
                    (0isize, tiny_exps[0]),
                    (1, tiny_exps[0]),
                    (2, tiny_exps[1]),
                    (isize::MAX - 1, -1),
                    (isize::MAX - 1, tiny_exps[0]),
                    (isize::MAX / 2 + 5, tiny_exps[2]),
                ] {
                    for (big_sig, tiny_sig) in [(1, 1), (1, -1), (-3, 1), (-3, -1)] {
                        let (big, tiny) =
                            (repr::<B>(big_sig, big_exp), repr::<B>(tiny_sig, tiny_exp));
                        // the tiny operand only acts as a sticky digit: the result is the big
                        // operand, or its neighbour toward the tiny one
                        for sum in [ctx.add(&big, &tiny), ctx.add(&tiny, &big)] {
                            let sum = match sum.unwrap() {
                                Inexact(v, _) => v,
                                Exact(v) => panic!("exact: {v:?}"),
                            };
                            let delta = Context::<R>::new(0).sub(sum.repr(), &big).unwrap().value();
                            assert!(
                                delta.repr().significand().is_zero()
                                    || delta.sign() == Sign::from(tiny_sig < 0)
                            );
                            assert!(sum.repr().digits() <= precision + 1);
                            let x = FBig::<R, B>::from_repr(big.clone(), ctx);
                            let y = FBig::<R, B>::from_repr(tiny.clone(), ctx);
                            assert_eq!(&x + &y, sum);
                            assert_eq!(y.clone() + x.clone(), sum);
                            assert_eq!(&x - &(-y), sum);
                        }
                    }
                }
            }
        }
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 10>();
        check::<mode::Down, 10>();
        check::<mode::HalfAway, 10>();

        // the reported case
        let ctx = Context::<mode::HalfEven>::new(20);
        let (big, tiny) = (repr::<2>(1, 1), repr::<2>(1, isize::MIN + 1));
        assert_eq!(ctx.add(&big, &tiny).unwrap().value().repr(), &big);
    }

    /// A sum that leaves the exponent range overflows like every other operation: `+` and
    /// `-` saturate through `Context::unwrap_fp`, exactly as `Context::add` followed by
    /// `unwrap_fp` (and as `*` does). They used to return the out-of-range significand at
    /// the sentinel exponent, a value that is neither infinite nor a valid finite number.
    #[test]
    fn test_add_sub_exponent_overflow_saturates() {
        fn check<R: Round, const B: Word>() {
            let e = isize::MAX - 1;
            for precision in [1usize, 20, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                let b_minus_1 = B as i128 - 1;
                for sign in [1i128, -1] {
                    let a = FBig::<R, B>::from_repr(repr(sign, e), ctx);
                    let b = FBig::<R, B>::from_repr(repr(sign * b_minus_1, e), ctx);
                    // 1·B^e + (B−1)·B^e = B^(e+1): the exponent reaches isize::MAX
                    let result = ctx.add(a.repr(), b.repr());
                    assert_eq!(result, Err(FpError::Overflow(Sign::from(sign < 0))));
                    let expected = ctx.unwrap_fp(result);
                    assert_eq!(&a + &b, expected);
                    assert_eq!(a.clone() + b.clone(), expected);
                    assert_eq!(&a - &(-b.clone()), expected);
                    assert_eq!(a.clone() - (-b.clone()), expected);
                    // the same magnitude through a product
                    let base = FBig::<R, B>::from_repr(repr(B as i128, 0), ctx);
                    assert_eq!(&a * &base, expected);
                    // a sum that does not carry stays finite and exact
                    if B > 2 {
                        let c = FBig::<R, B>::from_repr(repr(sign * (b_minus_1 - 1), e), ctx);
                        assert_eq!((&a + &c).repr(), &repr(sign * b_minus_1, e));
                    }
                }
            }
        }
        check::<mode::HalfEven, 10>();
        check::<mode::HalfAway, 10>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 10>();
        check::<mode::Down, 2>();
        check::<mode::Away, 2>();

        // nearest and outward modes reach the infinity, like the reported case expects
        type F = FBig<mode::HalfEven, 10>;
        let e = isize::MAX - 1;
        let sum = &F::from_parts(IBig::from(1), e) + &F::from_parts(IBig::from(9), e);
        assert!(sum.repr().is_infinite() && sum.sign() == Sign::Positive);
        let diff = &F::from_parts(IBig::from(-1), e) - &F::from_parts(IBig::from(9), e);
        assert!(diff.repr().is_infinite() && diff.sign() == Sign::Negative);
    }

    /// An integer operand (a primitive, `UBig` or `IBig`) is exact: next to an
    /// unlimited-precision float the result stays exact and unlimited. The integer used to
    /// bring the number of its digits as a precision, which — "unlimited" yielding to any
    /// limited precision — became the precision of the result: `12.34 + 1u8` was `10` (F41).
    /// The operators of `*` and `/` share the conversion and are covered here too.
    #[test]
    fn test_integer_operand_keeps_unlimited_precision() {
        use dashu_float::CachedFBig;
        use dashu_int::UBig;
        use std::panic::{catch_unwind, AssertUnwindSafe};

        #[track_caller]
        fn same<R: Round, const B: Word>(got: FBig<R, B>, want: &FBig<R, B>) {
            assert_eq!(got.repr(), want.repr());
            assert_eq!(got.precision(), want.precision());
        }

        fn check<R: Round, const B: Word>() {
            let exact = Context::<R>::new(0);
            let x = FBig::<R, B>::from_repr(repr(1234, -2), exact);
            let cx = CachedFBig::with_cache(x.repr().clone(), exact);

            macro_rules! check_integer {
                ($n:expr) => {{
                    let n = $n;
                    // the same integer as an exact float
                    let f = FBig::<R, B>::from_repr(Repr::from(IBig::from(n.clone())), exact);
                    let (sum, diff, rdiff, prod) = (&x + &f, &x - &f, &f - &x, &x * &f);
                    assert_eq!(sum.precision(), 0);

                    same(&x + n.clone(), &sum);
                    same(x.clone() + &n, &sum);
                    same(n.clone() + &x, &sum);
                    same(&n + x.clone(), &sum);
                    same(&x - n.clone(), &diff);
                    same(x.clone() - &n, &diff);
                    same(n.clone() - &x, &rdiff);
                    same(&n - x.clone(), &rdiff);
                    same(&x * n.clone(), &prod);
                    same(n.clone() * &x, &prod);
                    let mut y = x.clone();
                    y += n.clone();
                    same(y, &sum);
                    let mut y = x.clone();
                    y -= &n;
                    same(y, &diff);
                    let mut y = x.clone();
                    y *= n.clone();
                    same(y, &prod);

                    // the cached wrapper mirrors the operators
                    same((&cx + n.clone()).into_fbig(), &sum);
                    same((n.clone() - &cx).into_fbig(), &rdiff);
                    same((cx.clone() * &n).into_fbig(), &prod);
                    let mut cy = cx.clone();
                    cy -= n.clone();
                    same(cy.into_fbig(), &diff);

                    // a limited precision is combined with the digits of the integer, as
                    // it is for the integer converted with `From`
                    for precision in [20usize, 50, 100, 500] {
                        let xp = x.clone().with_precision(precision).value();
                        let fp = FBig::<R, B>::from(n.clone());
                        same(&xp + n.clone(), &(&xp + &fp));
                        same(n.clone() - &xp, &(&fp - &xp));
                        same(&xp * n.clone(), &(&xp * &fp));
                        same(&xp / 7u8, &(&xp / FBig::<R, B>::from(7u8)));
                        assert_eq!((&xp + n.clone()).precision(), precision.max(fp.precision()));
                    }
                }};
            }
            check_integer!(0u8);
            check_integer!(1u8);
            check_integer!(5i32);
            check_integer!(-77i64);
            check_integer!(usize::MAX);
            check_integer!(u128::MAX);
            check_integer!(i128::MIN);
            check_integer!(UBig::from(123456789u32).pow(9));
            check_integer!(-IBig::from(987654321u32).pow(7));
            same(&x + 0u8, &x);

            // A quotient has no unlimited-precision form in general: dividing an
            // unlimited-precision float by an integer panics like dividing it by an
            // unlimited-precision float does (it used to round to the digits of the integer).
            let four = FBig::<R, B>::from_repr(repr(4, 0), exact);
            assert!(catch_unwind(AssertUnwindSafe(|| &x / &four)).is_err());
            assert!(catch_unwind(AssertUnwindSafe(|| &x / 4u8)).is_err());
            assert!(catch_unwind(AssertUnwindSafe(|| 4u8 / &x)).is_err());
        }
        check::<mode::HalfAway, 10>();
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 10>();
        check::<mode::Up, 3>();

        // the reported case
        use core::str::FromStr;
        let x = dashu_float::DBig::from_str("12.34").unwrap();
        let x = x.with_precision(0).value();
        let sum = &x + 1u8;
        assert_eq!(sum, dashu_float::DBig::from_str("13.34").unwrap());
        assert_eq!(sum.precision(), 0);
        assert_eq!(1u8 - &x, dashu_float::DBig::from_str("-11.34").unwrap());
        let mut y = x.clone();
        y += 5i32;
        assert_eq!(y, dashu_float::DBig::from_str("17.34").unwrap());
    }
}
