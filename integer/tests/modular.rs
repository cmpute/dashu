use dashu_int::fast_div::ConstDivisor;

mod helper_macros;

#[test]
fn test_clone() {
    let ring1 = ConstDivisor::new(ubig!(100));
    let x = ring1.reduce(512);
    let y = x.clone();
    assert_eq!(x, y);
    let mut z = ring1.reduce(513);
    assert_ne!(x, z);
    z.clone_from(&x);
    assert_eq!(x, z);

    let ring2 = ConstDivisor::new(ubig!(1000000000000000000000000000000));
    let x = ring2.reduce(512);
    let y = x.clone();
    assert_eq!(x, y);
    let mut z = ring2.reduce(513);
    assert_ne!(x, z);
    z.clone_from(&x);
    assert_eq!(x, z);

    let mut x = ring1.reduce(512);
    let y = ring2.reduce(1);
    x.clone_from(&y);
    assert_eq!(x, y);

    let ring3 = ConstDivisor::new(ubig!(10).pow(100));
    let x = ring2.reduce(1);
    let mut y = ring3.reduce(2);
    y.clone_from(&x);
    assert_eq!(x, y);
}

#[test]
fn test_convert() {
    let ring = ConstDivisor::new(ubig!(100));
    let x = ring.reduce(6);
    assert_eq!(x, ring.reduce(ubig!(306)));
    assert_ne!(x, ring.reduce(ubig!(313)));
    assert_eq!(x, ring.reduce(ubig!(18297381723918723981723981723906)));
    assert_ne!(x, ring.reduce(ubig!(18297381723918723981723981723913)));
    assert_eq!(x, ring.reduce(ibig!(-18297381723918723981723981723994)));
    assert_eq!(x, ring.reduce(106u8));
    assert_eq!(x, ring.reduce(106u16));
    assert_eq!(x, ring.reduce(1006u32));
    assert_eq!(x, ring.reduce(10000000006u64));
    assert_eq!(x, ring.reduce(1000000000000000000006u128));
    assert_eq!(x, ring.reduce(106usize));
    assert_eq!(x, ring.reduce(6i8));
    assert_eq!(x, ring.reduce(-94i8));
    assert_eq!(x, ring.reduce(-94i16));
    assert_eq!(x, ring.reduce(-94i32));
    assert_eq!(x, ring.reduce(-94i64));
    assert_eq!(x, ring.reduce(-94i128));
    assert_eq!(x, ring.reduce(-94isize));

    assert_eq!(ring.reduce(0), ring.reduce(false));
    assert_eq!(ring.reduce(1), ring.reduce(true));

    let ring =
        ConstDivisor::new(ubig!(_1000000000000000000000000000000000000000000000000000000000000));
    let x = ring.reduce(6);
    let y = ring.reduce(ubig!(333333333333333333333333333333));
    assert_eq!(
        x,
        ring.reduce(ubig!(_1000000000000000000000000000000000000000000000000000000000006))
    );
    assert_ne!(
        x,
        ring.reduce(ubig!(_1000000000000000000000000000000000000000000000000000000000007))
    );
    assert_eq!(
        y,
        ring.reduce(ubig!(_7000000000000000000000000000000333333333333333333333333333333))
    );
}

#[test]
fn test_negate() {
    let ring = ConstDivisor::new(ubig!(100));
    let x = ring.reduce(-1234);
    let y = -&x;
    assert_eq!(y.residue(), ubig!(34));
    let y = -x;
    assert_eq!(y.residue(), ubig!(34));

    let ring = ConstDivisor::new(ubig!(1000000000000000000000000000000));
    let x = ring.reduce(ibig!(-33333123456789012345678901234567890));
    let y = -&x;
    assert_eq!(y, ring.reduce(ubig!(44444123456789012345678901234567890)));
    assert_eq!(y.residue(), ubig!(123456789012345678901234567890));
    let y = -x;
    assert_eq!(y, ring.reduce(ubig!(44444123456789012345678901234567890)));
}

#[test]
#[should_panic]
fn test_cmp_different_rings() {
    let ring1 = ConstDivisor::new(ubig!(100));
    let ring2 = ConstDivisor::new(ubig!(200));
    let x = ring1.reduce(5);
    let y = ring2.reduce(5);
    let _ = x == y;
}

#[test]
fn test_add_sub() {
    let ring1 = ConstDivisor::new(ubig!(100));
    let ring2 = ConstDivisor::new(ubig!(1000000000000000000000000000000));
    let test_cases = [
        (ring1.reduce(1), ring1.reduce(2), ring1.reduce(3)),
        (ring1.reduce(99), ring1.reduce(5), ring1.reduce(4)),
        (ring1.reduce(99), ring1.reduce(99), ring1.reduce(98)),
        (
            ring2.reduce(ubig!(111111111111111111111111111111)),
            ring2.reduce(ubig!(222222222222222223333333333333)),
            ring2.reduce(ubig!(333333333333333334444444444444)),
        ),
        (
            ring2.reduce(ubig!(111111111111111111111111111111)),
            ring2.reduce(ubig!(888888888888888888888888888889)),
            ring2.reduce(ubig!(0)),
        ),
        (
            ring2.reduce(ubig!(999999999999999999999999999999)),
            ring2.reduce(ubig!(999999999999999999999999999997)),
            ring2.reduce(ubig!(999999999999999999999999999996)),
        ),
    ];

    #[allow(clippy::map_identity)]
    let all_test_cases = test_cases
        .iter()
        .map(|(a, b, c)| (a, b, c)) // Need identity map to convert tuple ref to ref tuple
        .chain(test_cases.iter().map(|(a, b, c)| (b, a, c)));

    for (a, b, c) in all_test_cases {
        assert_eq!(a + b, *c);
        assert_eq!(a.clone() + b, *c);
        assert_eq!(a + b.clone(), *c);
        assert_eq!(a.clone() + b.clone(), *c);
        let mut x = a.clone();
        x += b;
        assert_eq!(x, *c);
        let mut x = a.clone();
        x += b.clone();
        assert_eq!(x, *c);

        assert_eq!(c - a, *b);
        assert_eq!(c.clone() - a, *b);
        assert_eq!(c - a.clone(), *b);
        assert_eq!(c.clone() - a.clone(), *b);
        let mut x = c.clone();
        x -= a;
        assert_eq!(x, *b);
        let mut x = c.clone();
        x -= a.clone();
        assert_eq!(x, *b);
    }
}

#[test]
fn test_mul() {
    let ring1 = ConstDivisor::new(ubig!(100));
    let ring2 = ConstDivisor::new(ubig!(1000000000000000000000000000000));
    let big = ubig!(10).pow(10000);
    let ring3 = ConstDivisor::new(big.clone());
    let test_cases = [
        (ring1.reduce(1), ring1.reduce(1), ring1.reduce(1)),
        (ring1.reduce(1), ring1.reduce(99), ring1.reduce(99)),
        (ring1.reduce(99), ring1.reduce(99), ring1.reduce(1)),
        (ring1.reduce(23), ring1.reduce(96), ring1.reduce(8)),
        (ring1.reduce(64), ring1.reduce(64), ring1.reduce(96)),
        (
            ring2.reduce(ubig!(46301564276035228370597101114)),
            ring2.reduce(ubig!(170100953649249045221461413048)),
            ring2.reduce(ubig!(399394418012748758198974935472)),
        ),
        (
            ring2.reduce(ubig!(1208925819614629174706176)),
            ring2.reduce(ubig!(1208925819614629174706176)),
            ring2.reduce(ubig!(203684832716283019655932542976)),
        ),
        (
            ring2.reduce(ubig!(1208925819614629174706175)),
            ring2.reduce(ubig!(1208925819614629174706175)),
            ring2.reduce(ubig!(203682414864643790397583130625)),
        ),
        (ring3.reduce(&big - ubig!(1)), ring3.reduce(&big - ubig!(1)), ring3.reduce(1)),
        (
            ring3.reduce(&big - ubig!(1)),
            ring3.reduce(&big - ubig!(10).pow(10)),
            ring3.reduce(ubig!(10).pow(10)),
        ),
        (
            ring3.reduce(&big - ubig!(10).pow(10)),
            ring3.reduce(&big - ubig!(10).pow(10)),
            ring3.reduce(ubig!(10).pow(20)),
        ),
    ];

    #[allow(clippy::map_identity)]
    let all_test_cases = test_cases
        .iter()
        .map(|(a, b, c)| (a, b, c)) // Need identity map to convert tuple ref to ref tuple
        .chain(test_cases.iter().map(|(a, b, c)| (b, a, c)));

    for (a, b, c) in all_test_cases {
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
}

#[test]
fn test_inv() {
    // small ring
    let ring = ConstDivisor::new(ubig!(1));
    assert_eq!(ring.reduce(0).inv(), Some(ring.reduce(0)));

    let ring = ConstDivisor::new(ubig!(100));
    let x = ring.reduce(9);
    let y = x.clone().inv().unwrap();
    assert_eq!((x * y).residue(), ubig!(1));

    let x = ring.reduce(0);
    assert!(x.inv().is_none());
    let x = ring.reduce(10);
    assert!(x.inv().is_none());

    let ring = ConstDivisor::new(ubig!(103));
    let x = ring.reduce(20);
    let y = x.inv().unwrap();
    assert_eq!(y.residue(), ubig!(67)); // inverse is unique for prime modulus

    // medium ring
    let ring = ConstDivisor::new(ubig!(1000000000000000000000000000000));
    let x = ring.reduce(ibig!(3333312345678901234567890123456789));
    let y = x.clone().inv().unwrap();
    assert_eq!((x * y).residue(), ubig!(1));

    let x = ring.reduce(0);
    assert!(x.inv().is_none());
    let x = ring.reduce(10);
    assert!(x.inv().is_none());

    let ring = ConstDivisor::new(ubig!(1000000000000000000000000000057)); // prime
    let x = ring.reduce(123456789);
    let y = x.inv().unwrap();
    assert_eq!(y.residue(), ubig!(951144331155413413514262063034));

    // large ring
    let ring = ConstDivisor::new(ubig!(
        0x100000000000000000000000000000000000000000000000000000000000000000000000000000000
    ));
    let x = ring.reduce(123456789);
    let y = x.inv().unwrap();
    assert_eq!(
        y.residue(),
        ubig!(502183094104378158094730467601915490123618665365443345649182408561985048745994978946725109832253)
    );

    let x = ring.reduce(0);
    assert!(x.inv().is_none());
    let x = ring.reduce(10);
    assert!(x.inv().is_none());

    let x = ring.reduce(ubig!(0x123456789123456789123456789));
    let y = x.inv().unwrap();
    assert_eq!(
        y.residue(),
        ubig!(1654687843822646720169408413229830444089197976699429504340681760590766246761104608701978442022585)
    );
    let x = ring.reduce(ubig!(0x123456789123456789123456788));
    assert!(x.inv().is_none());

    let x = ring.reduce(ubig!(0x123456789123456789123456789123456789123456789123456789));
    let y = x.inv().unwrap();
    let x = ring.reduce(ubig!(0x123456789123456789123456789123456789123456789000000000));
    assert!(x.inv().is_none());
    assert_eq!(
        y.residue(),
        ubig!(77064304169441121490325922823072327980740992335161695976803567323815961864721792027154186059449)
    );
}

#[test]
fn test_div() {
    let ring = ConstDivisor::new(ubig!(10));
    // 3 * 4 == 2 mod 10
    let a = ring.reduce(2);
    let b = ring.reduce(3);
    let res = ring.reduce(4);
    assert_eq!(a.clone() / b.clone(), res);
    assert_eq!(a.clone() / &b, res);
    assert_eq!(&a / b.clone(), res);
    assert_eq!(&a / &b, res);

    let mut a = ring.reduce(2);
    a /= b.clone();
    assert_eq!(a, res);

    let mut a = ring.reduce(2);
    a /= &b;
    assert_eq!(a, res);
}

#[test]
#[should_panic]
fn test_add_different_rings() {
    let ring1 = ConstDivisor::new(ubig!(100));
    let ring2 = ConstDivisor::new(ubig!(200));
    let x = ring1.reduce(5);
    let y = ring2.reduce(5);
    let _ = x + y;
}

#[test]
#[should_panic]
fn test_sub_different_rings() {
    let ring1 = ConstDivisor::new(ubig!(100));
    let ring2 = ConstDivisor::new(ubig!(200));
    let x = ring1.reduce(5);
    let y = ring2.reduce(5);
    let _ = x - y;
}
#[test]
#[should_panic]
fn test_div_different_rings() {
    let ring1 = ConstDivisor::new(ubig!(100));
    let ring2 = ConstDivisor::new(ubig!(200));
    let x = ring1.reduce(1);
    let y = ring2.reduce(1);
    let _ = x / y;
}

#[test]
#[should_panic]
fn test_div_by_noninvertible() {
    let ring = ConstDivisor::new(ubig!(100));
    let x = ring.reduce(10);
    let y = ring.reduce(2);
    let _ = x / y;
}

#[test]
fn test_pow() {
    let ring = ConstDivisor::new(ubig!(100));
    assert_eq!(ring.reduce(0).pow(&ubig!(0)), ring.reduce(1));
    assert_eq!(ring.reduce(13).pow(&ubig!(0)), ring.reduce(1));
    assert_eq!(ring.reduce(13).pow(&ubig!(1)), ring.reduce(13));
    assert_eq!(ring.reduce(13).pow(&ubig!(2)), ring.reduce(69));
    assert_eq!(ring.reduce(13).pow(&ubig!(12837918273)), ring.reduce(53));
    assert_eq!(
        ring.reduce(13)
            .pow(&((ubig!(1) << 10000) * ubig!(40) + ubig!(3))),
        ring.reduce(97)
    );

    let ring = ConstDivisor::new(ubig!(1000000000000000000000000000000));
    let x = ring.reduce(ubig!(658571505947767552546868380533));
    assert_eq!(x.pow(&ubig!(0)), ring.reduce(1));
    assert_eq!(x.pow(&ubig!(1)), x);
    assert_eq!(
        x.pow(&ubig!(794990856522773482558337459018)),
        ring.reduce(ubig!(660533815789733011052086421209))
    );

    // A Mersenne prime.
    let prime = ubig!(2).pow(4423) - ubig!(1);
    let ring = ConstDivisor::new(prime.clone());
    // Fermat theorem: a^(p-1) = 1
    assert_eq!(ring.reduce(13).pow(&(prime - ubig!(1))), ring.reduce(1));
}

#[test]
fn test_format() {
    let ring = ConstDivisor::new(ubig!(100));
    let x = ring.reduce(105);
    assert_eq!(format!("{}", ring), "100");
    assert_eq!(format!("{}", x), "5 (mod 100)");
    assert_eq!(format!("{:=^5}", x), "==5== (mod =100=)");
    assert_eq!(format!("{:b}", x), "101 (mod 1100100)");
    assert_eq!(format!("{:o}", x), "5 (mod 144)");
    assert_eq!(format!("{:#x}", x), "0x5 (mod 0x64)");
    assert_eq!(format!("{:X}", x), "5 (mod 64)");
    assert_eq!(format!("{:?}", x), "5 (mod 100)");
    assert_eq!(
        format!("{:#?}", x),
        r#"Reduced {
    residue: 5 (digits: 1, bits: 3),
    modulus: 100 (digits: 3, bits: 7),
}"#
    );

    // 1000000000000000000000000000000000000000 has 130 bits
    let ring = ConstDivisor::new(ubig!(1000000000000000000000000000000000000000));
    let x = -ring.reduce(1);
    assert_eq!(format!("{}", ring), "1000000000000000000000000000000000000000");
    assert_eq!(
        format!("{:45}", x),
        "      999999999999999999999999999999999999999 (mod      1000000000000000000000000000000000000000)"
    );
    assert_eq!(format!("{:b}", x),
        "1011110000010100001111111010010011100010010100001110101100110001000101111101100101010101100111111111111111111111111111111111111111 (mod 1011110000010100001111111010010011100010010100001110101100110001000101111101100101010101101000000000000000000000000000000000000000)");
    assert_eq!(
        format!("{:#o}", x),
        "0o13602417722342241654610575452547777777777777 (mod 0o13602417722342241654610575452550000000000000)"
    );
    assert_eq!(
        format!("{:x}", x),
        "2f050fe938943acc45f65567fffffffff (mod 2f050fe938943acc45f65568000000000)"
    );
    assert_eq!(
        format!("{:X}", x),
        "2F050FE938943ACC45F65567FFFFFFFFF (mod 2F050FE938943ACC45F65568000000000)"
    );

    if dashu_int::Word::BITS == 64 {
        assert_eq!(
            format!("{:?}", x),
            "9999999999999999999..9999999999999999999 (mod 1000000000000000000..0000000000000000000)"
        );
        assert_eq!(
            format!("{:#?}", x),
            r#"Reduced {
    residue: 9999999999999999999..9999999999999999999 (digits: 39, bits: 130),
    modulus: 1000000000000000000..0000000000000000000 (digits: 40, bits: 130),
}"#
        );
    }
}
