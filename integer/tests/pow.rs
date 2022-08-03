mod helper_macros;

#[test]
fn test_pow_ubig() {
    let test_cases = [
        // trivial cases
        (ubig!(0), 0, ubig!(1)),
        (ubig!(100), 0, ubig!(1)),
        (ubig!(0), 1, ubig!(0)),
        (ubig!(100), 1, ubig!(100)),
        (ubig!(0), 2, ubig!(0)),
        (ubig!(100), 2, ubig!(10000)),
        (ubig!(0), 100, ubig!(0)),
        (ubig!(1), 100, ubig!(1)),
        // pow by shifting
        (ubig!(2), 10, ubig!(1) << 10),
        (ubig!(64), 10, ubig!(1) << 60),
        (ubig!(2), 100, ubig!(1) << 100),
        (ubig!(64), 100, ubig!(1) << 600),
        // small bases
        (ubig!(7), 10, ubig!(282475249)),
        (ubig!(14), 10, ubig!(282475249) << 10),
        (
            ubig!(7),
            100,
            ubig!(3234476509624757991344647769100216810857203198904625400933895331391691459636928060001),
        ),
        (
            ubig!(14),
            100,
            ubig!(3234476509624757991344647769100216810857203198904625400933895331391691459636928060001)
                << 100,
        ),
        (ubig!(123), 13, ubig!(1474913153392179474539944683)),
        (
            ubig!(123),
            123,
            ubig!(114374367934617190099880295228066276746218078451850229775887975052369504785666896446606568365201542169649974727730628842345343196581134895919942820874449837212099476648958359023796078549041949007807220625356526926729664064846685758382803707100766740220839267),
        ),
        (
            (ubig!(1) << 70) - 1u8,
            10,
            ubig!(0xffffffffffffffffd80000000000000002cfffffffffffffffe20000000000000000d1fffffffffffffffc10000000000000000d1fffffffffffffffe200000000000000002cffffffffffffffffd800000000000000001),
        ),
        // large bases
        (
            (ubig!(1) << 250) - 1u8,
            3,
            (ubig!(1) << 750) - (ubig!(3) << 500) + (ubig!(3) << 250) - 1u8,
        ),
    ];

    for (a, b, c) in &test_cases {
        assert_eq!(a.pow(*b), *c);
    }
}

#[test]
fn test_pow_ibig() {
    let test_cases = [
        (ibig!(0), 0, ibig!(1)),
        (ibig!(0), 12, ibig!(0)),
        (ibig!(0), 13, ibig!(0)),
        (ibig!(7), 2, ibig!(49)),
        (ibig!(7), 3, ibig!(343)),
        (ibig!(-7), 2, ibig!(49)),
        (ibig!(-7), 3, ibig!(-343)),
    ];

    for (a, b, c) in &test_cases {
        assert_eq!(a.pow(*b), *c);
    }
}
