//! Tests for the sign queries (`sign`, `signum`).

use dashu_base::Sign;
use dashu_float::{round::mode, Context, DBig, FBig, Repr};

/// `signum` returns the documented constants `ONE` / `ZERO` / `NEG_ONE`, which are exact
/// (unlimited precision). It used to return a precision-1 value, so combining it with an
/// unlimited-precision operand rounded the result to a single digit (`11 * signum(11)` gave
/// `10`).
#[test]
fn test_signum_is_exact_constant() {
    fn check<R: dashu_float::round::Round, const B: dashu_float::Word>() {
        for precision in [0, 20, 50, 100, 500] {
            for (significand, expected) in [(11, 1), (123, 1), (-11, -1), (0, 0)] {
                let exact = Repr::new(significand.into(), 0);
                let x = FBig::<R, B>::from_repr(exact, Context::new(0))
                    .with_precision(precision)
                    .value();
                let s = x.signum();
                assert_eq!(s.precision(), 0);
                assert_eq!(s.repr(), &Repr::new(expected.into(), 0));
                // multiplying by the signum is exact at the precision of `x`
                let abs = &x * &s;
                assert_eq!(abs.precision(), x.precision());
                assert_eq!(abs.repr(), &Repr::new((significand * expected).into(), 0));
                // and so is adding it
                let sum = &x + &s;
                assert_eq!(sum.precision(), x.precision());
                assert_eq!(sum.repr(), &Repr::new((significand + expected).into(), 0));
            }
        }
    }
    check::<mode::Zero, 2>();
    check::<mode::HalfAway, 10>();
    check::<mode::HalfEven, 2>();
    check::<mode::Down, 10>();

    assert_eq!(DBig::ONE.signum().precision(), DBig::ONE.precision());
    assert_eq!(DBig::INFINITY.signum(), DBig::ONE);
    assert_eq!(DBig::NEG_INFINITY.signum(), DBig::NEG_ONE);
    // signum(±0) is +0
    assert_eq!((-DBig::ZERO).signum().sign(), Sign::Positive);
}
