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
        (ubig!(0x100000000000000000000000), ubig!(0x96000000009600000000960000000097000000)),
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
        (ubig!(0x100000000000000000000001), Some(ubig!(49597318474237639809488304913303841829624579953))),
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
        (ubig!(0x1000), ubig!(0x2000), ubig!(6836336448053429355926867185433977698732821296)),
        (ubig!(0x2000), ubig!(0x1000), ubig!(40493613063087797729807140685333106341989572736)),
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
