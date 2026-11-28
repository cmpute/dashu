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
        (fbig!(0x100), fbig!(-0x1p-10), fbig!(0xfffp-4), SubOne),
        (fbig!(0x100), fbig!(-0x1p-100), fbig!(0xfffp-4), SubOne),
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
        (dbig!(100), dbig!(-2e-2), dbig!(100), NoOp),
        (dbig!(100), dbig!(-5e-2), dbig!(100), NoOp),
        (dbig!(100), dbig!(-8e-2), dbig!(999e-1), SubOne),
        (dbig!(100), dbig!(-1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(-1e-100), dbig!(100), NoOp),
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
        (fbig!(0x100), fbig!(0x1p-10), fbig!(0xfffp-4), SubOne),
        (fbig!(0x100), fbig!(0x1p-100), fbig!(0xfffp-4), SubOne),
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
        (dbig!(100), dbig!(1e-10), dbig!(100), NoOp),
        (dbig!(100), dbig!(1e-100), dbig!(100), NoOp),
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

/// Regression for [#127](https://github.com/cmpute/dashu/issues/127): an exponent gap wider than
/// `isize::MAX`.
///
/// The alignment subtracted the two exponents in `isize`. Two individually valid exponents can be
/// up to `usize::MAX` apart, so the subtraction overflowed: debug builds panicked on it, and in
/// release builds the wrapped position arithmetic (`ediff + ldigits`) could send the alignment
/// down a path that materializes the gap.
///
/// Whatever the gap, the small operand lies entirely below the rounding window, so the exact sum
/// sits strictly between two values that differ from the large operand by one unit in its last
/// working digit, and the rounding mode picks one of them. For `a·B^e1` with `d` digits, the unit
/// is `B^(e1−256+d)` for a sum and, one guard digit wider, `B^(e1−257+d)` for a difference.
///
/// The large operand has one or two digits: the position `ediff + ldigits` overflows `usize`
/// only from two digits on, at the widest gaps.
#[test]
fn test_add_sub_exponent_gap_beyond_isize() {
    use dashu_base::Sign::{Negative, Positive};
    use dashu_float::round::{mode, Round, Rounding, Rounding::*};
    use dashu_float::{FBig, FpError, Repr, Word};
    use dashu_int::{IBig, UBig};
    use Neighbor::{Above, Below, Mid};

    // The large operand, or the value one unit in its last working digit above or below it.
    #[derive(Clone, Copy)]
    enum Neighbor {
        Mid,
        Above,
        Below,
    }

    // `precision` is the number of working digits: 256, or 257 with the subtraction guard digit.
    fn neighbor<const B: Word>(
        neighbor: Neighbor,
        neg: bool,
        a: &IBig,
        e1: isize,
        precision: usize,
    ) -> Repr<B> {
        let base = UBig::from_word(B);
        let d = a.ilog(&base) + 1; // digits of `a`
        let unit = IBig::from(base.pow(precision - d));
        let offset = precision as isize - d as isize;
        let (significand, exponent) = match neighbor {
            Neighbor::Mid => (a.clone(), e1),
            Neighbor::Above => (a * &unit + IBig::ONE, e1 - offset),
            Neighbor::Below => (a * &unit - IBig::ONE, e1 - offset),
        };
        Repr::new(if neg { -significand } else { significand }, exponent)
    }

    // The adjustment that rounds to `neighbor`.
    fn rounding(neighbor: Neighbor, neg: bool) -> Rounding {
        match (neighbor, neg) {
            (Neighbor::Mid, _) => NoOp,
            (Neighbor::Above, false) | (Neighbor::Below, true) => AddOne,
            (Neighbor::Below, false) | (Neighbor::Above, true) => SubOne,
        }
    }

    // Expected values of `large + small`, `large − small` and `small − large`.
    fn check<R: Round, const B: Word>(
        name: &str,
        exponents: (isize, isize),
        a: &IBig,
        b: &IBig,
        expected: [(Neighbor, bool); 3],
    ) {
        let (e1, e2) = exponents;
        let (large, small) = (Repr::<B>::new(a.clone(), e1), Repr::<B>::new(b.clone(), e2));
        // a sum has no guard digit; the two differences work one digit wider
        let expected = [(expected[0], 256), (expected[1], 257), (expected[2], 257)].map(
            |((n, neg), precision)| (neighbor::<B>(n, neg, a, e1, precision), rounding(n, neg)),
        );
        let context = Context::<R>::new(256);

        let cases = [
            (context.add(&large, &small), &expected[0], "large + small"),
            (context.add(&small, &large), &expected[0], "small + large"),
            (context.sub(&large, &small), &expected[1], "large − small"),
            (context.sub(&small, &large), &expected[2], "small − large"),
        ];
        for (result, (want, rnd), op) in cases {
            assert_eq!(
                result.map(|v| v.map(FBig::into_repr)),
                Ok(Inexact(want.clone(), *rnd)),
                "{name}: {op}"
            );
        }

        // the operators take the precision from the operands
        let x = FBig::<R, B>::from_parts(a.clone(), e1)
            .with_precision(256)
            .value();
        let y = FBig::<R, B>::from_parts(b.clone(), e2);
        let z = |i: usize| {
            let (significand, exponent) = expected[i].0.clone().into_parts();
            FBig::<R, B>::from_parts(significand, exponent)
        };
        test_add(&x, &y, &z(0));
        test_sub(&x, &y, &z(1));
        test_sub(&y, &x, &z(2));
    }

    // Expected values of (large + small, large − small, small − large) under each rounding mode.
    // The exact results are `mid + δ`, `mid − δ` and `−mid + δ` for a `δ` far below the unit in
    // the last working digit, so each of them lies between two adjacent values on that grid and
    // the mode picks one by its own direction: `Up`/`Down` take the upper/lower value, `Zero` and
    // `Away` the one of smaller/larger magnitude, and the nearest modes round back to `±mid`.
    macro_rules! check_modes {
        ($base:literal, $name:expr, $exponents:expr, $a:expr, $b:expr) => {{
            check::<mode::Zero, $base>(
                $name,
                $exponents,
                $a,
                $b,
                [(Mid, false), (Below, false), (Below, true)],
            );
            check::<mode::Away, $base>(
                $name,
                $exponents,
                $a,
                $b,
                [(Above, false), (Mid, false), (Mid, true)],
            );
            check::<mode::Up, $base>(
                $name,
                $exponents,
                $a,
                $b,
                [(Above, false), (Mid, false), (Below, true)],
            );
            check::<mode::Down, $base>(
                $name,
                $exponents,
                $a,
                $b,
                [(Mid, false), (Below, false), (Mid, true)],
            );
            check::<mode::HalfEven, $base>(
                $name,
                $exponents,
                $a,
                $b,
                [(Mid, false), (Mid, false), (Mid, true)],
            );
            check::<mode::HalfAway, $base>(
                $name,
                $exponents,
                $a,
                $b,
                [(Mid, false), (Mid, false), (Mid, true)],
            );
        }};
    }

    let one = IBig::from(1);
    // a small operand far wider than one digit is still below the window
    let wide = (IBig::ONE << 100) - IBig::ONE;
    for (name, exponents) in [
        ("issue reproducer", (isize::MAX / 2 + 1, isize::MIN / 2)),
        ("widest gap with a finite result", (isize::MAX - 1, isize::MIN)),
    ] {
        for a in [IBig::from(1), IBig::from(3)] {
            for b in [&one, &wide] {
                check_modes!(2, name, exponents, &a, b);
            }
        }
    }
    // Decimal floats materialize powers of the base on the alignment paths that the gap must
    // never reach.
    for (name, exponents) in [
        ("issue reproducer", (isize::MAX / 2 + 1, isize::MIN / 2)),
        ("widest gap with a finite result", (isize::MAX - 1, isize::MIN)),
    ] {
        check_modes!(10, name, exponents, &IBig::from(3), &one);
    }

    // At the widest gap possible (`isize::MAX` over `isize::MIN`) the large operand itself sits
    // on the `+inf` sentinel exponent, so a result that rounds back to it is reported as an
    // overflow by the result layer, like any other value landing on the sentinel.
    let (large, small) = (
        Repr::<2>::new(IBig::from(3), isize::MAX),
        Repr::<2>::new(IBig::from(1), isize::MIN),
    );
    let context = Context::<mode::HalfEven>::new(256);
    assert_eq!(context.add(&large, &small), Err(FpError::Overflow(Positive)));
    assert_eq!(context.add(&small, &large), Err(FpError::Overflow(Positive)));
    assert_eq!(context.sub(&large, &small), Err(FpError::Overflow(Positive)));
    assert_eq!(context.sub(&small, &large), Err(FpError::Overflow(Negative)));
}
