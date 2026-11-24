use core::{
    fmt::Debug,
    ops::{Mul, MulAssign},
};
use dashu_base::Approximation::*;
use dashu_float::{round::Rounding::*, Context};

mod helper_macros;

fn test_mul<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Mul<T, Output = T>,
    T: Mul<&'a T, Output = T>,
    &'a T: Mul<T, Output = T>,
    &'a T: Mul<&'a T, Output = T>,
    T: MulAssign<T>,
    T: MulAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
    assert_eq!(a * b, *c);
    assert_eq!(a.clone() * b, *c);
    assert_eq!(a * b.clone(), *c);
    assert_eq!(a.clone() * b.clone(), *c);

    let mut x = a.clone();
    x *= b;
    assert_eq!(x, *c);

    let mut x = a.clone();
    x *= b.clone();
    assert_eq!(x, *c);
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_mul_binary() {
    let exact_cases = [
        (fbig!(0), fbig!(0), fbig!(0)),
        (fbig!(0), fbig!(1), fbig!(0)),
        (fbig!(0), fbig!(-1), fbig!(0)),
        (fbig!(0x1000), fbig!(0), fbig!(0)),
        (fbig!(1), fbig!(-1), fbig!(-1)),
        (fbig!(0x12p1), fbig!(0x34p-1), fbig!(0x75p3)),
        (fbig!(0x056p2), fbig!(0x078p4), fbig!(0x285p10)),
    ];

    for (a, b, c) in &exact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Ok(Exact(v)) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    // for inexact division with rounding to zero, the rounding error is always NoOp
    let inexact_cases = [
        (fbig!(0xa), fbig!(0xb), fbig!(0xdp3)),
        (fbig!(0x13), fbig!(-0x17), fbig!(-0xdap1)),
        (fbig!(-0x56p2), fbig!(-0x78p4), fbig!(0xa1p12)),
    ];

    for (a, b, c) in &inexact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_mul_decimal() {
    let exact_cases = [
        (dbig!(0), dbig!(0), dbig!(0)),
        (dbig!(0), dbig!(1), dbig!(0)),
        (dbig!(0), dbig!(-1), dbig!(0)),
        (dbig!(1000), dbig!(0), dbig!(0)),
        (dbig!(1), dbig!(-1), dbig!(-1)),
        (dbig!(012e1), dbig!(034e-1), dbig!(408)),
        (dbig!(0056e2), dbig!(0078e4), dbig!(4368e6)),
        (dbig!(25), dbig!(16), dbig!(4e2)),
        (dbig!(25), dbig!(40), dbig!(1e3)),
        (dbig!(25), dbig!(64), dbig!(16e2)),
    ];

    for (a, b, c) in &exact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Ok(Exact(v)) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
        } else {
            panic!("the result should be exact!")
        }
    }

    // for inexact division with rounding to zero, the rounding error is always NoOp
    let inexact_cases = [
        (dbig!(7), dbig!(8), dbig!(6e1), AddOne),
        (dbig!(13), dbig!(-17), dbig!(-22e1), NoOp),
        (dbig!(-12e1), dbig!(34e-1), dbig!(-41e1), SubOne),
        (dbig!(-56e2), dbig!(-78e4), dbig!(44e8), AddOne),
    ];

    for (a, b, c, rnd) in &inexact_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);

        if let Ok(Inexact(v, e)) = Context::max(a.context(), b.context()).mul(a.repr(), b.repr()) {
            assert_eq!(v, *c);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
#[should_panic]
fn test_mul_by_inf() {
    let _ = dashu_float::DBig::ONE * dashu_float::DBig::INFINITY;
}

#[test]
fn test_mul_zero_precision() {
    let a = fbig!(0xff).with_precision(0).unwrap();
    let b = fbig!(-0xff);
    test_mul(&a, &a, &fbig!(0xfe01));
    test_mul(&a, &b, &fbig!(-0xfep8));

    let a = dbig!(99).with_precision(0).unwrap();
    let b = dbig!(-99);
    test_mul(&a, &a, &dbig!(9801));
    test_mul(&a, &b, &dbig!(-98e2));
}

// ---------------------------------------------------------------------------
// Regressions for the multiplication / fma reports of the Hegel triage
// (see triage/float-arith.md).
// ---------------------------------------------------------------------------

mod triage {
    use dashu_base::{Approximation::*, Sign};
    use dashu_float::{
        round::{mode, Round},
        Context, FBig, FpError, Repr, Word,
    };
    use dashu_int::{IBig, UBig};

    fn repr<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(sig.into(), exp)
    }

    /// 2^(64·words) + 1: an odd multi-word significand (wide enough operands take the
    /// short-product path)
    fn wide(words: usize) -> IBig {
        (IBig::ONE << (64 * words)) + IBig::ONE
    }

    /// A product that needs no rounding is `Exact`, whichever path computes it. The
    /// short-product path used to tag an exactly representable product of wide operands
    /// `Inexact(_, NoOp)`.
    #[test]
    fn test_mul_exact_product_of_wide_operands_is_exact() {
        // the reported case: 2^1408 · 5^633 = 2^775 · 10^633, 234 digits
        let ctx = Context::<mode::Zero>::new(339);
        let a = Repr::<10>::new((UBig::ONE << 1408).into(), 0);
        let b = Repr::<10>::new(UBig::from(5u8).pow(633).into(), 0);
        let got = ctx.mul(&a, &b).unwrap();
        assert!(matches!(got, Exact(_)));
        assert_eq!(got.value().repr(), &(&a * &b));

        fn check<R: Round>() {
            for precision in [20usize, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                for (twos, fives) in [(100usize, 50usize), (1408, 633), (3000, 2500), (6000, 1)] {
                    let a = Repr::<10>::new((UBig::ONE << twos).into(), 0);
                    let b = Repr::<10>::new(UBig::from(5u8).pow(fives).into(), 0);
                    let exact = &a * &b;
                    for got in [ctx.mul(&a, &b), ctx.mul(&b, &a)] {
                        let got = got.unwrap();
                        assert_eq!(matches!(got, Exact(_)), exact.digits() <= precision);
                        if exact.digits() <= precision {
                            assert_eq!(got.value().repr(), &exact);
                        }
                    }
                    // squares and cubes of an exactly representable power
                    let c = Repr::<10>::new((UBig::ONE << twos).into(), 0);
                    let sqr = ctx.sqr(&c).unwrap();
                    assert_eq!(matches!(sqr, Exact(_)), (&c * &c).digits() <= precision);
                    let cubic = ctx.cubic(&c).unwrap();
                    assert_eq!(matches!(cubic, Exact(_)), (&(&c * &c) * &c).digits() <= precision);
                }
            }
        }
        check::<mode::Zero>();
        check::<mode::HalfEven>();
        check::<mode::Up>();
    }

    /// The exponent range of a product is judged on the rounded result, not on the sum of the
    /// operand exponents: whether a product is in range must not depend on how many words
    /// its operands have. Narrow operands whose exponents sum to exactly `isize::MIN` used to
    /// report `Underflow` where wide ones returned the (representable) rounded product.
    #[test]
    fn test_mul_exponent_range_is_checked_after_rounding() {
        fn check<R: Round, const B: Word>() {
            let (min, max) = (isize::MIN, isize::MAX);
            for precision in [1usize, 20, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                let sigs = [
                    IBig::from(3),
                    IBig::from(5),
                    IBig::from(1023),
                    wide(8),
                    wide(20),
                ];
                for a_sig in &sigs {
                    for b_sig in &sigs {
                        // the product at ordinary exponents is the reference
                        let reference = ctx
                            .mul(
                                &Repr::<B>::new(a_sig.clone(), 0),
                                &Repr::<B>::new(b_sig.clone(), 0),
                            )
                            .unwrap();
                        let (ref_exact, reference) =
                            (matches!(reference, Exact(_)), reference.value());
                        let ref_exp = reference.repr().exponent() as i128;
                        for (a_exp, b_exp) in [
                            (min + 1, -1isize),
                            (min + 1, -2),
                            (min / 2, min / 2),
                            (min / 2 - 3, min / 2),
                            (max - 1, 1),
                            (max / 2 + 1, max / 2 + 1),
                            (max - 500, 400),
                        ] {
                            let a = Repr::<B>::new(a_sig.clone(), a_exp);
                            let b = Repr::<B>::new(b_sig.clone(), b_exp);
                            let exponent = ref_exp + a_exp as i128 + b_exp as i128;
                            let got = ctx.mul(&a, &b);
                            if exponent >= max as i128 {
                                assert_eq!(got, Err(FpError::Overflow(Sign::Positive)));
                            } else if exponent <= min as i128 {
                                assert_eq!(got, Err(FpError::Underflow(Sign::Positive)));
                            } else {
                                let got = got.unwrap();
                                assert_eq!(matches!(got, Exact(_)), ref_exact);
                                let got = got.value();
                                assert_eq!(
                                    got.repr().significand(),
                                    reference.repr().significand()
                                );
                                assert_eq!(got.repr().exponent() as i128, exponent);
                            }
                        }
                    }
                    // the same for the square
                    let reference = ctx.sqr(&Repr::<B>::new(a_sig.clone(), 0)).unwrap().value();
                    for a_exp in [min / 2, min / 2 - 1, min / 2 + 1, max / 2, max / 2 + 1] {
                        let exponent = reference.repr().exponent() as i128 + 2 * a_exp as i128;
                        let got = ctx.sqr(&Repr::<B>::new(a_sig.clone(), a_exp));
                        if exponent >= max as i128 {
                            assert_eq!(got, Err(FpError::Overflow(Sign::Positive)));
                        } else if exponent <= min as i128 {
                            assert_eq!(got, Err(FpError::Underflow(Sign::Positive)));
                        } else {
                            let got = got.unwrap().value();
                            assert_eq!(got.repr().significand(), reference.repr().significand());
                            assert_eq!(got.repr().exponent() as i128, exponent);
                        }
                    }
                    // and the cube
                    let reference = ctx
                        .cubic(&Repr::<B>::new(a_sig.clone(), 0))
                        .unwrap()
                        .value();
                    for a_exp in [min / 3, min / 3 - 1, max / 3, max / 3 + 1] {
                        let exponent = reference.repr().exponent() as i128 + 3 * a_exp as i128;
                        let got = ctx.cubic(&Repr::<B>::new(a_sig.clone(), a_exp));
                        if exponent >= max as i128 {
                            assert_eq!(got, Err(FpError::Overflow(Sign::Positive)));
                        } else if exponent <= min as i128 {
                            assert_eq!(got, Err(FpError::Underflow(Sign::Positive)));
                        } else {
                            let got = got.unwrap().value();
                            assert_eq!(got.repr().significand(), reference.repr().significand());
                            assert_eq!(got.repr().exponent() as i128, exponent);
                        }
                    }
                }
            }
        }
        check::<mode::HalfEven, 2>();
        check::<mode::Zero, 2>();
        check::<mode::Up, 10>();
        check::<mode::HalfAway, 10>();

        // the reported cases: 15·2^isize::MIN and 9·2^isize::MIN round to a single bit
        let ctx = Context::<mode::HalfEven>::new(1);
        let got = ctx.mul(&repr::<2>(3, isize::MIN + 1), &repr::<2>(5, -1));
        assert_eq!(got.unwrap().value().repr(), &repr::<2>(1, isize::MIN + 4));
        let got = ctx.sqr(&repr::<2>(3, isize::MIN / 2));
        assert_eq!(got.unwrap().value().repr(), &repr::<2>(1, isize::MIN + 3));
    }

    /// Results with a nonzero significand at exponent `isize::MIN` are reported as an
    /// underflow, and the outward modes then saturate to `1·B^isize::MIN` — which is *below*
    /// an exact result such as `3·2^isize::MIN`, so `mode::Up` returns less than the exact
    /// value. A sound treatment needs gradual underflow (or `isize::MIN` as a regular
    /// exponent) across all kernels and the saturation endpoints.
    #[test]
    #[ignore = "E145: needs a redesign of the exponent-range / underflow-endpoint model"]
    fn test_directed_rounding_at_minimum_exponent() {
        type F = FBig<mode::Up, 2>;
        let h = isize::MIN / 2;
        let scaled = |f: &F| -> IBig {
            let k = (f.repr().exponent() as i128 - isize::MIN as i128) as usize;
            f.repr().significand() << k
        };
        // exact product 3·2^isize::MIN
        let p = &F::from_parts(IBig::from(3), h) * &F::from_parts(IBig::ONE, h);
        assert!(scaled(&p) >= IBig::from(3), "{:?}", p.repr());
        // exact quotient 2^(isize::MIN+3) / 3
        let q = F::from_parts(IBig::ONE, isize::MIN + 80) / F::from_parts(IBig::from(3), 77);
        assert!(IBig::from(3) * scaled(&q) >= IBig::from(8), "{:?}", q.repr());
    }

    /// `fma` with a zero product follows the IEEE 754 §6.3 rule for a sum of zeros, exactly
    /// like `add`/`sub` on the (signed) zero product and `c`. The zero product used to be
    /// dropped, so the result was `c` whatever the product's sign and the rounding mode.
    #[test]
    fn test_fma_zero_product_sign() {
        fn check<R: Round, const B: Word>() {
            for precision in [20usize, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                let zeros = [Repr::<B>::zero(), Repr::<B>::neg_zero()];
                let others = [repr::<B>(1, 0), repr::<B>(-1, 0), repr::<B>(7, -3)];
                for zero in &zeros {
                    for other in others.iter().chain(zeros.iter()) {
                        for c in zeros.iter().chain(others.iter()) {
                            for sign in [Sign::Positive, Sign::Negative] {
                                for (a, b) in [(zero, other), (other, zero)] {
                                    let got = ctx.fma(a, b, c, sign).unwrap().value();
                                    let prod = Context::<R>::new(0).mul(a, b).unwrap().value();
                                    let reference = match sign {
                                        Sign::Positive => ctx.add(c, prod.repr()),
                                        Sign::Negative => ctx.sub(c, prod.repr()),
                                    }
                                    .unwrap()
                                    .value();
                                    assert_eq!(got.repr(), reference.repr());
                                    assert_eq!(
                                        got.sign(),
                                        reference.sign(),
                                        "fma({a:?}, {b:?}, {c:?}, {sign:?})"
                                    );
                                    let x = FBig::<R, B>::from_repr(a.clone(), ctx);
                                    let y = FBig::<R, B>::from_repr(b.clone(), ctx);
                                    let z = FBig::<R, B>::from_repr(c.clone(), ctx);
                                    assert_eq!(x.fma(&y, &z, sign).sign(), reference.sign());
                                }
                            }
                        }
                    }
                }
                // a zero c and a nonzero product: the result is the (signed) product
                for (a, b) in [(1i128, 3i128), (-5, 7), (11, -13)] {
                    for c in &zeros {
                        for sign in [Sign::Positive, Sign::Negative] {
                            let got = ctx.fma(&repr::<B>(a, 2), &repr::<B>(b, -4), c, sign);
                            let signed = if sign == Sign::Negative {
                                -a * b
                            } else {
                                a * b
                            };
                            assert_eq!(got.unwrap().value().repr(), &repr::<B>(signed, -2));
                        }
                    }
                }
            }
        }
        check::<mode::HalfEven, 2>();
        check::<mode::Down, 2>();
        check::<mode::Down, 10>();
        check::<mode::Up, 10>();
        check::<mode::Zero, 2>();
        check::<mode::HalfAway, 10>();

        // the reported cases
        let (pz, nz, one) = (Repr::<2>::zero(), Repr::<2>::neg_zero(), repr::<2>(1, 0));
        let ctx = Context::<mode::HalfEven>::new(10);
        assert!(ctx
            .fma(&pz, &one, &nz, Sign::Positive)
            .unwrap()
            .value()
            .repr()
            .is_pos_zero());
        let ctx = Context::<mode::Down>::new(10);
        let neg_one = repr::<2>(-1, 0);
        assert!(ctx
            .fma(&pz, &neg_one, &pz, Sign::Positive)
            .unwrap()
            .value()
            .repr()
            .is_neg_zero());
        assert!(ctx
            .fma(&pz, &one, &pz, Sign::Negative)
            .unwrap()
            .value()
            .repr()
            .is_neg_zero());
        // a nonzero product cancelling exactly is unchanged: -0 under roundTowardNegative
        let got = ctx.fma(&repr::<2>(2, 0), &repr::<2>(3, 0), &repr::<2>(-6, 0), Sign::Positive);
        assert!(got.unwrap().value().repr().is_neg_zero());
    }
}
