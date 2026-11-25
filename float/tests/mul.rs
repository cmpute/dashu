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

    /// `fma` judges the exponent range on the rounded result `c + sign·(a·b)`, not on the
    /// product: a product below the range next to an ordinary `c` only decides the rounding
    /// direction (as a tiny addend does in `add`), and a product above the range can be
    /// cancelled by `c`. It used to return the product's `Underflow` / `Overflow`, so
    /// `FBig::fma` gave `0` for `3 + 2^(isize::MIN - 2)` (F16).
    #[test]
    fn test_fma_product_outside_exponent_range() {
        const MIN: isize = isize::MIN;
        const MAX: isize = isize::MAX;
        type Operand = (i128, isize);

        /// The same operation with the exponents of `a`, `b` moved by `-sa`, `-sb` (and
        /// `c`'s by `-(sa + sb)`), which puts everything in range: the result is the same
        /// up to the factor `B^(sa + sb)`, whose exponent decides the range error.
        fn check_shifted<R: Round, const B: Word>(
            precision: usize,
            (a, b, c): (Operand, Operand, Operand),
            (sa, sb): (isize, isize),
            sign: Sign,
        ) {
            let ctx = Context::<R>::new(precision);
            let shift = sa as i128 + sb as i128;
            let got = ctx.fma(&repr::<B>(a.0, a.1), &repr(b.0, b.1), &repr(c.0, c.1), sign);
            let c_shifted = isize::try_from(c.1 as i128 - shift).unwrap();
            let (ra, rb) = (repr::<B>(a.0, a.1 - sa), repr::<B>(b.0, b.1 - sb));
            let reference = ctx.fma(&ra, &rb, &repr(c.0, c_shifted), sign).unwrap();
            let value = reference.value_ref().repr();
            let exponent = value.exponent() as i128 + shift;
            let expected = if value.significand().is_zero() {
                Ok(reference)
            } else if exponent >= MAX as i128 {
                Err(FpError::Overflow(value.sign()))
            } else if exponent <= MIN as i128 {
                Err(FpError::Underflow(value.sign()))
            } else {
                Ok(reference.map(|v| {
                    let signif = v.repr().significand().clone();
                    FBig::from_repr(Repr::new(signif, exponent as isize), ctx)
                }))
            };
            assert_eq!(got, expected, "fma({a:?}, {b:?}, {c:?}, {sign:?}) at {precision}");
            if let Ok(v) = got {
                let (v, e) = (v.value(), expected.unwrap().value());
                assert_eq!(v.sign(), e.sign());
                assert_eq!(v.precision(), precision);
            }
        }

        fn check<R: Round, const B: Word>() {
            for precision in [20usize, 50, 100, 500] {
                let ctx = Context::<R>::new(precision);
                let full = IBig::from(B).pow(precision) - IBig::ONE; // all the digits in use
                let wider = IBig::from(B).pow(precision + 5) + IBig::ONE;
                let cs = [
                    repr::<B>(3, 0),
                    repr::<B>(-3, 0),
                    repr::<B>(7, -3),
                    repr::<B>(1, 1000),
                    Repr::<B>::new(full.clone(), -4),
                    Repr::<B>::new(-full, 9),
                    Repr::<B>::new(wider, -(precision as isize)),
                ];
                for sign in [Sign::Positive, Sign::Negative] {
                    // a product far below the range next to an ordinary c: like adding
                    // (or subtracting) the smallest value of the product's sign
                    for (a, b) in [(1i128, 1i128), (-1, 1), (3, 7), (5, -9)] {
                        let (x, y) = (repr::<B>(a, MIN / 2), repr::<B>(b, MIN / 2 - 2));
                        let tiny = repr::<B>(a.signum() * b.signum(), MIN + 1);
                        for c in &cs {
                            let got = ctx.fma(&x, &y, c, sign);
                            let reference = match sign {
                                Sign::Positive => ctx.add(c, &tiny),
                                Sign::Negative => ctx.sub(c, &tiny),
                            };
                            assert!(matches!(reference, Ok(Inexact(_, _))));
                            assert_eq!(got, reference, "fma({x:?}, {y:?}, {c:?}, {sign:?})");
                            let v = got.unwrap().value();
                            assert_eq!(v.precision(), precision);

                            if c.digits() > precision {
                                continue; // not a value of this context
                            }
                            let fx = FBig::from_repr(x.clone(), ctx);
                            let fy = FBig::from_repr(y.clone(), ctx);
                            let fc = FBig::from_repr(c.clone(), ctx);
                            assert_eq!(fx.fma(&fy, &fc, sign).repr(), v.repr());
                        }
                    }

                    // a product below the range and a c just inside it: comparable
                    // magnitudes, the sum is formed exactly and rounded
                    let products = [
                        (3i128, 1i128, 5i128),
                        (7, -9, 5),
                        (-1, 1, 1),
                        (11, 13, -143),
                    ];
                    for c_exp in [MIN + 1, MIN + 2, MIN + 3, MIN + 30, MIN + 700] {
                        for (a, b, c) in products {
                            let operands = ((a, MIN / 2), (b, MIN / 2 - 2), (c, c_exp));
                            check_shifted::<R, B>(precision, operands, (MIN / 2, MIN / 2), sign);
                        }
                    }

                    // a product above the range and a c just inside it, which may cancel it
                    let base = B as i128;
                    let products = [
                        (1i128, 1i128, 1 - base), // B^MAX - (B - 1)·B^(MAX - 1)
                        (1, 1, base - 1),
                        (1, -1, base * base - 1),
                        (3, 7, -209),
                        (-5, 9, 3),
                    ];
                    for c_exp in [MAX - 1, MAX - 2, MAX - 3, MAX - 40, MAX - 700] {
                        for (a, b, c) in products {
                            // exponents (MAX/2 + 1) + (MAX/2) = MAX
                            let operands = ((a, MAX / 2 + 1), (b, MAX / 2), (c, c_exp));
                            check_shifted::<R, B>(precision, operands, (MAX / 2, MAX / 2), sign);
                            let operands = ((a, MAX / 2 + 1), (b, MAX / 2 + 3), (c, c_exp));
                            check_shifted::<R, B>(precision, operands, (MAX / 2, MAX / 2), sign);
                        }
                    }

                    // a product far above the range next to an ordinary c is an overflow
                    // (far: more than `precision + 1` digits, so that no rounding of the sum
                    // has an exponent in range)
                    let (x, y) = (repr::<B>(3, MAX / 2 + 1), repr::<B>(-7, MAX / 2 + 1000));
                    for c in &cs {
                        let overflow = Err(FpError::Overflow(-sign));
                        assert_eq!(ctx.fma(&x, &y, c, sign), overflow);
                    }
                }
            }

            // unlimited precision: the exact sum is as wide as the distance between c and
            // the product, so a distant out-of-range product still reports its range error
            let ctx = Context::<R>::new(0);
            let (x, y, c) = (repr::<B>(1, MIN / 2), repr::<B>(1, MIN / 2 - 2), repr::<B>(3, 0));
            let underflow = Err(FpError::Underflow(Sign::Positive));
            assert_eq!(ctx.fma(&x, &y, &c, Sign::Positive), underflow);
            // ... while a nearby one is added exactly
            let (x, y) = (repr::<B>(1, MAX / 2 + 1), repr::<B>(1, MAX / 2));
            let c = repr::<B>(1 - B as i128, MAX - 1);
            let got = ctx.fma(&x, &y, &c, Sign::Positive).unwrap();
            assert_eq!(got, Exact(FBig::from_repr(repr::<B>(1, MAX - 1), ctx)));
        }
        check::<mode::HalfEven, 2>();
        check::<mode::HalfAway, 10>();
        check::<mode::Zero, 2>();
        check::<mode::Away, 10>();
        check::<mode::Up, 10>();
        check::<mode::Down, 2>();
        check::<mode::Down, 3>();

        // the reported case
        let ctx = Context::<mode::HalfEven>::new(10);
        let (a, b) = (repr::<2>(1, MIN / 2), repr::<2>(1, MIN / 2 - 2));
        let three = FBig::<mode::HalfEven, 2>::from(3)
            .with_precision(10)
            .value();
        let got = ctx.fma(&a, &b, three.repr(), Sign::Positive);
        assert_eq!(got, Ok(Inexact(three.clone(), dashu_float::round::Rounding::NoOp)));
        let (fa, fb) = (FBig::from_repr(a, ctx), FBig::from_repr(b, ctx));
        assert_eq!(fa.fma(&fb, &three, Sign::Positive), three);
    }
}
