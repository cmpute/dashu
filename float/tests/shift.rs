use dashu_float::DBig;

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_shift() {
    assert_eq!(fbig!(0x0) << 1, fbig!(0x0));
    assert_eq!(fbig!(0x0) >> 1, fbig!(0x0));
    assert_eq!(fbig!(0x1) << 1, fbig!(0x1p1));
    assert_eq!(fbig!(0x1) >> 1, fbig!(0x1p-1));
    assert_eq!(fbig!(-0x1) << 1, fbig!(-0x1p1));
    assert_eq!(fbig!(-0x1) >> 1, fbig!(-0x1p-1));

    assert_eq!(dbig!(0) << 1, dbig!(0));
    assert_eq!(dbig!(0) >> 1, dbig!(0));
    assert_eq!(dbig!(1) << 1, dbig!(1e1));
    assert_eq!(dbig!(1) >> 1, dbig!(1e-1));
    assert_eq!(dbig!(-1) << 1, dbig!(-1e1));
    assert_eq!(dbig!(-1) >> 1, dbig!(-1e-1));
}

#[test]
#[should_panic]
fn test_shift_inf() {
    let _ = DBig::INFINITY >> 1;
}

/// A shift is an exact multiplication by a power of the base, and overflows / underflows like
/// one when the exponent leaves the finite range: the result saturates through
/// `Context::unwrap_fp` instead of carrying a nonzero significand to a sentinel exponent (or
/// wrapping the exponent around). Arithmetic on the result must not panic.
#[test]
fn test_shift_exponent_saturation() {
    use dashu_base::Sign;
    use dashu_float::{
        round::{mode, Round},
        Context, FBig, FpError, Repr, Word,
    };

    fn check<R: Round, const B: Word>() {
        let (min, max) = (isize::MIN, isize::MAX);
        for precision in [1usize, 20, 50, 100, 500] {
            let ctx = Context::<R>::new(precision);
            for sig in [1i32, -1, 3, -3] {
                let sign = Sign::from(sig < 0);
                for exp in [0isize, 5, -5, max - 1, min + 1] {
                    let x = FBig::<R, B>::from_repr(Repr::new(sig.into(), exp), ctx);
                    for amount in [0isize, 1, -1, 10, max - 1, max, min + 1, min, -max] {
                        // (shl by `amount`, shr by `amount`) as wide exponents
                        let cases = [
                            (x.clone() << amount, exp as i128 + amount as i128),
                            (x.clone() >> amount, exp as i128 - amount as i128),
                        ];
                        let mut assigned = (x.clone(), x.clone());
                        assigned.0 <<= amount;
                        assigned.1 >>= amount;
                        assert_eq!(assigned.0, cases[0].0);
                        assert_eq!(assigned.1, cases[1].0);
                        for (got, exponent) in cases {
                            let expected = if exponent >= max as i128 {
                                ctx.unwrap_fp(Err(FpError::Overflow(sign)))
                            } else if exponent <= min as i128 {
                                ctx.unwrap_fp(Err(FpError::Underflow(sign)))
                            } else {
                                FBig::from_repr(Repr::new(sig.into(), exponent as isize), ctx)
                            };
                            assert_eq!(got, expected, "{x:?} by {amount}");
                            assert_eq!(got.sign(), expected.sign());
                            assert_eq!(got.precision(), precision);
                            if !got.repr().is_infinite() {
                                // the result is usable in arithmetic
                                assert!((&got - &got).repr().significand().is_zero());
                            }
                        }
                    }
                }
            }
            // zeros are unchanged by any shift
            for zero in [Repr::<B>::zero(), Repr::<B>::neg_zero()] {
                let z = FBig::<R, B>::from_repr(zero, ctx);
                for amount in [1isize, max, min] {
                    assert_eq!((z.clone() << amount).sign(), z.sign());
                    assert!((z.clone() >> amount).repr().significand().is_zero());
                }
            }
        }
    }
    check::<mode::Zero, 2>();
    check::<mode::HalfEven, 2>();
    check::<mode::Up, 10>();
    check::<mode::Down, 10>();
    check::<mode::HalfAway, 10>();

    // nearest modes overflow to the infinity and underflow to zero
    let x = DBig::from_parts(1.into(), 0);
    assert_eq!(x.clone() << isize::MAX, DBig::INFINITY);
    assert_eq!(x.clone() >> -isize::MAX, DBig::INFINITY);
    assert!((x.clone() << isize::MIN).repr().is_pos_zero());
    assert_eq!((x.clone() << (isize::MAX - 1)).repr(), &Repr::new(1.into(), isize::MAX - 1));
    // also at unlimited precision
    assert_eq!(DBig::ONE << isize::MAX, DBig::INFINITY);
    assert_eq!(DBig::NEG_ONE << isize::MAX, DBig::NEG_INFINITY);
}

/// A shift by zero changes nothing — also for the values whose exponent is `isize::MAX` or
/// `isize::MIN`, which the saturating operations produce themselves (the largest finite value
/// under an inward mode, the smallest one under an outward mode) and which `from_parts`
/// accepts. The range check of a shift used to take them for an overflow / underflow:
/// `1·10^isize::MAX << 0` was `inf` (or `9·10^isize::MAX` under `mode::Zero`) and
/// `1·10^isize::MIN >> 0` was `0` (F88).
#[test]
fn test_shift_by_zero_is_identity() {
    use dashu_base::Sign;
    use dashu_float::{
        round::{mode, Round},
        Context, FBig, FpError, Repr, Word,
    };

    fn check<R: Round, const B: Word>() {
        for precision in [1usize, 20, 50, 100, 500] {
            let ctx = Context::<R>::new(precision);
            let mut values = Vec::new();
            for sig in [1i32, -1, 3, -3] {
                let exponents = [
                    0,
                    -7,
                    isize::MAX - 1,
                    isize::MAX,
                    isize::MIN + 1,
                    isize::MIN,
                ];
                for exp in exponents {
                    values.push(FBig::<R, B>::from_repr(Repr::new(sig.into(), exp), ctx));
                }
            }
            // the saturation endpoints of this mode and precision
            for sign in [Sign::Positive, Sign::Negative] {
                values.push(ctx.unwrap_fp(Err(FpError::Overflow(sign))));
                values.push(ctx.unwrap_fp(Err(FpError::Underflow(sign))));
            }
            values.push(FBig::from_repr(Repr::zero(), ctx));
            values.push(FBig::from_repr(Repr::neg_zero(), ctx));
            for x in values {
                if x.repr().is_infinite() {
                    continue; // infinities are not shifted at all
                }
                let cases = [x.clone() << 0, x.clone() >> 0];
                let mut assigned = (x.clone(), x.clone());
                assigned.0 <<= 0;
                assigned.1 >>= 0;
                for got in cases.iter().chain([&assigned.0, &assigned.1]) {
                    assert_eq!(got.repr(), x.repr(), "{x:?} shifted by 0");
                    assert_eq!(got.sign(), x.sign());
                    assert_eq!(got.precision(), precision);
                }
            }
        }
    }
    check::<mode::Zero, 10>();
    check::<mode::HalfEven, 10>();
    check::<mode::HalfAway, 2>();
    check::<mode::Up, 2>();
    check::<mode::Down, 3>();
    check::<mode::Away, 10>();

    // A nonzero shift onto a sentinel exponent is still an overflow / underflow, so such a
    // value does not survive a round trip — like a multiplication by the base would not.
    let x = DBig::from_parts(1.into(), isize::MAX);
    assert_eq!(x.clone() << 0, x);
    assert_eq!((x.clone() >> 1).repr(), &Repr::new(1.into(), isize::MAX - 1));
    assert_eq!((x.clone() >> 1) << 1, DBig::INFINITY);
    let y = DBig::from_parts(1.into(), isize::MIN);
    assert_eq!(y.clone() >> 0, y);
    assert!(((y << 1) >> 1).repr().is_pos_zero());
}
