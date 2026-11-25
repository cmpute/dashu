//! The runtime threshold overrides of the `tuning` feature select between algorithms;
//! no value may make an operation fail or change its result.

#![cfg(feature = "tuning")]

use dashu_int::{
    high::{mul_high, sqr_high},
    UBig, Word,
};

const WORD_BITS: usize = Word::BITS as usize;

/// Every threshold variable read by the multiplication, squaring and short product kernels.
const VARS: [&str; 13] = [
    "DASHU_THRESHOLD_SIMPLE_MUL",
    "DASHU_THRESHOLD_KARATSUBA_MUL",
    "DASHU_THRESHOLD_TOOM42_MIN",
    "DASHU_THRESHOLD_TOOM4_MUL",
    "DASHU_THRESHOLD_NTT_MUL",
    "DASHU_THRESHOLD_NTT_ASYM_MIN",
    "DASHU_THRESHOLD_NTT_ASYM_RATIO",
    "DASHU_THRESHOLD_SIMPLE_SQR",
    "DASHU_THRESHOLD_KARATSUBA_SQR",
    "DASHU_THRESHOLD_TOOM4_SQR",
    "DASHU_THRESHOLD_NTT_SQR",
    "DASHU_THRESHOLD_MULHIGH_SIMPLE",
    "DASHU_THRESHOLD_SQRHIGH_SIMPLE",
];

fn with_env<T>(vars: &[(&str, &str)], f: impl FnOnce() -> T) -> T {
    for (name, value) in vars {
        std::env::set_var(name, value);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    for (name, _) in vars {
        std::env::remove_var(name);
    }
    match result {
        Ok(v) => v,
        Err(e) => {
            let message = e
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap_or_default();
            panic!("panicked with {:?}: {}", vars, message)
        }
    }
}

/// `2^(p words) - 1` and `2^(q words) - 3`
fn operands(p: usize, q: usize) -> (UBig, UBig) {
    (
        (UBig::ONE << (WORD_BITS * p)) - UBig::ONE,
        (UBig::ONE << (WORD_BITS * q)) - UBig::from(3u8),
    )
}

/// `(2^P - 1)(2^Q - 3) = 2^(P+Q) - 3*2^P - 2^Q + 3`, with no multiplication involved.
fn product(p: usize, q: usize) -> UBig {
    (UBig::ONE << (WORD_BITS * (p + q)))
        - (UBig::from(3u8) << (WORD_BITS * p))
        - (UBig::ONE << (WORD_BITS * q))
        + UBig::from(3u8)
}

/// `(2^P - 1)^2 = 2^(2P) - 2^(P+1) + 1`
fn square(p: usize) -> UBig {
    (UBig::ONE << (2 * WORD_BITS * p)) - (UBig::ONE << (WORD_BITS * p + 1)) + UBig::ONE
}

fn check_products(shapes: &[(usize, usize)]) {
    for &(p, q) in shapes {
        let (a, b) = operands(p, q);
        assert_eq!(&a * &b, product(p, q), "{} x {} words", p, q);
        assert_eq!(&b * &a, product(p, q), "{} x {} words", q, p);
    }
}

fn check_squares(lens: &[usize]) {
    for &p in lens {
        let (a, _) = operands(p, 1);
        assert_eq!(a.sqr(), square(p), "{} words squared", p);
        assert_eq!(&a * &a, square(p), "{} words squared", p);
    }
}

/// The documented contract of the short products: the value under-estimates the
/// window by less than `n + 2` units, and the flag tells whether anything was dropped.
fn check_high_products(lens: &[usize]) {
    for &len in lens {
        let a = (UBig::ONE << (WORD_BITS * len)) - UBig::ONE;
        let b = (UBig::ONE << (WORD_BITS * len - 1)) + UBig::from(12345u32);
        for n in (1..=len).filter(|&n| n <= 3 || n + 2 >= len || n == len / 2) {
            for (x, y, (v, sticky)) in [
                (&a, &b, mul_high(&a, &b, n)),
                (&a, &a, mul_high(&a, &a, n)),
                (&a, &a, sqr_high(&a, n)),
                (&b, &b, sqr_high(&b, n)),
            ] {
                let full = x * y;
                let s = (2 * len - n) * WORD_BITS;
                let top = &full >> s;
                assert!(v <= top && top < &v + UBig::from(n + 2), "len = {}, n = {}", len, n);
                assert_eq!(sticky, full != (&top << s), "len = {}, n = {}", len, n);
            }
        }
    }
}

// A single test function: it changes process-wide environment variables.
#[test]
fn test_threshold_overrides_never_fail() {
    // the Toom-4x2 kernel entered below its minimum length
    with_env(&[("DASHU_THRESHOLD_TOOM42_MIN", "10")], || {
        check_products(&[(40, 20), (25, 12)])
    });
    // the Toom-3 kernel entered below its minimum length
    with_env(
        &[
            ("DASHU_THRESHOLD_SIMPLE_MUL", "4"),
            ("DASHU_THRESHOLD_KARATSUBA_MUL", "8"),
        ],
        || check_products(&[(12, 12), (9, 9), (15, 15), (30, 9)]),
    );
    // the Karatsuba kernels entered below their minimum length
    with_env(&[("DASHU_THRESHOLD_SIMPLE_MUL", "0")], || {
        check_products(&[(3, 3), (5, 2), (7, 1)])
    });
    for value in ["0", "1", "2"] {
        with_env(&[("DASHU_THRESHOLD_SIMPLE_SQR", value)], || {
            check_squares(&[2, 3, 4, 5, 6, 7, 9, 16])
        });
    }
    // the schoolbook kernel re-dispatches the tail of a long operand, which needs
    // scratch space once the simple threshold is above the Toom-4x2 minimum length
    with_env(&[("DASHU_THRESHOLD_SIMPLE_MUL", "100000")], || {
        check_products(&[(1924, 500), (1100, 1100), (1500, 300)])
    });
    // the transform kernel below the Toom-4 threshold
    with_env(&[("DASHU_THRESHOLD_NTT_MUL", "100")], || {
        check_products(&[(300, 200), (150, 150)])
    });
    with_env(&[("DASHU_THRESHOLD_NTT_ASYM_RATIO", "18446744073709551615")], || {
        check_products(&[(2600, 2600)])
    });

    // the short products split windows that are too short
    for value in ["0", "1", "2", "3", "5"] {
        with_env(
            &[
                ("DASHU_THRESHOLD_MULHIGH_SIMPLE", value),
                ("DASHU_THRESHOLD_SQRHIGH_SIMPLE", value),
            ],
            || check_high_products(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 17, 33, 40]),
        );
    }

    // every threshold at once, over shapes around the minimum lengths of the kernels
    let lens = [
        1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 40, 63, 64, 65, 80, 130,
    ];
    let mut shapes = Vec::new();
    for (i, &p) in lens.iter().enumerate() {
        for &q in &lens[..=i] {
            shapes.push((p, q));
        }
    }
    for value in [
        "0",
        "1",
        "2",
        "3",
        "8",
        "15",
        "31",
        "32",
        "64",
        "18446744073709551615",
    ] {
        let vars: Vec<(&str, &str)> = VARS.iter().map(|&name| (name, value)).collect();
        with_env(&vars, || {
            check_products(&shapes);
            check_squares(&lens);
            check_high_products(&[3, 9, 17, 40]);
        });
    }
}
