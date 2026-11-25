//! Tests for `ulp` / `ulp_lb`.

use core::str::FromStr;
use std::{cell::RefCell, rc::Rc};

use dashu_float::{
    round::{mode, ErrorBounds, Round},
    CachedFBig, ConstCache, Context, DBig, FBig, Repr, Word,
};

fn fbig<R: Round, const B: Word>(repr: Repr<B>, precision: usize) -> FBig<R, B> {
    FBig::from_repr(repr, Context::new(precision))
}

/// `ulp(x)` is the weight of the last of `precision` digits lined up with the leading digit
/// of `x`, i.e. `B^(e + digits − precision)`.
#[test]
fn test_ulp_value() {
    assert_eq!(DBig::from_str("1.23").unwrap().ulp(), DBig::from_str("0.01").unwrap());
    assert_eq!(DBig::from_str("01.23").unwrap().ulp(), DBig::from_str("0.001").unwrap());
    for precision in [20usize, 50, 100, 500] {
        for (sig, exp) in [(1i32, 0isize), (-1, 0), (7, 3), (-7, -3), (1023, -40)] {
            let x = fbig::<mode::Zero, 2>(Repr::new(sig.into(), exp), precision);
            let digits = x.repr().digits() as isize;
            let expected = Repr::<2>::new(1.into(), exp + digits - precision as isize);
            assert_eq!(x.ulp().repr(), &expected);
            assert_eq!(x.ulp().precision(), precision);
            assert!(x.ulp_lb() < x.ulp());
            // the sign of x does not matter
            assert_eq!((-x.clone()).ulp(), x.ulp());
        }
    }
}

/// What `x + ulp` is, as documented: the next float above a positive `x` (and `x − ulp` the
/// next one below a negative `x`). Toward zero the neighbour of a power of the base is only
/// `ulp / B` away — `-0.099` lies between `-0.1` and `-0.1 + ulp` at two digits.
#[test]
fn test_ulp_neighbours() {
    fn check<const B: Word>() {
        type R = mode::Zero;
        for precision in [8usize, 20, 50, 100, 500] {
            let ctx = Context::<R>::new(precision);
            let exact = Context::<R>::new(0);
            for (sig, exp) in [(1i32, -1isize), (2, -1), (1, 0), (37, -5), (1, 12), (5, 3)] {
                let x = fbig::<R, B>(Repr::new(sig.into(), exp), precision);
                let ulp = x.ulp();
                let is_power = x.repr().significand().is_one(); // (2·B^-1 is 1 in base 2)

                // away from zero: no float lies strictly between x and x ± ulp, because any
                // such value needs a digit below the ulp position
                let beyond = exact.add(x.repr(), ulp.repr()).unwrap().value();
                assert!(beyond.repr().digits() <= precision);
                let between = exact
                    .add(x.repr(), &Repr::new(1.into(), ulp.repr().exponent() - 1))
                    .unwrap()
                    .value();
                assert!(between.repr().digits() > precision, "{between:?}");
                assert_eq!(&x + &ulp, FBig::from_repr(beyond.repr().clone(), ctx));
                assert_eq!(-(&(-x.clone()) - &ulp), FBig::from_repr(beyond.repr().clone(), ctx));

                // toward zero: one ulp as well, except below a power of the base
                let finer = Repr::<B>::new(1.into(), ulp.repr().exponent() - 1);
                let inside = exact.sub(x.repr(), &finer).unwrap().value();
                assert_eq!(inside.repr().digits() <= precision, is_power, "{inside:?}");
            }
        }
    }
    check::<2>();
    check::<10>();

    // the reported case
    let x = DBig::from_str("-0.1").unwrap().with_precision(2).value();
    assert_eq!(x.ulp(), DBig::from_str("0.01").unwrap());
    let between = DBig::from_str("-0.099").unwrap();
    assert!(between > x && between < &x + &x.ulp() && between.repr().digits() <= 2);
}

/// The ulp of a zero does not depend on its sign: the `-0` encoding (a sentinel exponent)
/// used to make `ulp(-0)` and `ulp_lb(-0)` one digit smaller than for `+0`, and with them
/// the error bounds of `-0`.
#[test]
fn test_ulp_of_signed_zeros() {
    fn check<R: ErrorBounds, const B: Word>() {
        for precision in [1usize, 20, 50, 100, 500] {
            let pz = fbig::<R, B>(Repr::zero(), precision);
            let nz = fbig::<R, B>(Repr::neg_zero(), precision);
            let expected = Repr::<B>::new(1.into(), -(precision as isize));
            assert_eq!(pz.ulp().repr(), &expected);
            assert_eq!(nz.ulp().repr(), &expected);
            assert_eq!(pz.ulp_lb(), nz.ulp_lb());
            assert!(nz.ulp_lb() < nz.ulp());
            // the error bounds of -0 are as wide as the ulp it shares with +0 (they are
            // one-sided for a signed zero, so only the nonzero sides are compared)
            let ulp = pz.ulp();
            for zero in [&pz, &nz] {
                let (left, right, _, _) = R::error_bounds(zero);
                for bound in [left, right] {
                    assert!(bound.repr().significand().is_zero() || bound <= ulp, "{bound:?}");
                    assert!(bound.repr().significand().is_zero() || &bound + &bound >= ulp);
                }
            }
        }
    }
    check::<mode::HalfAway, 10>();
    check::<mode::HalfEven, 2>();
    check::<mode::Zero, 2>();
    check::<mode::Down, 10>();
    check::<mode::Up, 2>();
}

/// `CachedFBig::ulp` is `FBig::ulp`: it used to recompute the exponent with unchecked
/// arithmetic and without the infinity shortcut, so it panicked on `±inf` and — in builds
/// with overflow checks — on exponents near the ends of the `isize` range (and returned a
/// wrapped exponent otherwise).
#[test]
fn test_cached_ulp_matches_fbig() {
    fn check<R: Round, const B: Word>() {
        for precision in [1usize, 2, 20, 50, 100, 500] {
            for repr in [
                Repr::<B>::new(1.into(), 0),
                Repr::new((-3).into(), -7),
                Repr::zero(),
                Repr::neg_zero(),
                Repr::infinity(),
                Repr::neg_infinity(),
                Repr::new(1.into(), isize::MAX - 1),
                Repr::new(1.into(), isize::MAX),
                Repr::new(1.into(), isize::MIN + 1),
                Repr::new(1.into(), isize::MIN),
                Repr::new((-1).into(), isize::MIN),
            ] {
                let plain = fbig::<R, B>(repr.clone(), precision);
                let cache = Rc::new(RefCell::new(ConstCache::new()));
                let cached = CachedFBig::<R, B>::from_repr(repr, Context::new(precision), cache);
                assert_eq!(cached.ulp().into_fbig(), plain.ulp());
                assert_eq!(cached.ulp().as_fbig().sign(), plain.ulp().sign());
                assert_eq!(cached.ulp_lb().into_fbig(), plain.ulp_lb());
            }
        }
    }
    check::<mode::Zero, 2>();
    check::<mode::HalfAway, 10>();
}

#[test]
#[should_panic(expected = "precision cannot be 0")]
fn test_cached_ulp_unlimited_precision() {
    let cache = Rc::new(RefCell::new(ConstCache::new()));
    let x = CachedFBig::<mode::Zero, 2>::from_repr(Repr::one(), Context::new(0), cache);
    let _ = x.ulp();
}

/// Next to the ends of the exponent range the ulp is still `B^(e + len − precision)` whenever
/// that exponent exists: the sum must not be saturated half way through.
#[test]
fn test_ulp_near_exponent_limits() {
    type F = FBig<mode::HalfEven, 10>;
    let sig = dashu_int::IBig::from(12345678901234567891u64); // 20 decimal places
    for precision in [20usize, 25, 50, 100, 500] {
        for e in [
            isize::MAX - 30,
            isize::MAX - 8,
            isize::MAX - 1,
            isize::MIN + 1000,
        ] {
            let x: F = fbig(Repr::new(sig.clone(), e), precision);
            let expected = e as i128 + 20 - precision as i128;
            assert_eq!(x.ulp().repr(), &Repr::new(1.into(), expected as isize), "{e} {precision}");
            assert!(x.ulp_lb() < x.ulp(), "{e} {precision}");
        }
    }
    // binary, with a one-word and a multi-word significand
    for precision in [20usize, 50, 100, 500] {
        for bits in [1usize, 19, 64, 100] {
            if bits > precision {
                continue;
            }
            let sig = (dashu_int::IBig::ONE << bits) - dashu_int::IBig::ONE;
            let e = isize::MAX - 3;
            let x: FBig<mode::Zero, 2> = fbig(Repr::new(sig, e), precision);
            let expected = e as i128 + bits as i128 - precision as i128;
            assert_eq!(x.ulp().repr(), &Repr::new(1.into(), expected as isize));
            assert!(x.ulp_lb() < x.ulp());
        }
    }
    // an ulp below the range still saturates at the smallest exponent instead of panicking
    let x: F = fbig(Repr::new(sig, isize::MIN + 2), 25);
    assert_eq!(x.ulp().repr().exponent(), isize::MIN);
    assert_eq!(x.ulp_lb().repr().exponent(), isize::MIN);
}
