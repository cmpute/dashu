use dashu_int::{ubig, IBig};

#[test]
fn test_gcd_ubig() {
    // test cases (x, y, gcd(x,y))
    let test_cases = [
        // trivial cases
        (ubig!(0), ubig!(123), ubig!(123)),
        (ubig!(123), ubig!(0), ubig!(123)),
        (ubig!(1), ubig!(123), ubig!(1)),
        (ubig!(123), ubig!(1), ubig!(1)),
        (ubig!(123), ubig!(123), ubig!(123)),
        (
            ubig!(0),
            ubig!(_0x123456789123456789123456789123456789),
            ubig!(_0x123456789123456789123456789123456789),
        ),
        (
            ubig!(1),
            ubig!(_0x123456789123456789123456789123456789),
            ubig!(1),
        ),
        (
            ubig!(_0x123456789123456789123456789123456789),
            ubig!(_0x123456789123456789123456789123456789),
            ubig!(_0x123456789123456789123456789123456789),
        ),
        // Small cases
        (ubig!(3), ubig!(7), ubig!(1)),
        (ubig!(8), ubig!(9), ubig!(1)),
        (ubig!(9), ubig!(8), ubig!(1)),
        (ubig!(42), ubig!(56), ubig!(14)),
        (ubig!(7966496), ubig!(314080416), ubig!(32)),
        // big cases
        (
            ubig!(0xffffffffffffffffffffffff1), // largest prime under 2^100
            ubig!(0x7ffffffffffffffffffffff8d), // largest prime under 2^99
            ubig!(1),
        ),
        (
            ubig!(0xffffffffffffffffffffffffffffff61), // largest prime under 2^128
            ubig!(0xffffffffffffffffffffffffffffff53), // second largest prime under 2^128
            ubig!(1),
        ),
        (
            ubig!(_0x3ffffffffffffffffffffffffffffffffffffd), // largest prime under 2^150
            ubig!(_0x1fffffffffffffffffffffffffffffffffffe1), // largest prime under 2^149
            ubig!(1),
        ),
        (
            ubig!(_0x123456789123456789123456789123456789),
            ubig!(_0x987654321),
            ubig!(_0x2d),
        ),
        (
            ubig!(_0x123456789123456789123456789123456789),
            ubig!(_0x987654321987654321987654321987654321),
            ubig!(_0x2d00000002d00000002d00000002d),
        ),
        (
            ubig!(_0x5a4653ca673768565b41f775d6947d55cf3813d1), // 3^100
            ubig!(_0x1000000000000000000000000000000000000000),
            ubig!(1),
        ),
    ];

    for (a, b, c) in &test_cases {
        assert_eq!(&a.gcd(b), c);
        assert_eq!(&b.gcd(a), c);

        let (g, x, y) = a.extended_gcd(b);
        assert_eq!(&g, c);
        assert_eq!(
            x * IBig::from(a.clone()) + y * IBig::from(b.clone()),
            IBig::from(g)
        );
        let (g, y, x) = b.extended_gcd(a);
        assert_eq!(&g, c);
        assert_eq!(
            x * IBig::from(a.clone()) + y * IBig::from(b.clone()),
            IBig::from(g)
        );
    }
}

#[test]
#[should_panic]
fn test_gcd_0() {
    let _ = ubig!(0).gcd(&ubig!(0));
    let _ = ubig!(0).extended_gcd(&ubig!(0));
}
