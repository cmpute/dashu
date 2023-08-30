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
