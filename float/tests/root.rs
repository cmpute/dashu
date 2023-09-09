use dashu_base::Approximation::*;
use dashu_float::{ops::SquareRoot, round::Rounding::*, DBig};

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
            fbig!(0x3).with_precision(200).unwrap(),
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
            dbig!(3).with_precision(60).unwrap(),
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
#[should_panic]
fn test_sqrt_unlimited_precision() {
    let _ = dbig!(2).with_precision(0).unwrap().sqrt();
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
