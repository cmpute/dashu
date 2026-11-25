//! Complex addition and subtraction at the edge of the exponent range.

use dashu_base::Approximation::Inexact;
use dashu_cmplx::{CBig, Context};
use dashu_float::round::mode::HalfAway;
use dashu_float::round::Rounding;
use dashu_float::FBig;
use dashu_float::FpError;
use dashu_int::IBig;

type F = FBig<HalfAway, 2>;
type C = CBig<HalfAway, 2>;

fn big() -> F {
    F::from_parts(IBig::ONE, isize::MAX - 1)
}

#[test]
fn part_out_of_range_keeps_the_other_part() {
    // A sum is componentwise: when one part leaves the exponent range it saturates to a signed
    // infinity (flagged inexact) and the other part is still the correctly rounded sum.
    let one = F::ONE;
    let two = F::from(2);
    for precision in [10, 20, 50, 100, 500] {
        let ctx = Context::<HalfAway>::new(precision);
        let z = C::from_parts(big(), one.clone());
        let nz = C::from_parts(-big(), one.clone());
        let swapped = C::from_parts(one.clone(), big());

        // the same value as `mul_real(z, 2)`
        let sum = ctx.add(&z, &z).expect("only one part overflows");
        assert!(matches!(sum, Inexact(_, (Rounding::NoOp, Rounding::NoOp))));
        let doubled = ctx.mul_real(&z, &two).expect("only one part overflows");
        assert_eq!(sum.clone().value(), doubled.value());
        let (re, im) = sum.value().into_parts();
        assert_eq!(re, F::INFINITY);
        assert_eq!(im, two);

        // the sign of the overflowing part is kept
        let (re, im) = ctx.add(&nz, &nz).unwrap().value().into_parts();
        assert_eq!(re, F::NEG_INFINITY);
        assert_eq!(im, two);
        let (re, im) = ctx.sub(&nz, &z).unwrap().value().into_parts();
        assert_eq!(re, F::NEG_INFINITY);
        assert_eq!(im, F::ZERO);
        let (re, im) = ctx.sub(&z, &nz).unwrap().value().into_parts();
        assert_eq!(re, F::INFINITY);
        assert_eq!(im, F::ZERO);

        // an overflowing imaginary part
        let (re, im) = ctx.add(&swapped, &swapped).unwrap().value().into_parts();
        assert_eq!(re, two);
        assert_eq!(im, F::INFINITY);
        let (re, im) = ctx
            .sub(&swapped.conj(), &swapped)
            .unwrap()
            .value()
            .into_parts();
        assert_eq!(re, F::ZERO);
        assert_eq!(im, F::NEG_INFINITY);

        // both parts out of range: the error is returned for the caller's policy
        let both = C::from_parts(big(), big());
        assert_eq!(
            ctx.add(&both, &both).unwrap_err(),
            FpError::Overflow(dashu_base::Sign::Positive)
        );
    }

    // the operators follow the same policy
    let z = C::from_parts(big(), F::ONE);
    let nz = C::from_parts(-big(), F::ONE);
    let (re, im) = (&z + &z).into_parts();
    assert_eq!((re, im), (F::INFINITY, F::from(2)));
    let (re, im) = (&nz + &nz).into_parts();
    assert_eq!((re, im), (F::NEG_INFINITY, F::from(2)));
    let (re, im) = (&nz - &z).into_parts();
    assert_eq!((re, im), (F::NEG_INFINITY, F::ZERO));
    let mut acc = z.clone();
    acc += &z;
    assert_eq!(acc.into_parts(), (F::INFINITY, F::from(2)));
    // both parts overflowing collapse to the complex infinity
    let both = C::from_parts(big(), big());
    let sum = &both + &both;
    assert!(sum.re().is_infinite() && sum.im().is_pos_zero());
}
