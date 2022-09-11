use dashu_int::ops::{ExtendedGcd, Gcd};

mod helper_macros;

#[test]
fn test_gcd_ubig_ibig() {
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
            ubig!(0x123456789123456789123456789123456789),
            ubig!(0x123456789123456789123456789123456789),
        ),
        (ubig!(1), ubig!(0x123456789123456789123456789123456789), ubig!(1)),
        (
            ubig!(0x123456789123456789123456789123456789),
            ubig!(0x123456789123456789123456789123456789),
            ubig!(0x123456789123456789123456789123456789),
        ),
        (
            ubig!(0),
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000009d),
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000009d),
        ),
        (
            ubig!(1),
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000009d),
            ubig!(1),
        ),
        // small cases
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
            ubig!(0x3ffffffffffffffffffffffffffffffffffffd), // largest prime under 2^150
            ubig!(0x1fffffffffffffffffffffffffffffffffffe1), // largest prime under 2^149
            ubig!(1),
        ),
        (ubig!(0x123456789123456789123456789123456789), ubig!(0x987654321), ubig!(0x2d)),
        (
            ubig!(0x123456789123456789123456789123456789),
            ubig!(0x987654321987654321987654321987654321),
            ubig!(0x2d00000002d00000002d00000002d),
        ),
        (
            ubig!(0x5a4653ca673768565b41f775d6947d55cf3813d1), // 3^100
            ubig!(0x1000000000000000000000000000000000000000),
            ubig!(1),
        ),
        (
            ubig!(0x1000000000000000000000000000000000000000000000000eb), // first prime after 2^200
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000009d), // first prime after 2^300
            ubig!(1),
        ),
        (
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000009d),
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000b5), // first prime after 2^400
            ubig!(1),
        ),
        (
            ubig!(0x123456789),
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000009d),
            ubig!(1),
        ),
        (
            ubig!(0x123456789123456789123456789),
            ubig!(0x100000000000000000000000000000000000000000000000000000000000000000000000009d),
            ubig!(1),
        ),
        (
            ubig!(9541590761809372806823445181494772614224189786781),
            ubig!(269120040206265862655074245332316974136638590557129744496105952360956998867985564496114298920939436079650169510482532190861609609644663884343086721587551983677544514295),
            ubig!(1),
        ),
        (
            ubig!(82374668589351327716850961655220860777198309362585356767551962516307253429467315703959898802658586372875264763319007374387473758850822495865554449187711436126),
            ubig!(26670195456596011221424694237548782549589802807766865293275505988717182112618046143878494843215456811172970603344701167285123673971424775466198384682101501381319877471305282410925936647623427837324275758273098197674268400910393213728144245761180650830272102839801127045093737937200818),
            ubig!(6),
        ),
        // random cases
        (ubig!(2032), ubig!(494444716490368), ubig!(16)),
        (ubig!(188), ubig!(614678047460453929180024679), ubig!(47)),
        (ubig!(22725712718635), ubig!(2072275484441518576), ubig!(37)),
    ];

    for (a, b, c) in &test_cases {
        let ia = -a.clone();
        let ib = -b.clone();

        assert_eq!(&a.gcd(b), c);
        assert_eq!(&a.gcd(b.clone()), c);
        assert_eq!(&b.gcd(a), c);
        assert_eq!(&b.clone().gcd(a), c);
        assert_eq!(&(&ia).gcd(&ib), c);
        assert_eq!(&(&ia).gcd(ib.clone()), c);
        assert_eq!(&(&ib).gcd(&ia), c);
        assert_eq!(&(ib.clone()).gcd(&ia), c);

        let (g, x, y) = a.gcd_ext(b);
        assert_eq!(&g, c);
        assert_eq!(x * a + y * b, g);
        let (g, y, x) = b.gcd_ext(a);
        assert_eq!(&g, c);
        assert_eq!(x * a + y * b, g);

        let (g, x, y) = (&ia).gcd_ext(&ib);
        assert_eq!(&g, c);
        assert_eq!(x * &ia + y * &ib, g);
        let (g, y, x) = (&ib).gcd_ext(&ia);
        assert_eq!(&g, c);
        assert_eq!(x * &ia + y * &ib, g);
    }
}

#[test]
#[should_panic]
fn test_gcd_0() {
    let _ = ubig!(0).gcd(ubig!(0));
}

#[test]
#[should_panic]
fn test_gcd_ext_0() {
    let _ = ubig!(0).gcd_ext(ubig!(0));
}
