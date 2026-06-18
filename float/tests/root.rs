use dashu_base::Approximation::*;
use dashu_float::{
    ops::{CubicRoot, SquareRoot},
    round::Rounding::*,
    DBig, FBig,
};
use dashu_int::IBig;

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_sqrt_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(0)),
        (fbig!(1), fbig!(1)),
        (fbig!(0x9), fbig!(0x3)),
        (fbig!(0x100), fbig!(0x10)),
    ];
    for (x, sqrt) in &exact_cases {
        assert_eq!(x.sqrt(), *sqrt);
        if let Exact(v) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *sqrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (fbig!(0x3), fbig!(0xdp-3)),
        (fbig!(0x0003), fbig!(0xddb3p-15)),
        (fbig!(0x0000000000000003), fbig!(0xddb3d742c265539dp-63)),
        (
            fbig!(0x3).with_precision(200).value(),
            fbig!(0xddb3d742c265539d92ba16b83c5c1dc492ec1a6629ed23cc63p-199),
        ),
        (fbig!(0x3000), fbig!(0xddb3p-9)),
        (fbig!(0x3000000000000000), fbig!(0xddb3d742c265539dp-33)),
        (fbig!(0xf), fbig!(0xfp-2)),
        (fbig!(0xffff), fbig!(0xffffp-8)),
    ];

    for (x, root) in &inexact_cases {
        assert_eq!(x.sqrt(), *root);
        if let Inexact(v, e) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *root);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_sqrt_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(0)),
        (dbig!(1), dbig!(1)),
        (dbig!(9), dbig!(3)),
        (dbig!(6561), dbig!(81)),
    ];
    for (x, sqrt) in &exact_cases {
        assert_eq!(x.sqrt(), *sqrt);
        if let Exact(v) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *sqrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(3), dbig!(2), AddOne),
        (dbig!(0003), dbig!(1732e-3), NoOp),
        (dbig!(0000000000000003), dbig!(1732050807568877e-15), NoOp),
        (
            dbig!(3).with_precision(60).value(),
            dbig!(173205080756887729352744634150587236694280525381038062805581e-59),
            AddOne,
        ),
        (dbig!(3000), dbig!(5477e-2), NoOp),
        (dbig!(3000000000000000), dbig!(5477225575051661e-8), NoOp),
        (dbig!(9999), dbig!(9999e-2), NoOp),
        (dbig!(9999e-4), dbig!(9999e-4), NoOp),
    ];
    for (x, sqrt, rnd) in &inexact_cases {
        assert_eq!(x.sqrt(), *sqrt);
        if let Inexact(v, e) = x.context().sqrt(x.repr()) {
            assert_eq!(v, *sqrt);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_cbrt_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(0)),
        (fbig!(1), fbig!(1)),
        (fbig!(0x8), fbig!(0x2)),
        (fbig!(0x1000), fbig!(0x10)), // 4096 = 16^3
        (fbig!(0x8000), fbig!(0x20)), // 32768 = 32^3
    ];
    for (x, cbrt) in &exact_cases {
        assert_eq!(x.cbrt(), *cbrt);
        if let Exact(v) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *cbrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (fbig!(0x3), fbig!(0xbp-3)),
        (fbig!(0x0003), fbig!(0xb89bp-15)),
        (fbig!(0x0000000000000003), fbig!(0xb89ba24891f7b2e6p-63)),
        (
            fbig!(0x3).with_precision(200).value(),
            fbig!(0xb89ba24891f7b2e6ef3f8b62b71933e050c4a6157ab766ccfap-199),
        ),
        (fbig!(0x300000), fbig!(0x9285ffp-16)),
        (fbig!(0x300000000000000000000000), fbig!(0x9285ff0d8417a7cdc6a6742ep-64)),
    ];
    for (x, root) in &inexact_cases {
        assert_eq!(x.cbrt(), *root);
        if let Inexact(v, e) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *root);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_cbrt_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(0)),
        (dbig!(1), dbig!(1)),
        (dbig!(8), dbig!(2)),
        (dbig!(27), dbig!(3)),
        (dbig!(1000), dbig!(10)),
        (dbig!(8000), dbig!(20)),
    ];
    for (x, cbrt) in &exact_cases {
        assert_eq!(x.cbrt(), *cbrt);
        if let Exact(v) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *cbrt);
        } else {
            panic!("the result should be exact!")
        }
    }

    let inexact_cases = [
        (dbig!(3), dbig!(1), NoOp),
        (dbig!(0003), dbig!(1442e-3), NoOp),
        (dbig!(0000000000000003), dbig!(1442249570307408e-15), NoOp),
        (
            dbig!(3).with_precision(60).value(),
            dbig!(144224957030740838232163831078010958839186925349935057754642e-59),
            AddOne,
        ),
        (dbig!(300000), dbig!(669433e-4), AddOne),
        (dbig!(300000000000000000000000), dbig!(669432950082169521882659e-16), NoOp),
        (dbig!(999999), dbig!(100), AddOne),
        (dbig!(999999e-6), dbig!(1.00000), AddOne),
    ];
    for (x, cbrt, rnd) in &inexact_cases {
        assert_eq!(x.cbrt(), *cbrt);
        if let Inexact(v, e) = x.context().cbrt(x.repr()) {
            assert_eq!(v, *cbrt);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_nth_root_exact_decimal() {
    // (input, n, expected)
    let cases: &[(DBig, usize, DBig)] = &[
        (dbig!(1), 7, dbig!(1)),
        (dbig!(4), 2, dbig!(2)),
        (dbig!(27), 3, dbig!(3)),
        (dbig!(16), 4, dbig!(2)),
        (dbig!(1024), 5, dbig!(4)),  // 4^5 = 1024
        (dbig!(2187), 7, dbig!(3)),  // 3^7 = 2187
        (dbig!(4096), 12, dbig!(2)), // 2^12 = 4096
    ];
    for (x, n, expected) in cases {
        assert_eq!(x.nth_root(*n), *expected, "nth_root({n}) of {x:?}");
        match x.context().nth_root(*n, x.repr()) {
            Exact(v) => assert_eq!(v, *expected),
            _ => panic!("the result should be exact!"),
        }
        // the identity case returns the value unchanged
        assert_eq!(x.nth_root(1), *x);
    }

    let inexact_cases = [
        // 20 digits
        (dbig!(00000000000000000003), 4, dbig!(1.3160740129524924608), NoOp),
        (dbig!(00000000000000000003), 5, dbig!(1.2457309396155173260), AddOne),
        (dbig!(00000000000000000003), 6, dbig!(1.2009369551760027267), AddOne),
        (dbig!(00000000000000000003), 7, dbig!(1.1699308127586868865), AddOne),
        (dbig!(00000000000000000003), 8, dbig!(1.1472026904398770895), AddOne),
        (dbig!(00000000000000000003), 9, dbig!(1.1298309639097530326), NoOp),
        (dbig!(00000000000000000003), 10, dbig!(1.1161231740339044344), NoOp),
        (dbig!(00000000000000000003), 11, dbig!(1.1050315033964666965), NoOp),
        (dbig!(00000000000000000003), 10000, dbig!(1.0001098672638326159), NoOp),
    ];
    for (x, n, root, rnd) in &inexact_cases {
        assert_eq!(x.nth_root(*n), *root);
        if let Inexact(v, e) = x.context().nth_root(*n, x.repr()) {
            assert_eq!(v, *root);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_cbrt_and_nth_root_negative() {
    // odd roots of negative numbers are real and negative
    assert_eq!(fbig!(-0x8).cbrt(), fbig!(-0x2));
    assert_eq!(dbig!(-27).cbrt(), dbig!(-3));
    assert_eq!(dbig!(-1).cbrt(), dbig!(-1));
    assert_eq!(dbig!(-8).nth_root(3), dbig!(-2));
    assert_eq!(dbig!(-32).nth_root(5), dbig!(-2)); // (-2)^5 = -32
    assert_eq!(dbig!(-1).nth_root(9), dbig!(-1));

    // cbrt(-x) == -cbrt(x)
    let x = dbig!(2);
    assert_eq!(dbig!(-2).cbrt(), -x.cbrt());
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_nth_root_equals_sqrt() {
    // nth_root(2, x) must match sqrt(x) exactly (value and rounding flag),
    // which exercises the full general nth-root path against the trusted sqrt.
    let binary_cases = [
        fbig!(0x3),
        fbig!(0x3000),
        fbig!(0xf),
        fbig!(0xffff),
        fbig!(0xfeed),
        fbig!(0x3).with_precision(200).value(),
        fbig!(0x1234).with_precision(80).value(),
    ];
    for x in &binary_cases {
        let by_sqrt = x.context().sqrt(x.repr());
        let by_nth = x.context().nth_root(2, x.repr());
        assert_eq!(by_sqrt, by_nth, "nth_root(2) != sqrt for {x:?}");
    }

    let decimal_cases = [
        dbig!(3),
        dbig!(2),
        dbig!(3000),
        dbig!(9999),
        dbig!(3).with_precision(60).value(),
    ];
    for x in &decimal_cases {
        let by_sqrt = x.context().sqrt(x.repr());
        let by_nth = x.context().nth_root(2, x.repr());
        assert_eq!(by_sqrt, by_nth, "nth_root(2) != sqrt for {x:?}");
    }
}

/// For toward-zero rounding (the binary FBig default), the correctly rounded
/// nth root `r` satisfies `r^n <= x < (r + ulp(r))^n`. The powers are evaluated
/// at unlimited precision so the bracketing comparison is exact.
#[rustfmt::skip::macros(fbig)]
fn assert_root_bracketed_bin(x: FBig, n: usize) {
    let r = x.nth_root(n);
    let ulp = r.ulp();
    let r = r.with_precision(0).value();
    let ulp = ulp.with_precision(0).value();
    let x = x.with_precision(0).value();

    let lower = r.clone().powi(IBig::from(n));
    let upper = (r.clone() + ulp).powi(IBig::from(n));
    assert!(
        lower <= x && x < upper,
        "nth_root({n}) bracketing failed for input: expected {lower:?} <= x < {upper:?}"
    );
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_nth_root_rounding_binary() {
    let inputs = [
        fbig!(0x2),
        fbig!(0x3),
        fbig!(0x5),
        fbig!(0x9),
        fbig!(0x100),
        fbig!(0x1ff),
        fbig!(0x3000),
        fbig!(0xfeed),
        fbig!(0xdeadbeef),
        fbig!(0x3).with_precision(100).value(),
        fbig!(0x2).with_precision(64).value(),
        fbig!(0xfeed).with_precision(37).value(),
    ];
    for x in &inputs {
        for n in [2usize, 3, 5, 7, 10, 13] {
            assert_root_bracketed_bin(x.clone(), n);
        }
    }
}

/// For nearest rounding (the decimal DBig default), the correctly rounded nth
/// root `r` lies within one ulp of the true root, so
/// `(r - ulp)^n < x < (r + ulp)^n`.
fn assert_root_within_ulp_dec(x: DBig, n: usize) {
    let r = x.nth_root(n);
    let ulp = r.ulp();
    let r = r.with_precision(0).value();
    let ulp = ulp.with_precision(0).value();
    let x = x.with_precision(0).value();

    let lower = (r.clone() - ulp.clone()).powi(IBig::from(n));
    let upper = (r + ulp).powi(IBig::from(n));
    assert!(
        lower < x && x < upper,
        "nth_root({n}) bracketing failed for input: expected {lower:?} < x < {upper:?}"
    );
}

#[test]
fn test_nth_root_rounding_decimal() {
    let inputs = [
        dbig!(2),
        dbig!(3),
        dbig!(10),
        dbig!(2000),
        dbig!(99999),
        dbig!(2).with_precision(50).value(),
        dbig!(123).with_precision(40).value(),
    ];
    for x in &inputs {
        for n in [2usize, 3, 5, 7] {
            assert_root_within_ulp_dec(x.clone(), n);
        }
    }
}

#[test]
#[should_panic]
fn test_sqrt_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).value().sqrt();
}

#[test]
#[should_panic]
fn test_sqrt_inf() {
    let _ = DBig::INFINITY.sqrt();
}

#[test]
#[should_panic]
fn test_sqrt_negative() {
    let _ = DBig::NEG_ONE.sqrt();
}

#[test]
#[should_panic]
fn test_nth_root_zero_degree() {
    let _ = dbig!(8).nth_root(0);
}

#[test]
#[should_panic]
fn test_nth_root_even_of_negative() {
    let _ = dbig!(-4).nth_root(2);
}

#[test]
#[should_panic]
fn test_cbrt_inf() {
    let _ = DBig::INFINITY.cbrt();
}

#[test]
#[should_panic]
fn test_cbrt_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).value().cbrt();
}
