//! Construction and conversion: `from_parts` with mixed precisions, and the integer embeddings.

use dashu_cmplx::CBig;
use dashu_float::round::mode::HalfAway;
use dashu_float::{Context as FloatContext, FBig, Repr};
use dashu_int::{IBig, UBig};

type F = FBig<HalfAway, 10>;
type C = CBig<HalfAway, 10>;

fn f(sig: i64, exp: isize, p: usize) -> F {
    F::from_repr(Repr::new(sig.into(), exp), FloatContext::new(p))
}

#[test]
fn from_parts_fits_an_unlimited_part_to_the_shared_precision() {
    // A limited part paired with an unlimited one: the shared precision is the limited one, and
    // the unlimited part is rounded to it when it holds more digits (as `FBig` arithmetic mixing
    // the two precisions does) — so `into_parts` never yields an over-wide `FBig`.
    let z = C::from_parts(f(0, 0, 1), f(101, 0, 0));
    assert_eq!(z.precision(), 1);
    assert_eq!(z.im(), (f(0, 0, 1) + f(101, 0, 0)).repr());
    assert_eq!(z.im(), &Repr::<10>::new(1.into(), 2));
    let (re, im) = z.into_parts();
    assert_eq!((re.precision(), im.precision()), (1, 1));

    for p in [20usize, 50, 100, 500] {
        // p + 3 nines at unlimited precision next to a p-digit part (either position)
        let nines = F::from_repr(Repr::new(IBig::from(10).pow(p + 3) - 1, 0), FloatContext::new(0));
        for z in [
            C::from_parts(f(7, 0, p), nines.clone()),
            C::from_parts(nines.clone(), f(7, 0, p)),
        ] {
            assert_eq!(z.precision(), p);
            assert!(z.re().digits() <= p && z.im().digits() <= p);
            let (re, im) = z.into_parts(); // must not panic
            let rounded = if re.repr().significand() == &IBig::from(7) {
                im
            } else {
                re
            };
            assert_eq!(rounded.repr(), &Repr::<10>::new(1.into(), p as isize + 3));
        }
        // an unlimited part that fits is kept exactly
        let z = C::from_parts(f(7, 0, 0), f(123, -1, p));
        assert_eq!(z.re(), &Repr::<10>::new(7.into(), 0));
        // two limited parts: the larger precision wins and nothing is rounded
        let z = C::from_parts(f(123456789, 0, 9), f(1, 0, p));
        assert_eq!(z.precision(), p.max(9));
        assert_eq!(z.re(), &Repr::<10>::new(123456789.into(), 0));
    }
    // both unlimited: nothing is rounded
    let z = C::from_parts(f(101, 0, 0), f(0, 0, 0));
    assert_eq!((z.precision(), z.re().digits()), (0, 3));
}

#[test]
fn integer_embeddings_follow_fbig() {
    // `CBig::from(integer)` is `CBig::from(FBig::from(integer))`: exact, with the precision set
    // to the integer's digit count (like `FBig`) — *not* unlimited.
    let z = C::from(IBig::from(-123));
    assert_eq!(z.precision(), F::from(IBig::from(-123)).precision());
    assert_eq!(z.precision(), 3);
    assert_eq!(z.re(), &Repr::<10>::new((-123).into(), 0));
    assert!(z.im().is_pos_zero());
    assert_eq!(C::from(UBig::from(6u8)).precision(), 1);
    assert_eq!(C::from(6u8).precision(), 1);
    // so arithmetic on such values rounds to that precision, exactly as for `FBig` …
    let prod = C::from(IBig::from(2)) * C::from(IBig::from(6));
    assert_eq!(prod.re(), (F::from(2) * F::from(6)).repr());
    // … while values built at unlimited precision multiply exactly
    let exact = |v: i64| C::from(f(v, 0, 0));
    assert_eq!(IBig::try_from(exact(2) * exact(6)), Ok(IBig::from(12)));
}
