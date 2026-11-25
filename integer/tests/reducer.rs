use dashu_int::fast_div::ConstDivisor;
use num_modular::Reducer;

mod helper_macros;

#[test]
fn test_reducer_add() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), ubig!(0), ubig!(0)),
        (ubig!(0), ubig!(1), ubig!(1)),
        (ubig!(1), ubig!(2), ubig!(3)),
        (ubig!(1000), ubig!(2000), ubig!(3000)),
    ];

    for (a, b, c) in small_cases {
        assert_eq!(reducer.add(&reducer.transform(a), &reducer.transform(b)), reducer.transform(c));
    }

    let reducer = ConstDivisor::new(ubig!(0x1234567890123456789012345678901234567890));
    let large_cases = [
        (ubig!(0), ubig!(0), ubig!(0)),
        (ubig!(0), ubig!(1), ubig!(1)),
        (ubig!(1), ubig!(2), ubig!(3)),
        (ubig!(0x1000), ubig!(0x2000), ubig!(0x3000)),
        (
            ubig!(0x100000000000000000000000),
            ubig!(0x200000000000000000000000),
            ubig!(0x300000000000000000000000),
        ),
    ];

    for (a, b, c) in large_cases {
        assert_eq!(reducer.add(&reducer.transform(a), &reducer.transform(b)), reducer.transform(c));
    }
}

#[test]
fn test_reducer_dbl() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), ubig!(0)),
        (ubig!(1), ubig!(2)),
        (ubig!(2), ubig!(4)),
        (ubig!(1000), ubig!(2000)),
    ];

    for (a, b) in small_cases {
        assert_eq!(reducer.dbl(reducer.transform(a)), reducer.transform(b));
    }

    let reducer = ConstDivisor::new(ubig!(0x1234567890123456789012345678901234567890));
    let large_cases = [
        (ubig!(0), ubig!(0)),
        (ubig!(1), ubig!(2)),
        (ubig!(2), ubig!(4)),
        (ubig!(0x1000), ubig!(0x2000)),
        (ubig!(0x100000000000000000000000), ubig!(0x200000000000000000000000)),
    ];

    for (a, b) in large_cases {
        assert_eq!(reducer.dbl(reducer.transform(a)), reducer.transform(b));
    }
}

#[test]
fn test_reducer_sub() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), ubig!(0), ubig!(0)),
        (ubig!(0), ubig!(1), ubig!(1233)),
        (ubig!(1), ubig!(0), ubig!(1)),
        (ubig!(1000), ubig!(2000), ubig!(234)),
        (ubig!(2000), ubig!(1000), ubig!(1000)),
    ];

    for (a, b, c) in small_cases {
        assert_eq!(reducer.sub(&reducer.transform(a), &reducer.transform(b)), reducer.transform(c));
    }

    let m = ubig!(0x1234567890123456789012345678901234567890);
    let reducer = ConstDivisor::new(m.clone());
    let large_cases = [
        (ubig!(0), ubig!(0), ubig!(0)),
        (ubig!(0), ubig!(1), &m - ubig!(1)),
        (ubig!(1), ubig!(0), ubig!(1)),
        (ubig!(0x1000), ubig!(0x2000), &m - ubig!(0x1000)),
        (ubig!(0x2000), ubig!(0x1000), ubig!(0x1000)),
        (
            ubig!(0x100000000000000000000000),
            ubig!(0x200000000000000000000000),
            &m - ubig!(0x100000000000000000000000),
        ),
        (
            ubig!(0x200000000000000000000000),
            ubig!(0x100000000000000000000000),
            ubig!(0x100000000000000000000000),
        ),
    ];

    for (a, b, c) in large_cases {
        assert_eq!(reducer.sub(&reducer.transform(a), &reducer.transform(b)), reducer.transform(c));
    }
}

#[test]
fn test_reducer_neg() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), ubig!(0)),
        (ubig!(1), ubig!(1233)),
        (ubig!(2), ubig!(1232)),
        (ubig!(1000), ubig!(234)),
    ];

    for (a, b) in small_cases {
        assert_eq!(reducer.neg(reducer.transform(a)), reducer.transform(b));
    }

    let m = ubig!(0x1234567890123456789012345678901234567890);
    let reducer = ConstDivisor::new(m.clone());
    let large_cases = [
        (ubig!(0), ubig!(0)),
        (ubig!(1), &m - ubig!(1)),
        (ubig!(2), &m - ubig!(2)),
        (ubig!(0x1000), &m - ubig!(0x1000)),
        (ubig!(0x100000000000000000000000), &m - ubig!(0x100000000000000000000000)),
    ];

    for (a, b) in large_cases {
        assert_eq!(reducer.neg(reducer.transform(a)), reducer.transform(b));
    }
}

#[test]
fn test_reducer_mul() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), ubig!(0), ubig!(0)),
        (ubig!(0), ubig!(1), ubig!(0)),
        (ubig!(1), ubig!(1), ubig!(1)),
        (ubig!(1000), ubig!(2000), ubig!(920)),
        (ubig!(2000), ubig!(1000), ubig!(920)),
    ];

    for (a, b, c) in small_cases {
        assert_eq!(reducer.mul(&reducer.transform(a), &reducer.transform(b)), reducer.transform(c));
    }

    let m = ubig!(0x1234567890123456789012345678901234567890);
    let reducer = ConstDivisor::new(m.clone());
    let large_cases = [
        (ubig!(0), ubig!(0), ubig!(0)),
        (ubig!(0), ubig!(1), ubig!(0)),
        (ubig!(1), ubig!(1), ubig!(1)),
        (ubig!(0x1000), ubig!(0x2000), ubig!(0x2000000)),
        (ubig!(0x2000), ubig!(0x1000), ubig!(0x2000000)),
        (
            ubig!(0x100000000000000000000000),
            ubig!(0x200000000000000000000000),
            ubig!(0x12c000000012c000000012c000000012e000000),
        ),
        (
            ubig!(0x200000000000000000000000),
            ubig!(0x100000000000000000000000),
            ubig!(0x12c000000012c000000012c000000012e000000),
        ),
    ];

    for (a, b, c) in large_cases {
        assert_eq!(reducer.mul(&reducer.transform(a), &reducer.transform(b)), reducer.transform(c));
    }
}

#[test]
fn test_reducer_sqr() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), ubig!(0)),
        (ubig!(1), ubig!(1)),
        (ubig!(2), ubig!(4)),
        (ubig!(1000), ubig!(460)),
    ];

    for (a, b) in small_cases {
        assert_eq!(reducer.sqr(reducer.transform(a)), reducer.transform(b));
    }

    let m = ubig!(0x1234567890123456789012345678901234567890);
    let reducer = ConstDivisor::new(m.clone());
    let large_cases = [
        (ubig!(0), ubig!(0)),
        (ubig!(1), ubig!(1)),
        (ubig!(2), ubig!(4)),
        (ubig!(0x1000), ubig!(0x1000000)),
        (
            ubig!(0x100000000000000000000000),
            ubig!(0x96000000009600000000960000000097000000),
        ),
    ];

    for (a, b) in large_cases {
        assert_eq!(reducer.sqr(reducer.transform(a)), reducer.transform(b));
    }
}

#[test]
fn test_reducer_inv() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), None),
        (ubig!(1), Some(ubig!(1))),
        (ubig!(2), None),
        (ubig!(1001), Some(ubig!(1091))),
    ];

    for (a, b) in small_cases {
        assert_eq!(reducer.inv(reducer.transform(a)), b.map(|v| reducer.transform(v)));
    }

    let m = ubig!(0x1234567890123456789012345678901234567890);
    let reducer = ConstDivisor::new(m.clone());
    let large_cases = [
        (ubig!(0), None),
        (ubig!(1), Some(ubig!(1))),
        (ubig!(2), None),
        (ubig!(0x1001), Some(ubig!(94593912811883961533498385659457234825064814993))),
        (
            ubig!(0x100000000000000000000001),
            Some(ubig!(49597318474237639809488304913303841829624579953)),
        ),
    ];

    for (a, b) in large_cases {
        assert_eq!(reducer.inv(reducer.transform(a)), b.map(|v| reducer.transform(v)));
    }
}

#[test]
fn test_reducer_pow() {
    let reducer = ConstDivisor::new(ubig!(1234));
    let small_cases = [
        (ubig!(0), ubig!(0), ubig!(1)),
        (ubig!(0), ubig!(1), ubig!(0)),
        (ubig!(1), ubig!(1), ubig!(1)),
        (ubig!(1000), ubig!(2000), ubig!(968)),
        (ubig!(2000), ubig!(1000), ubig!(446)),
    ];

    for (a, b, c) in small_cases {
        assert_eq!(reducer.pow(reducer.transform(a), &b), reducer.transform(c));
    }

    let m = ubig!(0x1234567890123456789012345678901234567890);
    let reducer = ConstDivisor::new(m.clone());
    let large_cases = [
        (ubig!(0), ubig!(0), ubig!(1)),
        (ubig!(0), ubig!(1), ubig!(0)),
        (ubig!(1), ubig!(1), ubig!(1)),
        (
            ubig!(0x1000),
            ubig!(0x2000),
            ubig!(6836336448053429355926867185433977698732821296),
        ),
        (
            ubig!(0x2000),
            ubig!(0x1000),
            ubig!(40493613063087797729807140685333106341989572736),
        ),
        (
            ubig!(0x100000000000000000000000),
            ubig!(0x200000000000000000000000),
            ubig!(3353173600746198511020157785071195910194367376),
        ),
        (
            ubig!(0x100000000000000000000001),
            ubig!(0x200000000000000000000001),
            ubig!(5183365446386658625038933891731846219325326897),
        ),
    ];

    for (a, b, c) in large_cases {
        assert_eq!(reducer.pow(reducer.transform(a), &b), reducer.transform(c));
    }
}

#[test]
fn test_reducer_sum_equal_to_modulus() {
    use dashu_int::UBig;

    // moduli of one, two and more words (also with 16-bit and 32-bit words)
    let moduli = [
        ubig!(1234),
        ubig!(1) << 16,
        (ubig!(1) << 32) + ubig!(6),
        ubig!(1) << 64,
        (ubig!(1) << 64) + ubig!(4),
        ubig!(1) << 128,
        (ubig!(1) << 128) + ubig!(3),
        (ubig!(1) << 200) - ubig!(1),
        ubig!(3).pow(200),
        ubig!(0x1234567890123456789012345678901234567890),
    ];
    for m in moduli {
        let reducer = ConstDivisor::new(m.clone());
        let zero = reducer.transform(UBig::ZERO);
        assert!(reducer.check(&zero));
        // the modulus itself is not a reduced value
        assert_eq!(reducer.transform(m.clone()), zero);

        let half = &m >> 1;
        let splits = [
            (ubig!(1), &m - ubig!(1)),
            (half.clone(), &m - &half),
            (&m - ubig!(1), ubig!(1)),
        ];
        for (a, b) in splits {
            let (ta, tb) = (reducer.transform(a.clone()), reducer.transform(b.clone()));
            // a + b == m
            let sum = reducer.add(&ta, &tb);
            assert!(reducer.check(&sum), "m = {m}, a = {a}");
            assert!(reducer.is_zero(&sum), "m = {m}, a = {a}");
            assert_eq!(sum, zero, "m = {m}, a = {a}");
            assert_eq!(reducer.residue(sum), UBig::ZERO, "m = {m}, a = {a}");
            // a + (b - 1) == m - 1
            let sum = reducer.add(&ta, &reducer.transform(&b - ubig!(1)));
            assert_eq!(reducer.residue(sum), &m - ubig!(1), "m = {m}, a = {a}");
            // -a == b and a - (m - b) == 0
            assert_eq!(reducer.neg(ta.clone()), tb, "m = {m}, a = {a}");
            assert_eq!(reducer.sub(&ta, &reducer.neg(tb)), zero, "m = {m}, a = {a}");
        }

        if &half << 1 == m {
            // 2 * (m / 2) == m
            let doubled = reducer.dbl(reducer.transform(half.clone()));
            assert!(reducer.is_zero(&doubled), "m = {m}");
            assert_eq!(reducer.residue(doubled), UBig::ZERO, "m = {m}");
        }
        let doubled = reducer.dbl(reducer.transform(&m - ubig!(1)));
        assert_eq!(reducer.residue(doubled), (&m - ubig!(2)) % &m, "m = {m}");
    }
}

#[test]
fn test_reducer_check_rejects_unshifted_values() {
    use dashu_int::{ops::PowerOfTwo, UBig};

    // The reduced values are multiples of transform(1) (a power of two given by the
    // normalization shift of the modulus), so other values are not valid, however small.
    let moduli = [
        (ubig!(1) << 100) + ubig!(1), // two 64-bit words
        (ubig!(1) << 150) + ubig!(1), // three 64-bit words
        (ubig!(1) << 200) + ubig!(1),
        (ubig!(1) << 1000) + ubig!(1),
        ubig!(0x1234567890123456789012345678901234567890),
    ];
    for m in moduli {
        let reducer = ConstDivisor::new(m.clone());
        let unit = reducer.transform(ubig!(1));
        assert!(unit > ubig!(1) && unit.is_power_of_two(), "m = {m}");
        assert!(reducer.check(&unit), "m = {m}");
        assert!(reducer.check(&UBig::ZERO), "m = {m}");
        for v in [
            ubig!(1),
            ubig!(3),
            &unit - ubig!(1),
            &unit + ubig!(1),
            &unit >> 1,
        ] {
            assert!(!reducer.check(&v), "m = {m}, v = {v}");
        }
        for v in [ubig!(3), ubig!(12345), &m - ubig!(1), &m >> 1] {
            let t = reducer.transform(v.clone());
            assert!(reducer.check(&t), "m = {m}, v = {v}");
            assert_eq!(reducer.residue(t), v, "m = {m}, v = {v}");
        }
    }
}
