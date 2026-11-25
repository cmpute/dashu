use core::{
    fmt::Debug,
    ops::{Div, DivAssign, Rem, RemAssign},
};
use dashu_base::{Approximation::*, DivEuclid, DivRemEuclid, RemEuclid};
use dashu_float::{round::Rounding::*, Context};

mod helper_macros;
#[allow(dead_code)]
mod helpers;

fn test_div<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Div<T, Output = T>,
    T: Div<&'a T, Output = T>,
    &'a T: Div<T, Output = T>,
    &'a T: Div<&'a T, Output = T>,
    T: DivAssign<T>,
    T: DivAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a / b, *c);
    assert_eq!(a.clone() / b, *c);
    assert_eq!(a / b.clone(), *c);
    assert_eq!(a.clone() / b.clone(), *c);

    let mut x = a.clone();
    x /= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x /= b.clone();
    assert_eq!(x, *c);
}

fn test_rem<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Rem<T, Output = T>,
    T: Rem<&'a T, Output = T>,
    &'a T: Rem<T, Output = T>,
    &'a T: Rem<&'a T, Output = T>,
    T: RemAssign<T>,
    T: RemAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a % b, *c);
    assert_eq!(a.clone() % b, *c);
    assert_eq!(a % b.clone(), *c);
    assert_eq!(a.clone() % b.clone(), *c);

    let mut x = a.clone();
    x %= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x %= b.clone();
    assert_eq!(x, *c);
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_div_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(1), fbig!(0)),
        (fbig!(1), fbig!(1), fbig!(1)),
        (fbig!(0x1000), fbig!(0x1000), fbig!(1)),
        (fbig!(0x1000), fbig!(0x10), fbig!(0x100)),
        (fbig!(0x1000), fbig!(-0x10), fbig!(-0x100)),
        (fbig!(-0xffff), fbig!(-0xff), fbig!(0x101)),
        (fbig!(0x1b), fbig!(0x3), fbig!(0x9)),
    ];

    for (a, b, c) in &exact_cases {
        test_div(a, b, c);

        if let Ok(Exact(v)) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (fbig!(0x43), fbig!(0x21), fbig!(0x81p-6)),
        (fbig!(0x654), fbig!(-0x321), fbig!(-0x817p-10)),
        (fbig!(-0x98765), fbig!(-0x43210), fbig!(0x915b1p-18)),
        (fbig!(0x1), fbig!(0x9), fbig!(0xep-7)),
        (fbig!(0x1), fbig!(0x09), fbig!(0xe3p-11)),
        (fbig!(0x1), fbig!(0x009), fbig!(0x1c7p-12)),
        (fbig!(0x13), fbig!(-0x9), fbig!(-0x87p-6)),
        (fbig!(0x169), fbig!(-0x9), fbig!(-0xa07p-6)),
    ];
    for (a, b, c) in &inexact_cases {
        test_div(a, b, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_div_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(1), dbig!(0)),
        (dbig!(1), dbig!(1), dbig!(1)),
        (dbig!(1000), dbig!(1000), dbig!(1)),
        (dbig!(1000), dbig!(10), dbig!(100)),
        (dbig!(1000), dbig!(-10), dbig!(-100)),
        (dbig!(-9999), dbig!(-99), dbig!(101)),
        (dbig!(27), dbig!(3), dbig!(9)),
    ];

    for (a, b, c) in &exact_cases {
        test_div(a, b, c);

        if let Ok(Exact(v)) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(43), dbig!(21), dbig!(2), NoOp),
        (dbig!(654), dbig!(-321), dbig!(-204e-2), SubOne),
        (dbig!(-98765), dbig!(-43210), dbig!(22857e-4), AddOne),
        (dbig!(1), dbig!(9), dbig!(1e-1), NoOp),
        (dbig!(1), dbig!(09), dbig!(11e-2), NoOp),
        (dbig!(1), dbig!(009), dbig!(111e-3), NoOp),
        (dbig!(13), dbig!(-9), dbig!(-14e-1), NoOp),
        (dbig!(169), dbig!(-9), dbig!(-188e-1), SubOne),
        (dbig!(1), dbig!(4), dbig!(3e-1), AddOne),
        (dbig!(1), dbig!(-4), dbig!(-3e-1), SubOne),
    ];
    for (a, b, c, rnd) in &inexact_cases {
        test_div(a, b, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).div(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_div_by_inf() {
    let _ = dashu_float::DBig::ONE / dashu_float::DBig::INFINITY;
}

#[test]
#[should_panic]
fn test_div_by_0() {
    let _ = dashu_float::DBig::ONE / dashu_float::DBig::ZERO;
}

#[test]
#[should_panic]
fn test_div_by_unlimited_precision() {
    let _ = dbig!(1).with_precision(0).unwrap() / dbig!(3).with_precision(0).unwrap();
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_rem_binary() {
    // test cases: n, d, remainder. The binary default type is FBig<Zero, 2>, so the
    // quotient is truncated (fmod-style, remainder keeps the dividend's sign).
    let test_cases = [
        (fbig!(0), fbig!(1), fbig!(0)),
        (fbig!(1), fbig!(1), fbig!(0)),
        (fbig!(0x1000), fbig!(0x1000), fbig!(0)),
        (fbig!(0x1000), fbig!(0x10), fbig!(0)),
        (fbig!(0x1000), fbig!(-0x10), fbig!(0)),
        (fbig!(0x3), fbig!(0x2), fbig!(1)),
        (fbig!(0x3), fbig!(-0x2), fbig!(1)),
        (fbig!(-0x3), fbig!(0x2), fbig!(-1)),
        (fbig!(0x43), fbig!(0x21), fbig!(0x1)),
        (fbig!(0x43), fbig!(0x23), fbig!(0x20)),
        (fbig!(0x654), fbig!(-0x321), fbig!(0x12)),
        (fbig!(-0x98765), fbig!(-0x43210), fbig!(-0x12345)),
        (fbig!(0x1), fbig!(0x9), fbig!(0x1)),
        (fbig!(0x1), fbig!(0x9p-4), fbig!(0x7p-4)),
        (fbig!(0x1), fbig!(0x9p-8), fbig!(0x4p-8)),
        (fbig!(0x13), fbig!(-0x9), fbig!(0x1)),
        (fbig!(0x169), fbig!(-0x9), fbig!(0x1)),
    ];

    for (n, d, r) in &test_cases {
        test_rem(n, d, r);
    }
}

#[test]
fn test_rem_decimal() {
    // test cases: n, d, remainder
    let test_cases = [
        (dbig!(0), dbig!(1), dbig!(0)),
        (dbig!(1), dbig!(1), dbig!(0)),
        (dbig!(1000), dbig!(1000), dbig!(0)),
        (dbig!(1000), dbig!(10), dbig!(0)),
        (dbig!(1000), dbig!(-10), dbig!(0)),
        (dbig!(-9999), dbig!(-99), dbig!(0)),
        (dbig!(3), dbig!(2), dbig!(-1)),
        (dbig!(3), dbig!(-2), dbig!(-1)),
        (dbig!(-3), dbig!(2), dbig!(1)),
        (dbig!(27), dbig!(3), dbig!(0)),
        (dbig!(43), dbig!(21), dbig!(1)),
        (dbig!(43), dbig!(23), dbig!(-3)),
        (dbig!(654), dbig!(-321), dbig!(12)),
        (dbig!(-98765), dbig!(-43210), dbig!(-12345)),
        (dbig!(1), dbig!(9), dbig!(1)),
        (dbig!(1), dbig!(9e-1), dbig!(1e-1)),
        (dbig!(1), dbig!(9e-2), dbig!(1e-2)),
        (dbig!(13), dbig!(-9), dbig!(4)),
        (dbig!(169), dbig!(-9), dbig!(-2)),
    ];

    for (n, d, r) in &test_cases {
        test_rem(n, d, r);
    }
}

// The remainder is `n − k·d` with the quotient `k` rounded to an integer under the
// rounding mode attached to the type. Each pair below is chosen to hit a distinct
// rule: exact multiple, fractional part below/above one half, exact tie with even
// and odd truncated quotient, and a negative quotient.
#[test]
fn test_rem_rounding_modes() {
    use core::str::FromStr;
    use dashu_float::{round::mode, FBig};

    macro_rules! rem_case {
        ($mode:ty, $n:literal, $d:literal, $r:literal) => {
            assert_eq!(
                FBig::<$mode, 10>::from_str($n).unwrap() % FBig::<$mode, 10>::from_str($d).unwrap(),
                FBig::<$mode, 10>::from_str($r).unwrap(),
                "{} % {} under {}",
                $n,
                $d,
                stringify!($mode)
            );
        };
    }

    type Zero = mode::Zero;
    type Away = mode::Away;
    type Down = mode::Down;
    type Up = mode::Up;
    type HalfEven = mode::HalfEven;
    type HalfAway = mode::HalfAway;

    // exact multiple: every mode keeps the quotient
    rem_case!(Zero, "20", "10", "0");
    rem_case!(Away, "20", "10", "0");
    rem_case!(Down, "20", "10", "0");
    rem_case!(Up, "20", "10", "0");
    rem_case!(HalfEven, "20", "10", "0");
    rem_case!(HalfAway, "20", "10", "0");

    // quotient 1.4: truncation keeps it (Zero/Down/nearest), away/ceil step to 2
    rem_case!(Zero, "14", "10", "4");
    rem_case!(Away, "14", "10", "-6");
    rem_case!(Down, "14", "10", "4");
    rem_case!(Up, "14", "10", "-6");
    rem_case!(HalfEven, "14", "10", "4");
    rem_case!(HalfAway, "14", "10", "4");

    // quotient 1.6: every non-truncating mode steps to 2
    rem_case!(Zero, "16", "10", "6");
    rem_case!(Away, "16", "10", "-4");
    rem_case!(Down, "16", "10", "6");
    rem_case!(Up, "16", "10", "-4");
    rem_case!(HalfEven, "16", "10", "-4");
    rem_case!(HalfAway, "16", "10", "-4");

    // tie, truncated quotient 1 (odd): HalfEven steps away with HalfAway
    rem_case!(Zero, "15", "10", "5");
    rem_case!(Away, "15", "10", "-5");
    rem_case!(Down, "15", "10", "5");
    rem_case!(Up, "15", "10", "-5");
    rem_case!(HalfEven, "15", "10", "-5");
    rem_case!(HalfAway, "15", "10", "-5");

    // tie, truncated quotient 2 (even): only HalfEven keeps it
    rem_case!(Zero, "25", "10", "5");
    rem_case!(Away, "25", "10", "-5");
    rem_case!(Down, "25", "10", "5");
    rem_case!(Up, "25", "10", "-5");
    rem_case!(HalfEven, "25", "10", "5");
    rem_case!(HalfAway, "25", "10", "-5");

    // negative quotient −1.5: sign conventions flip per mode
    rem_case!(Zero, "-15", "10", "-5");
    rem_case!(Away, "-15", "10", "5");
    rem_case!(Down, "-15", "10", "5");
    rem_case!(Up, "-15", "10", "-5");
    rem_case!(HalfEven, "-15", "10", "5");
    rem_case!(HalfAway, "-15", "10", "5");
}

// Binary ties pinning the quotient-parity bookkeeping in all three exponent branches
// of the remainder kernel (exponents equal / lhs above / lhs below).
#[test]
#[rustfmt::skip::macros(fbig)]
fn test_rem_binary_ties() {
    use core::str::FromStr;
    use dashu_float::{round::mode, FBig};

    type HE = FBig<mode::HalfEven, 2>;
    let s = |x: &str| HE::from_str(x).unwrap();

    // equal exponents: 3 % 2 has quotient 1.5 (odd truncated quotient)
    assert_eq!(s("0x3") % s("0x2"), s("-0x1"));
    assert_eq!(fbig!(0x3) % fbig!(0x2), fbig!(0x1)); // Zero: truncated

    // lhs exponent below: 3 % (1·2^1) has quotient 1.5
    assert_eq!(s("0x3") % s("0x1p1"), s("-0x1"));
    assert_eq!(fbig!(0x3) % fbig!(0x1p1), fbig!(0x1)); // Zero: truncated

    // lhs exponent above: 0x18p3 % 0x80 has quotient 1.5 (odd truncated
    // quotient), and % 0x180 has quotient 0.5 (even truncated quotient 0)
    assert_eq!(s("0x18p3") % s("0x80"), s("-0x40"));
    assert_eq!(s("0x18p3") % s("0x180"), s("0xc0"));
    assert_eq!(fbig!(0x18p3) % fbig!(0x80), fbig!(0x40)); // Zero: truncated
}

#[test]
#[should_panic]
fn test_rem_by_inf() {
    let _ = dashu_float::DBig::ONE % dashu_float::DBig::INFINITY;
}

#[test]
#[should_panic]
fn test_rem_by_0() {
    let _ = dashu_float::DBig::ONE % dashu_float::DBig::ZERO;
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_div_rem_euclid_binary() {
    // test cases: n, d, quotient, remainder
    let test_cases = [
        (fbig!(0), fbig!(1), ibig!(0), fbig!(0)),
        (fbig!(1), fbig!(1), ibig!(1), fbig!(0)),
        (fbig!(0x1000), fbig!(0x1000), ibig!(1), fbig!(0)),
        (fbig!(0x1000), fbig!(0x10), ibig!(0x100), fbig!(0)),
        (fbig!(0x1000), fbig!(-0x10), ibig!(-0x100), fbig!(0)),
        (fbig!(-0xffff), fbig!(-0xff), ibig!(0x101), fbig!(0)),
        (fbig!(0x1b), fbig!(0x3), ibig!(9), fbig!(0)),
        (fbig!(0x43), fbig!(0x21), ibig!(2), fbig!(0x1)),
        (fbig!(0x654), fbig!(-0x321), ibig!(-2), fbig!(0x12)),
        (fbig!(-0x98765), fbig!(-0x43210), ibig!(3), fbig!(0x30ecb)),
        (fbig!(0x1), fbig!(0x9), ibig!(0), fbig!(0x1)),
        (fbig!(0x1), fbig!(0x9p-4), ibig!(1), fbig!(0x7p-4)),
        (fbig!(0x1), fbig!(0x9p-8), ibig!(28), fbig!(0x4p-8)),
        (fbig!(0x13), fbig!(-0x9), ibig!(-2), fbig!(0x1)),
        (fbig!(0x169), fbig!(-0x9), ibig!(-40), fbig!(0x1)),
    ];

    for (n, d, q, r) in &test_cases {
        assert_eq!(n.div_euclid(d), *q);
        assert_eq!(n.rem_euclid(d), *r);
        let (quotient, remainder) = n.div_rem_euclid(d);
        assert_eq!(quotient, *q);
        assert_eq!(remainder, *r);
    }
}

#[test]
fn test_div_rem_euclid_decimal() {
    // test cases: n, d, quotient, remainder
    let test_cases = [
        (dbig!(0), dbig!(1), ibig!(0), dbig!(0)),
        (dbig!(1), dbig!(1), ibig!(1), dbig!(0)),
        (dbig!(1000), dbig!(1000), ibig!(1), dbig!(0)),
        (dbig!(1000), dbig!(10), ibig!(100), dbig!(0)),
        (dbig!(1000), dbig!(-10), ibig!(-100), dbig!(0)),
        (dbig!(-9999), dbig!(-99), ibig!(101), dbig!(0)),
        (dbig!(27), dbig!(3), ibig!(9), dbig!(0)),
        (dbig!(43), dbig!(21), ibig!(2), dbig!(1)),
        (dbig!(654), dbig!(-321), ibig!(-2), dbig!(12)),
        (dbig!(-98765), dbig!(-43210), ibig!(3), dbig!(30865)),
        (dbig!(1), dbig!(9), ibig!(0), dbig!(1)),
        (dbig!(1), dbig!(9e-1), ibig!(1), dbig!(1e-1)),
        (dbig!(1), dbig!(9e-2), ibig!(11), dbig!(1e-2)),
        (dbig!(13), dbig!(-9), ibig!(-1), dbig!(4)),
        (dbig!(169), dbig!(-9), ibig!(-18), dbig!(7)),
    ];

    for (n, d, q, r) in &test_cases {
        assert_eq!(n.div_euclid(d), *q);
        assert_eq!(n.rem_euclid(d), *r);
        let (quotient, remainder) = n.div_rem_euclid(d);
        assert_eq!(quotient, *q);
        assert_eq!(remainder, *r);
    }
}

// ---------------------------------------------------------------------------
// Regressions for the division / remainder reports of the Hegel triage
// (see triage/float-arith.md).
// ---------------------------------------------------------------------------

mod triage {
    use super::helpers::quotient_oracle;
    use core::cmp::Ordering;
    use dashu_base::{
        Approximation, DivEuclid, DivRem, DivRemEuclid, RemEuclid, Sign, UnsignedAbs,
    };
    use dashu_float::{
        round::{mode, Round},
        Context, DBig, FBig, Repr, Word,
    };
    use dashu_int::IBig;
    use proptest::prelude::*;

    fn fbig<R: Round, const B: Word>(sig: i64, exp: isize, precision: usize) -> FBig<R, B> {
        FBig::from_repr(Repr::new(sig.into(), exp), Context::new(0))
            .with_precision(precision)
            .value()
    }

    /// `/` with an operand of unlimited precision (or simply one wider than the result
    /// precision) is the correctly rounded quotient at the larger precision. The operator used
    /// to feed the unbounded dividend straight into the bounded kernel: a debug assertion in
    /// debug builds, an unrounded or wrongly rounded quotient in release builds.
    #[test]
    fn test_div_unlimited_dividend() {
        fn check<R: Round, const B: Word>() {
            let dividends = [1i64, 5, 11, 1000003, -1000003, 987654321987654321];
            let divisors = [1i64, 3, -7, 1023];
            for precision in [1usize, 4, 20, 50, 100, 500] {
                for a_sig in dividends {
                    for b_sig in divisors {
                        // unlimited dividend, limited divisor
                        let a = fbig::<R, B>(a_sig, 0, 0);
                        let b = fbig::<R, B>(b_sig, 0, 0).with_precision(precision).value();
                        let expected = quotient_oracle::<R, B>(precision, a.repr(), b.repr());
                        let via_context = Context::<R>::new(precision)
                            .div(a.repr(), b.repr())
                            .unwrap();
                        assert_eq!(via_context.value_ref().repr(), expected.value_ref());
                        for got in [
                            &a / &b,
                            a.clone() / &b,
                            &a / b.clone(),
                            a.clone() / b.clone(),
                        ] {
                            assert_eq!(got.precision(), precision);
                            assert!(got.repr().digits() <= precision);
                            assert_eq!(
                                got.repr(),
                                expected.value_ref(),
                                "{a_sig} / {b_sig} at precision {precision}, base {B}"
                            );
                        }
                    }
                }
            }
        }
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 10>();
        check::<mode::HalfAway, 10>();
    }

    /// A zero remainder takes the sign of the dividend (IEEE 754 §5.3.1), whether the
    /// dividend is itself a zero or an exact multiple of the divisor.
    #[test]
    fn test_rem_zero_takes_dividend_sign() {
        fn check<R: Round, const B: Word>() {
            for precision in [0usize, 20, 50, 100, 500] {
                // (dividend, divisor) pairs that divide exactly, at assorted exponent gaps
                for (a, a_exp, b, b_exp) in [
                    (6, 0, 3, 0),
                    (25, -1, 5, -1),
                    (3, 4, 3, 0),
                    (3, 4, 1, 2),
                    (12, -2, 3, -3),
                ] {
                    for (a_sign, b_sign) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                        let x = fbig::<R, B>(a_sign * a, a_exp, precision);
                        let y = fbig::<R, B>(b_sign * b, b_exp, precision);
                        let r = &x % &y;
                        assert!(r.repr().significand().is_zero());
                        assert_eq!(r.repr().is_neg_zero(), a_sign < 0, "{x:?} % {y:?}");
                        let r = Context::<R>::new(precision)
                            .rem(x.repr(), y.repr())
                            .unwrap();
                        assert!(matches!(r, Approximation::Exact(_)));
                        assert_eq!(r.value().repr().is_neg_zero(), a_sign < 0);
                        // the Euclidean remainder is non-negative: its zero is +0
                        assert!((&x).rem_euclid(&y).repr().is_pos_zero());
                    }
                }
                let three = fbig::<R, B>(3, 0, precision);
                assert!((-fbig::<R, B>(0, 0, precision) % &three)
                    .repr()
                    .is_neg_zero());
                assert!((fbig::<R, B>(0, 0, precision) % &three)
                    .repr()
                    .is_pos_zero());
            }
        }
        check::<mode::Zero, 2>();
        check::<mode::Zero, 10>();
        check::<mode::HalfEven, 2>();
        check::<mode::HalfAway, 10>();
        check::<mode::Up, 2>();
        check::<mode::Down, 10>();
        check::<mode::Away, 2>();

        // the reported inputs
        use core::str::FromStr;
        let r = DBig::from_str("-2.5").unwrap() % DBig::from_str("0.5").unwrap();
        assert!(r.repr().is_neg_zero());
        let r = FBig::<mode::Zero>::from(6) % FBig::<mode::Zero>::from(-3);
        assert!(r.repr().is_pos_zero());
        // the nonzero remainder still keeps the dividend's sign under truncation
        assert_eq!(
            FBig::<mode::Zero>::from(-7) % FBig::<mode::Zero>::from(3),
            FBig::<mode::Zero>::from(-1)
        );
    }

    /// The remainder is computed without materializing the operands aligned across their
    /// exponent gap, in either direction and in every base: `2^(2^40) % 3` used to run out of
    /// memory (base 2 built the shifted dividend), and `rem_euclid` took minutes and
    /// gigabytes for a gap of 10^9.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_rem_huge_exponent_gap() {
        fn check<R: Round, const B: Word>() {
            // B^e mod 7 by plain modular exponentiation
            let pow_mod = |mut e: u64| {
                #[allow(clippy::unnecessary_cast)] // `Word` is not `u64` on every target
                let (mut base, mut acc) = (B as u64 % 7, 1u64);
                while e > 0 {
                    if e & 1 == 1 {
                        acc = acc * base % 7;
                    }
                    base = base * base % 7;
                    e >>= 1;
                }
                acc
            };
            for precision in [0usize, 20, 50, 100, 500] {
                let seven = fbig::<R, B>(7, 0, precision);
                for e in [1u64 << 20, (1 << 40) + 1, 1_000_000_000, 100_000_000_003] {
                    // dividend far above the divisor
                    let x = fbig::<R, B>(1, e as isize, precision);
                    let residue = pow_mod(e) as i64; // in 1..=6
                    assert_eq!((&x).rem_euclid(&seven), fbig::<R, B>(residue, 0, precision));
                    assert_eq!((-&x).rem_euclid(&seven), fbig::<R, B>(7 - residue, 0, precision));
                    let r = &x % &seven;
                    assert!(
                        r == fbig::<R, B>(residue, 0, precision)
                            || r == fbig::<R, B>(residue - 7, 0, precision),
                        "{r:?}"
                    );

                    // divisor far above the dividend: the dividend is its own remainder,
                    // unless the quotient is stepped away from zero
                    let small = fbig::<R, B>(5, 0, precision);
                    let big = fbig::<R, B>(3, e as isize, precision);
                    assert_eq!((&small).rem_euclid(&big), small);
                    assert_eq!((&small).div_euclid(&big), IBig::ZERO);
                    assert_eq!((-&small).div_euclid(&big), IBig::from(-1));
                    assert_eq!((-&small).div_euclid(-&big), IBig::from(1));
                    if precision != 0 {
                        // 3·B^e − 5 rounded to the precision: only the sign of the 5 matters
                        let complement = (-&small).rem_euclid(&big);
                        // (3·B^(p+3) − 1)·B^(e−p−3) lies on the same side of every rounding
                        // boundary as the exact 3·B^e − 5
                        let stand_in = IBig::from(3) * IBig::from(B).pow(precision + 3) - IBig::ONE;
                        let stand_in = Repr::new(stand_in, e as isize - precision as isize - 3);
                        let reference = FBig::<R, B>::from_repr(stand_in, Context::new(0))
                            .with_precision(precision)
                            .value();
                        assert_eq!(complement, reference);
                        assert!(complement <= big);
                        let r = &small % &big;
                        assert!(
                            r == small || (r.sign() == Sign::Negative && -r.clone() <= big),
                            "{r:?}"
                        );
                    }
                }
            }
        }
        check::<mode::Zero, 2>();
        check::<mode::Down, 2>();
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 10>();
        check::<mode::Up, 10>();
        check::<mode::HalfAway, 10>();
        check::<mode::Away, 3>();

        // the reported inputs
        let x = FBig::<mode::Down, 2>::from_parts(IBig::ONE, 1 << 40);
        let y = FBig::<mode::Down, 2>::from_parts(IBig::from(3), 0);
        assert_eq!(x % y, FBig::<mode::Down, 2>::from_parts(IBig::ONE, 0));
        let x = DBig::from_parts(IBig::ONE, 1_000_000_000);
        assert_eq!((&x).rem_euclid(&DBig::from(3u8)), DBig::ONE);
        assert_eq!(&x % &DBig::from(3u8), DBig::ONE);
    }

    /// A zero dividend is its own remainder, whatever the divisor's exponent is. The exponent
    /// of a zero (0, or -1 for `-0`) is not a scale, but it used to be aligned with the
    /// divisor's: `0 % 3·2^(isize::MAX/2)` ran out of memory (F94).
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_rem_zero_dividend_huge_divisor_exponent() {
        fn check<R: Round, const B: Word>() {
            for precision in [0usize, 20, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                let exponents = [
                    1000,
                    1 << 40,
                    isize::MAX / 2,
                    isize::MAX - 1,
                    isize::MIN + 1,
                ];
                for exponent in exponents {
                    for divisor in [3i64, -3, 1] {
                        let big = fbig::<R, B>(divisor, exponent, precision);
                        for zero in [Repr::<B>::zero(), Repr::<B>::neg_zero()] {
                            let r = match ctx.rem(&zero, big.repr()) {
                                Ok(Approximation::Exact(r)) => r,
                                other => panic!("rem({zero:?}, {big:?}) = {other:?}"),
                            };
                            // the zero keeps the dividend's sign
                            assert_eq!(r.repr(), &zero);
                            assert_eq!(r.precision(), precision);

                            let z = FBig::<R, B>::from_repr(zero.clone(), ctx);
                            assert_eq!((&z % &big).repr(), &zero);
                            // a Euclidean remainder is non-negative
                            assert!((&z).rem_euclid(&big).repr().is_pos_zero());
                            assert_eq!((&z).div_euclid(&big), IBig::ZERO);
                            let (q, r) = z.div_rem_euclid(&big);
                            assert!(q == IBig::ZERO && r.repr().is_pos_zero());
                        }
                    }
                }
            }
        }
        check::<mode::Zero, 2>();
        check::<mode::Down, 2>();
        check::<mode::HalfEven, 10>();
        check::<mode::HalfAway, 10>();
        check::<mode::Up, 3>();
    }

    /// `Context::rem` reports a zero divisor as an error, like `Context::div` reports `0 / 0`
    /// (it used to panic inside the integer division, although it returns an `FpResult`).
    /// The operators have no error channel and keep panicking (F47).
    #[test]
    fn test_rem_by_zero_is_an_error() {
        use dashu_float::FpError;
        use std::panic::catch_unwind;

        fn check<R: Round + std::panic::RefUnwindSafe, const B: Word>() {
            for precision in [0usize, 20, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                let dividends = [
                    Repr::<B>::one(),
                    Repr::<B>::zero(),
                    Repr::<B>::neg_zero(),
                    Repr::<B>::new((-7).into(), -3),
                    Repr::<B>::new(5.into(), isize::MAX / 2),
                    Repr::<B>::new(5.into(), isize::MIN + 1),
                ];
                for x in &dividends {
                    for zero in [Repr::<B>::zero(), Repr::<B>::neg_zero()] {
                        assert_eq!(ctx.rem(x, &zero), Err(FpError::Indeterminate));

                        let fx = FBig::<R, B>::from_repr(x.clone(), ctx);
                        let fz = FBig::<R, B>::from_repr(zero, ctx);
                        assert!(catch_unwind(|| &fx % &fz).is_err());
                        assert!(catch_unwind(|| (&fx).rem_euclid(&fz)).is_err());
                        assert!(catch_unwind(|| (&fx).div_euclid(&fz)).is_err());
                    }
                }
                // an infinite operand is still reported first
                let inf = Repr::<B>::infinity();
                assert_eq!(ctx.rem(&inf, &Repr::zero()), Err(FpError::InfiniteInput));
            }
        }
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 10>();
    }

    /// Like every operation, the Euclidean remainder is rounded to the operands' precision.
    /// When the exact remainder does not fit (the divisor's exponent is above the dividend's)
    /// the rounded value can reach `|rhs|` — the same documented limitation as
    /// `f64::rem_euclid`. With enough precision the remainder is exact.
    #[test]
    fn test_rem_euclid_rounds_to_precision() {
        use core::str::FromStr;
        let x = DBig::from_str("-1").unwrap(); // precision 1
        let d_short = DBig::from_parts(IBig::ONE, 2); // 1e2, precision 1
        let d_long = DBig::from_str("100").unwrap(); // 1e2, precision 3

        assert_eq!((&x).rem_euclid(&d_long), DBig::from(99u8));
        assert_eq!((&x).div_rem_euclid(&d_long), (IBig::from(-1), DBig::from(99u8)));
        // 99 rounded half away from zero to one digit
        assert_eq!((&x).rem_euclid(&d_short), DBig::from_parts(IBig::ONE, 2));
        assert_eq!((&x).div_rem_euclid(&d_short), (IBig::from(-1), DBig::from_parts(IBig::ONE, 2)));
        assert_eq!((&x).div_euclid(&d_short), IBig::from(-1));
        // ... and toward zero under that mode
        type Z = FBig<mode::Zero, 10>;
        let (x, d) = (Z::from_parts((-1).into(), 0), Z::from_parts(1.into(), 2));
        assert_eq!(x.rem_euclid(d), Z::from_parts(9.into(), 1));
    }

    /// The remainder of the exact integers `num / den` under the quotient rule of the mode
    /// named by `rule` (0 = Zero, 1 = Away, 2 = Up, 3 = Down, 4 = HalfAway, 5 = HalfEven,
    /// 6 = Euclidean).
    fn int_rem(num: &IBig, den: &IBig, rule: u8) -> IBig {
        let (q, r) = num.div_rem(den);
        if r.is_zero() {
            return r;
        }
        let quotient_positive = num.sign() == den.sign();
        let twice: IBig = &r << 1;
        let half = twice.unsigned_abs().cmp(&den.clone().unsigned_abs());
        let odd = (&q % IBig::from(2)) != IBig::ZERO;
        let step = match rule {
            0 => false,
            1 => true,
            2 => quotient_positive,
            3 => !quotient_positive,
            4 => half != Ordering::Less,
            5 => half == Ordering::Greater || (half == Ordering::Equal && odd),
            _ => num.sign() == Sign::Negative,
        };
        if step {
            // the quotient moves away from zero by one: r − sign(q)·den
            if quotient_positive {
                r - den
            } else {
                r + den
            }
        } else {
            r
        }
    }

    fn check_rem_against_integers<R: Round, const B: Word>(
        rule: u8,
        (a, a_exp): (i64, isize),
        (b, b_exp): (i64, isize),
        precision: usize,
    ) {
        let min_exp = a_exp.min(b_exp);
        let base = IBig::from(B);
        let num = IBig::from(a) * base.pow((a_exp - min_exp) as usize);
        let den = IBig::from(b) * base.pow((b_exp - min_exp) as usize);
        let exact = |r: IBig| -> FBig<R, B> {
            FBig::from_repr(Repr::new(r, min_exp), Context::new(0))
                .with_precision(precision)
                .value()
        };
        let (x, y) = (fbig::<R, B>(a, a_exp, 0), fbig::<R, B>(b, b_exp, 0));
        let (x, y) = (x.with_precision(precision).value(), y.with_precision(precision).value());
        // the operands themselves must be exact at this precision
        if x.repr() != &Repr::new(a.into(), a_exp) || y.repr() != &Repr::new(b.into(), b_exp) {
            return;
        }

        let r = &x % &y;
        let expected = exact(int_rem(&num, &den, rule));
        assert_eq!(r, expected, "{x:?} % {y:?}");
        if expected.repr().significand().is_zero() {
            assert_eq!(r.sign(), x.sign());
        }
        assert_eq!((&x).rem_euclid(&y), exact(int_rem(&num, &den, 6)), "{x:?} rem_euclid {y:?}");
        let (q, r) = (&x).div_rem_euclid(&y);
        assert_eq!(r, (&x).rem_euclid(&y));
        assert_eq!(q, (&x).div_euclid(&y));
        assert_eq!(q, (&num).div_euclid(&den));
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..Default::default() })]

        /// `%`, `rem_euclid`, `div_euclid` and `div_rem_euclid` against exact integer
        /// arithmetic on the aligned operands, across the exponent-gap branches of the
        /// kernel, every quotient rule, and the standard precisions (0 = unlimited).
        #[test]
        fn prop_rem_matches_integer_arithmetic(
            a in -5000i64..5000,
            b in prop_oneof![1i64..5000, -5000i64..-1],
            a_exp in -40isize..40,
            b_exp in -40isize..40,
            precision in prop_oneof![Just(0usize), Just(20), Just(50), Just(100), Just(500)],
        ) {
            check_rem_against_integers::<mode::Zero, 2>(0, (a, a_exp), (b, b_exp), precision);
            check_rem_against_integers::<mode::Away, 2>(1, (a, a_exp), (b, b_exp), precision);
            check_rem_against_integers::<mode::Up, 10>(2, (a, a_exp), (b, b_exp), precision);
            check_rem_against_integers::<mode::Down, 10>(3, (a, a_exp), (b, b_exp), precision);
            check_rem_against_integers::<mode::HalfAway, 10>(4, (a, a_exp), (b, b_exp), precision);
            check_rem_against_integers::<mode::HalfEven, 2>(5, (a, a_exp), (b, b_exp), precision);
            check_rem_against_integers::<mode::HalfEven, 3>(5, (a, a_exp), (b, b_exp), precision);
        }
    }
}

mod triage_range {
    use dashu_base::{Approximation::*, Sign};
    use dashu_float::{
        round::{mode, Round, Rounding},
        Context, FBig, FpError, Repr, Word,
    };

    fn repr<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(sig.into(), exp)
    }

    /// The exponent range of a quotient is judged on the scaled and rounded result, not on
    /// the difference of the operand exponents: `2 / (3·2^(isize::MIN+1))` is about
    /// `0.67·2^isize::MAX`, i.e. `1·2^(isize::MAX−1)` at one bit, but used to be reported as an
    /// overflow because `1 − (isize::MIN + 1)` does not fit an `isize`.
    #[test]
    fn test_div_exponent_range_is_checked_after_rounding() {
        fn check<R: Round, const B: Word>() {
            let (min, max) = (isize::MIN, isize::MAX);
            for precision in [1usize, 20, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                for (a_sig, b_sig) in [(2i128, 3i128), (1, 3), (-7, 3), (1, 1), (12, -4), (1, 1023)]
                {
                    // the quotient at ordinary exponents is the reference
                    let reference = ctx.div(&repr::<B>(a_sig, 0), &repr::<B>(b_sig, 0)).unwrap();
                    let (ref_exact, reference) = (matches!(reference, Exact(_)), reference.value());
                    let ref_exp = reference.repr().exponent() as i128;
                    let sign = reference.sign();
                    // (the significands are normalized: a trailing zero digit moves into the
                    // exponent, which the reference shares)
                    for (a_exp, b_exp) in [
                        (1isize, min + 1),
                        (0, min + 1),
                        (0, min + 2),
                        (max - 1, -1),
                        (max - 1, -(precision as isize) - 20),
                        (min + 1, 1),
                        (min + 1, 0),
                        (min + precision as isize + 20, 2),
                        (min / 2, max / 2),
                        (max / 2, min / 2),
                    ] {
                        let (a, b) = (repr::<B>(a_sig, a_exp), repr::<B>(b_sig, b_exp));
                        let a_fold = a.exponent() as i128 - a_exp as i128;
                        let b_fold = b.exponent() as i128 - b_exp as i128;
                        let ref_fold = repr::<B>(a_sig, 0).exponent() as i128
                            - repr::<B>(b_sig, 0).exponent() as i128;
                        let exponent = ref_exp - ref_fold + (a_exp as i128 + a_fold)
                            - (b_exp as i128 + b_fold);
                        let got = ctx.div(&a, &b);
                        if exponent >= max as i128 {
                            assert_eq!(got, Err(FpError::Overflow(sign)), "{a:?} / {b:?}");
                        } else if exponent <= min as i128 {
                            assert_eq!(got, Err(FpError::Underflow(sign)), "{a:?} / {b:?}");
                        } else {
                            let got = got.unwrap();
                            assert_eq!(matches!(got, Exact(_)), ref_exact);
                            let got = got.value();
                            assert_eq!(got.repr().significand(), reference.repr().significand());
                            assert_eq!(got.repr().exponent() as i128, exponent, "{a:?} / {b:?}");
                            // the operator agrees (for operands that fit the precision)
                            if a.digits() <= precision && b.digits() <= precision {
                                let x = FBig::<R, B>::from_repr(a.clone(), ctx);
                                let y = FBig::<R, B>::from_repr(b.clone(), ctx);
                                assert_eq!(x / y, got);
                            }
                        }
                    }
                }
            }
        }
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 10>();
        check::<mode::HalfAway, 10>();

        // the reported case
        let ctx = Context::<mode::HalfEven>::new(1);
        let got = ctx.div(&repr::<2>(1, 1), &repr::<2>(3, isize::MIN + 1));
        let expected = FBig::from_repr(repr::<2>(1, isize::MAX - 1), ctx);
        assert_eq!(got, Ok(Inexact(expected.clone(), Rounding::NoOp)));
        let x = FBig::<mode::HalfEven, 2>::from_repr(repr(1, 1), ctx);
        let y = FBig::<mode::HalfEven, 2>::from_repr(repr(3, isize::MIN + 1), ctx);
        assert_eq!(&x / &y, expected);
        assert_eq!((&x / &y).sign(), Sign::Positive);
    }
}
