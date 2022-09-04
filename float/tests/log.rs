use dashu_base::Approximation::*;
use dashu_float::round::Rounding::*;

mod helper_macros;

#[test]
fn test_log_binary() {
    assert_eq!(fbig!(1).ln(), fbig!(0));

    let inexact_cases = [
        (fbig!(0x3), fbig!(0x8p-3)),
        (fbig!(0x0003), fbig!(0x8c9fp-15)),
        (fbig!(0x0000000000000003), fbig!(0x8c9f53d5681854bbp-63)),
        (fbig!(0x3000), fbig!(0x96a9p-12)),
        (fbig!(0x3000000000000000), fbig!(0xaabff116fff344b6p-58)),
        (fbig!(0xf), fbig!(0xap-2)),
        (fbig!(0xffff), fbig!(0xb172p-12)),
        (fbig!(0xffffp-16), fbig!(-0x1p-16)),
        (fbig!(0xffffp-32), fbig!(-0xb172p-12)),
        (fbig!(0xffffffffffffffff), fbig!(0xb17217f7d1cf79abp-58)),
        (fbig!(0xffff000000000000p-64), fbig!(-0x800040002aaacaaap-79)),
        (fbig!(0xffff000000000000p-128), fbig!(-0xb1721bf7d3cf7b01p-58)),
        (fbig!(0xffffffffffffffffp-64), fbig!(-0x1p-64)),
        (fbig!(0xf0f0f0f0f0f0f0f0), fbig!(0xb134039651acb45fp-58)),
        (fbig!(0xf0f0f0f0f0f0f0f0p-64), fbig!(-0xf85186008b15331bp-68)),
        (fbig!(0xf0f0f0f0f0f0f0f0p-128), fbig!(-0xb1b02c5951f23ef8p-58)),
        (fbig!(0xf0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0p-128), fbig!(-0xf85186008b15330be64b8b775997899dp-132)),
    ];
    for (x, ln) in &inexact_cases {
        assert_eq!(x.ln(), *ln);
        if let Inexact(v, e) = x.context().ln(x) {
            assert_eq!(v, *ln);
            assert_eq!(e, NoOp);
        } else {
            panic!("the result should be inexact!")
        }
    }
}

#[test]
fn test_log_decimal() {
    assert_eq!(dbig!(1).ln(), dbig!(0));

    let inexact_cases = [
        (dbig!(3), dbig!(1), NoOp),
        (dbig!(0003), dbig!(1099e-3), AddOne),
        (dbig!(0000000000000003), dbig!(1098612288668110e-15), AddOne),
        (dbig!(3000), dbig!(8006e-3), NoOp),
        (dbig!(3000000000000000), dbig!(3563738868357879e-14), NoOp),
        (dbig!(9), dbig!(2), NoOp),
        (dbig!(9999), dbig!(9210e-3), NoOp),
        (dbig!(9999e-4), dbig!(-1e-4), NoOp),
        (dbig!(9999e-8), dbig!(-9210e-3), NoOp),
        (dbig!(9999999999999999), dbig!(3684136148790473e-14), NoOp),
        (dbig!(9999000000000000e-16), dbig!(-1000050003333583e-19), NoOp),
        (dbig!(9999000000000000e-32), dbig!(-3684146149290506e-14), NoOp),
        (dbig!(9999999999999999e-16), dbig!(-1e-16), NoOp),
        (dbig!(9090909090909090), dbig!(3674605130810041e-14), AddOne),
        (dbig!(9090909090909090e-16), dbig!(-9531017980432496e-17), NoOp),
        (dbig!(9090909090909090e-32), dbig!(-3693667166770906e-14), SubOne),
        (dbig!(90909090909090909090909090909090e-32), dbig!(-95310179804324860043952123280775e-33), NoOp),
    ];
    for (x, ln, rnd) in &inexact_cases {
        assert_eq!(x.ln(), *ln);
        if let Inexact(v, e) = x.context().ln(x) {
            assert_eq!(v, *ln);
            assert_eq!(e, *rnd);
        } else {
            panic!("the result should be inexact!")
        }
    }
}
