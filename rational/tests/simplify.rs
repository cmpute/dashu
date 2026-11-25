use dashu_base::{Approximation::*, Sign::*};
use dashu_ratio::RBig;

mod helper_macros;

#[test]
fn test_simplest_in() {
    assert_eq!(RBig::simplest_in(rbig!(0), rbig!(0)), rbig!(0));
    assert_eq!(RBig::simplest_in(rbig!(-1), rbig!(1)), rbig!(0));
    assert_eq!(RBig::simplest_in(rbig!(-1), rbig!(-1)), rbig!(-1));
    assert_eq!(RBig::simplest_in(rbig!(2 / 7), rbig!(2 / 9)), rbig!(1 / 4));
    assert_eq!(RBig::simplest_in(rbig!(-20 / 7), rbig!(-20 / 9)), rbig!(-5 / 2));
    assert_eq!(RBig::simplest_in(rbig!(5), rbig!(7)), rbig!(6));
    assert_eq!(RBig::simplest_in(rbig!(5), rbig!(6)), rbig!(11 / 2));
}

#[test]
fn test_simplest_from_f32() {
    assert_eq!(RBig::simplest_from_f32(0f32), Some(rbig!(0)));
    assert_eq!(RBig::simplest_from_f32(f32::INFINITY), None);
    assert_eq!(RBig::simplest_from_f32(f32::NEG_INFINITY), None);
    assert_eq!(RBig::simplest_from_f32(f32::NAN), None);

    let cases = [
        // (numerator, denominator)
        // NOTE: make sure each of these numbers fit in a f32
        (1i32, 1u32),
        (2, 1),
        (-3, 1),
        (1, 2),
        (-1, 3),
        (1, 4),
        (-1, 5),
        // convergents of pi
        (22, 7),
        (333, 106),
        (355, 113),
    ];
    for (num, den) in cases {
        let f = num as f32 / den as f32;
        assert_eq!(RBig::simplest_from_f32(f).unwrap(), RBig::from_parts(num.into(), den.into()));
    }
}

#[test]
fn test_simplest_from_f64() {
    assert_eq!(RBig::simplest_from_f64(0f64), Some(rbig!(0)));
    assert_eq!(RBig::simplest_from_f64(f64::INFINITY), None);
    assert_eq!(RBig::simplest_from_f64(f64::NEG_INFINITY), None);
    assert_eq!(RBig::simplest_from_f64(f64::NAN), None);

    let cases = [
        // (numerator, denominator)
        // NOTE: make sure each of these numbers fit in a f64
        (1i64, 1u64),
        (2, 1),
        (-3, 1),
        (1, 2),
        (-1, 3),
        (1, 4),
        (-1, 5),
        // convergents of pi
        (22, 7),
        (333, 106),
        (355, 113),
        (103993, 33102),
        (104348, 33215),
        (208341, 66317),
        (312689, 99532),
        (833719, 265381),
        (1146408, 364913),
        (4272943, 1360120),
        (5419351, 1725033),
        (80143857, 25510582),
        (165707065, 52746197),
        (245850922, 78256779),
    ];
    for (num, den) in cases {
        let f = num as f64 / den as f64;
        assert_eq!(RBig::simplest_from_f64(f).unwrap(), RBig::from_parts(num.into(), den.into()));
    }
}

#[test]
fn test_nearest() {
    // test trivial cases
    assert_eq!(rbig!(0).nearest(&ubig!(1)), Exact(rbig!(0)));
    assert_eq!(rbig!(1).nearest(&ubig!(2)), Exact(rbig!(1)));
    assert_eq!(rbig!(-1).nearest(&ubig!(3)), Exact(rbig!(-1)));

    // test general cases
    let test_cases = [
        // (value, limit, next down, next up, sign of nearest - value)
        (core::f64::consts::PI, 10u64, rbig!(25 / 8), rbig!(22 / 7), Positive),
        (core::f64::consts::PI, 100, rbig!(311 / 99), rbig!(22 / 7), Negative),
        (core::f64::consts::PI, 1000, rbig!(2818 / 897), rbig!(355 / 113), Positive),
        (core::f64::consts::PI, 10000, rbig!(31218 / 9937), rbig!(355 / 113), Positive),
        (
            core::f64::consts::PI,
            100000,
            rbig!(208341 / 66317),
            rbig!(312689 / 99532),
            Positive,
        ),
        (core::f64::consts::SQRT_2, 10, rbig!(7 / 5), rbig!(10 / 7), Negative),
        (core::f64::consts::SQRT_2, 100, rbig!(140 / 99), rbig!(99 / 70), Negative),
        (core::f64::consts::SQRT_2, 1000, rbig!(1393 / 985), rbig!(577 / 408), Negative),
        (
            core::f64::consts::SQRT_2,
            10000,
            rbig!(8119 / 5741),
            rbig!(11482 / 8119),
            Positive,
        ),
        (
            core::f64::consts::SQRT_2,
            100000,
            rbig!(47321 / 33461),
            rbig!(114243 / 80782),
            Positive,
        ),
    ];
    for (value, limit, down, up, cmp) in test_cases {
        let ratio: RBig = value.try_into().unwrap();
        assert_eq!(ratio.next_up(&limit.into()), up.clone());
        assert_eq!(ratio.next_down(&limit.into()), down.clone());
        if cmp == Positive {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(up.clone(), cmp));
        } else {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(down.clone(), cmp));
        }

        let ratio = -ratio;
        assert_eq!(ratio.next_down(&limit.into()), -up.clone());
        assert_eq!(ratio.next_up(&limit.into()), -down.clone());
        if cmp == Positive {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(-up.clone(), -cmp));
        } else {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(-down.clone(), -cmp));
        }
    }
}

#[test]
#[should_panic]
fn test_nearest_zero_limit() {
    let _ = rbig!(1 / 2).nearest(&ubig!(0));
}

#[test]
fn test_is_simpler_than() {
    // the denominator decides first
    assert!(rbig!(1).is_simpler_than(&rbig!(3 / 2)));
    assert!(rbig!(1000).is_simpler_than(&rbig!(1 / 2)));
    assert!(rbig!(-7 / 2).is_simpler_than(&rbig!(1 / 3)));
    assert!(!rbig!(3 / 2).is_simpler_than(&rbig!(1)));
    assert!(!rbig!(1 / 3).is_simpler_than(&rbig!(7 / 2)));
    // then the magnitude of the numerator
    assert!(rbig!(1 / 2).is_simpler_than(&rbig!(3 / 2)));
    assert!(rbig!(-1 / 2).is_simpler_than(&rbig!(3 / 2)));
    assert!(rbig!(6).is_simpler_than(&rbig!(7)));
    assert!(rbig!(0).is_simpler_than(&rbig!(-1)));
    assert!(!rbig!(3 / 2).is_simpler_than(&rbig!(-1 / 2)));
    assert!(!rbig!(7).is_simpler_than(&rbig!(6)));
    // then the sign
    assert!(rbig!(1).is_simpler_than(&rbig!(-1)));
    assert!(rbig!(2 / 3).is_simpler_than(&rbig!(-2 / 3)));
    assert!(!rbig!(-1).is_simpler_than(&rbig!(1)));
    // a number is not simpler than itself
    for r in [rbig!(0), rbig!(1), rbig!(-1), rbig!(2 / 3), rbig!(-2 / 3)] {
        assert!(!r.is_simpler_than(&r));
    }
}

/// The simplest number of the open interval `(lower, upper)` found by enumeration.
fn simplest_in_by_search(lower: &RBig, upper: &RBig) -> RBig {
    for den in 1u32.. {
        // the candidates with this denominator, by increasing magnitude of the numerator
        let mut best: Option<RBig> = None;
        let lo = (lower * RBig::from(den)).floor();
        let hi = (upper * RBig::from(den)).ceil();
        let mut num = lo;
        while num <= hi {
            let candidate = RBig::from_parts(num.clone(), den.into());
            if candidate > *lower && candidate < *upper && candidate.denominator() == &den.into() {
                let better = match &best {
                    None => true,
                    Some(b) => candidate.is_simpler_than(b),
                };
                if better {
                    best = Some(candidate);
                }
            }
            num += dashu_int::IBig::ONE;
        }
        if let Some(best) = best {
            return best;
        }
    }
    unreachable!()
}

#[test]
fn test_simplest_in_zero_endpoint() {
    // zero as an endpoint is not inside the open interval
    assert_eq!(RBig::simplest_in(rbig!(-1), rbig!(0)), rbig!(-1 / 2));
    assert_eq!(RBig::simplest_in(rbig!(0), rbig!(-1)), rbig!(-1 / 2));
    assert_eq!(RBig::simplest_in(rbig!(0), rbig!(1)), rbig!(1 / 2));
    assert_eq!(RBig::simplest_in(rbig!(1), rbig!(0)), rbig!(1 / 2));
    assert_eq!(RBig::simplest_in(rbig!(0), rbig!(-1 / 3)), rbig!(-1 / 4));
    assert_eq!(RBig::simplest_in(rbig!(-5), rbig!(0)), rbig!(-1));
    assert_eq!(RBig::simplest_in(rbig!(0), rbig!(7 / 2)), rbig!(1));
    assert_eq!(RBig::simplest_in(rbig!(-1), rbig!(1)), rbig!(0));
    assert_eq!(RBig::simplest_in(rbig!(-1 / 2), rbig!(-1 / 3)), rbig!(-2 / 5));
}

#[test]
fn test_simplest_in_against_search() {
    let mut endpoints = Vec::new();
    for den in 1i32..=9 {
        for num in -2 * den..=2 * den {
            endpoints.push(RBig::from_parts(num.into(), (den as u32).into()));
        }
    }
    endpoints.sort();
    endpoints.dedup();
    for (i, lower) in endpoints.iter().enumerate() {
        for upper in &endpoints[i + 1..] {
            let expected = simplest_in_by_search(lower, upper);
            assert_eq!(
                RBig::simplest_in(lower.clone(), upper.clone()),
                expected,
                "({lower}, {upper})"
            );
            assert_eq!(
                RBig::simplest_in(upper.clone(), lower.clone()),
                expected,
                "({upper}, {lower})"
            );
        }
    }
}

#[test]
fn test_simplest_from_float_interval() {
    // Above 2^53 (2^24) the floats are more than one apart, and the rounding interval
    // holds several integers: the simplest is the one with the least magnitude.
    // 2^54 + 8 has an even mantissa, so the midpoint 2^54 + 6 itself rounds to it.
    let f = 18014398509481992.0f64; // 2^54 + 8
    assert_eq!(18014398509481990u64 as f64, f);
    assert_eq!(18014398509481989u64 as f64, 18014398509481988.0);
    assert_eq!(RBig::simplest_from_f64(f), Some(rbig!(18014398509481990)));
    assert_eq!(RBig::simplest_from_f64(-f), Some(rbig!(-18014398509481990)));
    // 2^54 + 4 has an odd mantissa, the midpoints round to its neighbours
    let f = 18014398509481988.0f64;
    assert_eq!(18014398509481987u64 as f64, f);
    assert_eq!(18014398509481986u64 as f64, 18014398509481984.0);
    assert_eq!(RBig::simplest_from_f64(f), Some(rbig!(18014398509481987)));

    let g = 33554440.0f32; // 2^25 + 8
    assert_eq!(33554438u32 as f32, g);
    assert_eq!(RBig::simplest_from_f32(g), Some(rbig!(33554438)));
    assert_eq!(RBig::simplest_from_f32(33554436.0f32), Some(rbig!(33554435))); // 2^25 + 4

    let r = RBig::simplest_from_f64(1e20).unwrap();
    assert_eq!(r.to_f64().value(), 1e20);
    assert!(r.is_simpler_than(&rbig!(100000000000000000000)));

    // Below a power of two the floats are twice as dense, so the interval is narrower there.
    assert_eq!(RBig::simplest_from_f64(1.0), Some(rbig!(1)));
    assert_eq!(RBig::simplest_from_f32(0.5), Some(rbig!(1 / 2)));
    assert_eq!(RBig::simplest_from_f64(2f64.powi(-30)), Some(rbig!(1 / 0x40000000)));
    // 2^60: the interval is [2^60 - 2^6, 2^60 + 2^7]
    let f = 2f64.powi(60);
    assert_eq!(RBig::simplest_from_f64(f), Some(rbig!(0xfffffffffffffc0)));
    assert_eq!(0xfffffffffffffc0u64 as f64, f);
    assert_eq!(0xfffffffffffffbfu64 as f64, f - 128.);
    // 2^30 as f32: the interval is [2^30 - 2^5, 2^30 + 2^6]
    assert_eq!(RBig::simplest_from_f32(2f32.powi(30)), Some(rbig!(0x3fffffe0)));
    assert_eq!(0x3fffffe0u32 as f32, 2f32.powi(30));
    assert_eq!(0x3fffffdfu32 as f32, 2f32.powi(30) - 64.);
}

#[test]
fn test_simplest_from_float_roundtrip() {
    // the result always rounds back to the float, and it is never less simple than the
    // exact value of the float
    let mut state = 0x243f6a8885a308d3u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut f64_cases = vec![
        1.0,
        -1.0,
        0.1,
        1e-5,
        1e20,
        1e300,
        4503599627370498.0,
        9007199254740992.0,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::from_bits(2),
        f64::from_bits(0x000f_ffff_ffff_ffff),
        f64::from_bits(0x0010_0000_0000_0001),
        f64::EPSILON,
    ];
    for exp in [
        -1074i32, -1073, -1023, -1022, -1021, -53, -1, 0, 1, 52, 53, 54, 100, 1023,
    ] {
        f64_cases.push(2f64.powi(exp));
    }
    for _ in 0..300 {
        let f = f64::from_bits(next());
        if f.is_finite() {
            f64_cases.push(f);
        }
    }
    for f in f64_cases {
        let r = RBig::simplest_from_f64(f).unwrap();
        assert_eq!(r.to_f64().value(), f, "{f:e} -> {r}");
        assert!(!RBig::try_from(f).unwrap().is_simpler_than(&r), "{f:e} -> {r}");
        assert_eq!(RBig::simplest_from_f64(-f), Some(-r));
    }

    let mut f32_cases = vec![
        1.0f32,
        0.1,
        1e-5,
        1e20,
        16777216.0,
        33554432.0,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x007f_ffff),
        f32::from_bits(0x0080_0001),
        f32::EPSILON,
    ];
    for exp in [
        -149i32, -148, -127, -126, -125, -24, -1, 0, 1, 23, 24, 25, 60, 127,
    ] {
        f32_cases.push(2f32.powi(exp));
    }
    for _ in 0..300 {
        let f = f32::from_bits(next() as u32);
        if f.is_finite() {
            f32_cases.push(f);
        }
    }
    for f in f32_cases {
        let r = RBig::simplest_from_f32(f).unwrap();
        assert_eq!(r.to_f32().value(), f, "{f:e} -> {r}");
        assert!(!RBig::try_from(f).unwrap().is_simpler_than(&r), "{f:e} -> {r}");
        assert_eq!(RBig::simplest_from_f32(-f), Some(-r));
    }
}

#[test]
fn test_farey_neighbours_against_search() {
    use dashu_int::{IBig, UBig};

    // next_up / next_down / nearest against an enumeration of the fractions with a
    // denominator of at most `limit`
    for limit in 1u32..=9 {
        let mut farey = Vec::new();
        for den in 1..=limit {
            for num in -3 * den as i32..=3 * den as i32 {
                farey.push(RBig::from_parts(num.into(), den.into()));
            }
        }
        farey.sort();
        farey.dedup();
        let limit = UBig::from(limit);

        for den in 1u32..=24 {
            for num in -2 * den as i32..=2 * den as i32 {
                let x = RBig::from_parts(num.into(), den.into());
                let up = farey.iter().find(|f| **f > x).unwrap();
                let down = farey.iter().rev().find(|f| **f < x).unwrap();
                assert_eq!(x.next_up(&limit), *up, "next_up({x}, {limit})");
                assert_eq!(x.next_down(&limit), *down, "next_down({x}, {limit})");

                let nearest = x.nearest(&limit);
                if x.denominator() <= &limit {
                    assert_eq!(nearest, Exact(x.clone()));
                } else if &x - down <= up - &x {
                    // a tie goes to the lesser candidate
                    assert_eq!(nearest, Inexact(down.clone(), Negative), "nearest({x}, {limit})");
                } else {
                    assert_eq!(nearest, Inexact(up.clone(), Positive), "nearest({x}, {limit})");
                }

                // the documented error bounds (inclusive)
                let bound = RBig::from_parts(IBig::ONE, limit.clone());
                assert!(up - &x <= bound && &x - down <= bound);
                let diff = nearest.value() - &x;
                let abs_diff = if diff < rbig!(0) { -diff } else { diff };
                assert!(abs_diff <= bound / rbig!(2));
            }
        }
    }
}

#[test]
fn test_farey_neighbours_of_integers() {
    // The Farey sequence of order 1 is made of the integers; an integer is at a distance
    // of exactly 1/limit from its neighbours.
    for (x, limit, down, up) in [
        (rbig!(0), ubig!(1), rbig!(-1), rbig!(1)),
        (rbig!(5), ubig!(1), rbig!(4), rbig!(6)),
        (rbig!(-5), ubig!(1), rbig!(-6), rbig!(-4)),
        (rbig!(0), ubig!(2), rbig!(-1 / 2), rbig!(1 / 2)),
        (rbig!(5), ubig!(2), rbig!(9 / 2), rbig!(11 / 2)),
        (rbig!(0), ubig!(10), rbig!(-1 / 10), rbig!(1 / 10)),
        (rbig!(-3), ubig!(10), rbig!(-31 / 10), rbig!(-29 / 10)),
        (rbig!(1 / 2), ubig!(1), rbig!(0), rbig!(1)),
        (rbig!(-1 / 2), ubig!(1), rbig!(-1), rbig!(0)),
        (rbig!(1 / 2), ubig!(10), rbig!(4 / 9), rbig!(5 / 9)),
        (rbig!(3 / 7), ubig!(10), rbig!(2 / 5), rbig!(4 / 9)),
    ] {
        assert_eq!(x.next_up(&limit), up, "next_up({x}, {limit})");
        assert_eq!(x.next_down(&limit), down, "next_down({x}, {limit})");
    }

    // the error bound of nearest is reached at the midpoints, where the lesser one is chosen
    assert_eq!(rbig!(1 / 2).nearest(&ubig!(1)), Inexact(rbig!(0), Negative));
    assert_eq!(rbig!(-1 / 2).nearest(&ubig!(1)), Inexact(rbig!(-1), Negative));
    assert_eq!(rbig!(1 / 256).nearest(&ubig!(128)), Inexact(rbig!(0), Negative));
    assert_eq!(rbig!(3 / 512).nearest(&ubig!(128)), Inexact(rbig!(1 / 128), Positive));
    assert_eq!(rbig!(1 / 3).nearest(&ubig!(1)), Inexact(rbig!(0), Negative));
    assert_eq!(rbig!(2 / 3).nearest(&ubig!(1)), Inexact(rbig!(1), Positive));
}

#[test]
fn test_farey_neighbours_large_limit() {
    use dashu_int::{IBig, UBig};

    // the cost depends on the size of the limit in digits, not on its value
    let limit = ubig!(1000000000);
    assert_eq!(rbig!(0).next_down(&limit), rbig!(-1 / 1000000000));
    assert_eq!(rbig!(0).next_up(&limit), rbig!(1 / 1000000000));
    assert_eq!(rbig!(7).next_up(&limit), rbig!(7000000001 / 1000000000));

    let limit = ubig!(1000000000000);
    let tiny = RBig::try_from(1e-9f64).unwrap();
    let nearest = tiny.nearest(&limit);
    assert_eq!(nearest, Inexact(rbig!(1 / 1000000000), Negative));
    let pi = RBig::try_from(core::f64::consts::PI).unwrap();
    assert_eq!(pi.nearest(&limit), Inexact(rbig!(1257316528023 / 400216280932), Negative));

    let googol = UBig::from(10u8).pow(100);
    let values = [
        tiny.clone(),
        -tiny,
        pi.clone(),
        -pi,
        rbig!(1 / 3) + RBig::from_parts(IBig::ONE, UBig::from(7u8).pow(150)),
        rbig!(-22 / 7) - RBig::from_parts(IBig::ONE, UBig::from(3u8).pow(300)),
        RBig::from_parts(IBig::ONE, &googol * &googol + UBig::ONE),
        RBig::from_parts(IBig::from(5), ubig!(1) << 2000),
    ];
    for limit in [ubig!(1), ubig!(1000000007), ubig!(1) << 64, googol.clone()] {
        for x in &values {
            if x.denominator() <= &limit {
                continue;
            }
            let (down, up) = (x.next_down(&limit), x.next_up(&limit));
            assert!(down < *x && *x < up, "{x} with limit {limit}");
            assert!(down.denominator() <= &limit && up.denominator() <= &limit);
            // adjacent in the Farey sequence: the cross difference is one, and their
            // mediant is beyond the limit
            assert_eq!(
                up.numerator() * down.denominator() - down.numerator() * up.denominator(),
                IBig::ONE
            );
            assert!(down.denominator() + up.denominator() > limit);

            let expected = if x - &down <= &up - x {
                Inexact(down, Negative)
            } else {
                Inexact(up, Positive)
            };
            assert_eq!(x.nearest(&limit), expected);
        }
        // a value inside the sequence: its neighbours are adjacent to it
        let x = rbig!(-22 / 7);
        if x.denominator() <= &limit {
            let (down, up) = (x.next_down(&limit), x.next_up(&limit));
            assert!(down < x && x < up);
            assert!(down.denominator() <= &limit && up.denominator() <= &limit);
            assert_eq!(
                up.numerator() * x.denominator() - x.numerator() * up.denominator(),
                IBig::ONE
            );
            assert_eq!(
                x.numerator() * down.denominator() - down.numerator() * x.denominator(),
                IBig::ONE
            );
            assert!(up.denominator() + x.denominator() > limit);
            assert!(down.denominator() + x.denominator() > limit);
            assert_eq!(x.nearest(&limit), Exact(x.clone()));
        }
    }
}
