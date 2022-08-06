use dashu_int::modular::ModuloRing;

mod helper_macros;

#[test]
fn test_modulus() {
    let ring = ModuloRing::new(ubig!(100));
    assert_eq!(ring.modulus(), ubig!(100));

    let ring = ModuloRing::new(ubig!(10).pow(100));
    assert_eq!(ring.modulus(), ubig!(10).pow(100));
}

#[test]
fn test_clone() {
    let ring1 = ModuloRing::new(ubig!(100));
    let x = ring1.convert(512);
    let y = x.clone();
    assert_eq!(x, y);
    let mut z = ring1.convert(513);
    assert_ne!(x, z);
    z.clone_from(&x);
    assert_eq!(x, z);

    let ring2 = ModuloRing::new(ubig!(1000000000000000000000000000000));
    let x = ring2.convert(512);
    let y = x.clone();
    assert_eq!(x, y);
    let mut z = ring2.convert(513);
    assert_ne!(x, z);
    z.clone_from(&x);
    assert_eq!(x, z);

    let mut x = ring1.convert(512);
    let y = ring2.convert(1);
    x.clone_from(&y);
    assert_eq!(x, y);

    let ring3 = ModuloRing::new(ubig!(10).pow(100));
    let x = ring2.convert(1);
    let mut y = ring3.convert(2);
    y.clone_from(&x);
    assert_eq!(x, y);
}

#[test]
fn test_convert() {
    let ring = ModuloRing::new(ubig!(100));
    let x = ring.convert(6);
    assert_eq!(x, ring.convert(&ubig!(306)));
    assert_ne!(x, ring.convert(&ubig!(313)));
    assert_eq!(x, ring.convert(&ubig!(18297381723918723981723981723906)));
    assert_ne!(x, ring.convert(&ubig!(18297381723918723981723981723913)));
    assert_eq!(x, ring.convert(ubig!(18297381723918723981723981723906)));
    assert_eq!(x, ring.convert(ibig!(18297381723918723981723981723906)));
    assert_eq!(x, ring.convert(ibig!(-18297381723918723981723981723994)));
    assert_eq!(x, ring.convert(&ibig!(-18297381723918723981723981723994)));
    assert_eq!(x, ring.convert(106u8));
    assert_eq!(x, ring.convert(106u16));
    assert_eq!(x, ring.convert(1006u32));
    assert_eq!(x, ring.convert(10000000006u64));
    assert_eq!(x, ring.convert(1000000000000000000006u128));
    assert_eq!(x, ring.convert(106usize));
    assert_eq!(x, ring.convert(6i8));
    assert_eq!(x, ring.convert(-94i8));
    assert_eq!(x, ring.convert(-94i16));
    assert_eq!(x, ring.convert(-94i32));
    assert_eq!(x, ring.convert(-94i64));
    assert_eq!(x, ring.convert(-94i128));
    assert_eq!(x, ring.convert(-94isize));

    assert_eq!(ring.convert(0), ring.convert(false));
    assert_eq!(ring.convert(1), ring.convert(true));

    let ring =
        ModuloRing::new(ubig!(_1000000000000000000000000000000000000000000000000000000000000));
    let x = ring.convert(6);
    let y = ring.convert(ubig!(333333333333333333333333333333));
    assert_eq!(
        x,
        ring.convert(ubig!(_1000000000000000000000000000000000000000000000000000000000006))
    );
    assert_eq!(
        x,
        ring.convert(&ubig!(_1000000000000000000000000000000000000000000000000000000000006))
    );
    assert_ne!(
        x,
        ring.convert(ubig!(_1000000000000000000000000000000000000000000000000000000000007))
    );
    assert_eq!(
        y,
        ring.convert(ubig!(_7000000000000000000000000000000333333333333333333333333333333))
    );
}

#[test]
fn test_negate() {
    let ring = ModuloRing::new(ubig!(100));
    let x = ring.convert(-1234);
    let y = -&x;
    assert_eq!(y.residue(), ubig!(34));
    let y = -x;
    assert_eq!(y.residue(), ubig!(34));

    let ring = ModuloRing::new(ubig!(1000000000000000000000000000000));
    let x = ring.convert(ibig!(-33333123456789012345678901234567890));
    let y = -&x;
    assert_eq!(y, ring.convert(ubig!(44444123456789012345678901234567890)));
    assert_eq!(y.residue(), ubig!(123456789012345678901234567890));
    let y = -x;
    assert_eq!(y, ring.convert(ubig!(44444123456789012345678901234567890)));
}

#[test]
#[allow(clippy::eq_op)]
fn test_different_rings() {
    let ring1 = ModuloRing::new(ubig!(100));
    let ring2 = ModuloRing::new(ubig!(100));
    assert_eq!(ring1, ring1);
    assert_ne!(ring1, ring2);
}

#[test]
#[should_panic]
fn test_cmp_different_rings() {
    let ring1 = ModuloRing::new(ubig!(100));
    let ring2 = ModuloRing::new(ubig!(200));
    let x = ring1.convert(5);
    let y = ring2.convert(5);
    let _ = x == y;
}

#[test]
fn test_add_sub() {
    let ring1 = ModuloRing::new(ubig!(100));
    let ring2 = ModuloRing::new(ubig!(1000000000000000000000000000000));
    let test_cases = [
        (ring1.convert(1), ring1.convert(2), ring1.convert(3)),
        (ring1.convert(99), ring1.convert(5), ring1.convert(4)),
        (ring1.convert(99), ring1.convert(99), ring1.convert(98)),
        (
            ring2.convert(ubig!(111111111111111111111111111111)),
            ring2.convert(ubig!(222222222222222223333333333333)),
            ring2.convert(ubig!(333333333333333334444444444444)),
        ),
        (
            ring2.convert(ubig!(111111111111111111111111111111)),
            ring2.convert(ubig!(888888888888888888888888888889)),
            ring2.convert(ubig!(0)),
        ),
        (
            ring2.convert(ubig!(999999999999999999999999999999)),
            ring2.convert(ubig!(999999999999999999999999999997)),
            ring2.convert(ubig!(999999999999999999999999999996)),
        ),
    ];

    let all_test_cases = test_cases
        .iter()
        .map(|(a, b, c)| (a, b, c))
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
    let ring1 = ModuloRing::new(ubig!(100));
    let ring2 = ModuloRing::new(ubig!(1000000000000000000000000000000));
    let big = ubig!(10).pow(10000);
    let ring3 = ModuloRing::new(big.clone());
    let test_cases = [
        (ring1.convert(1), ring1.convert(1), ring1.convert(1)),
        (ring1.convert(1), ring1.convert(99), ring1.convert(99)),
        (ring1.convert(99), ring1.convert(99), ring1.convert(1)),
        (ring1.convert(23), ring1.convert(96), ring1.convert(8)),
        (ring1.convert(64), ring1.convert(64), ring1.convert(96)),
        (
            ring2.convert(ubig!(46301564276035228370597101114)),
            ring2.convert(ubig!(170100953649249045221461413048)),
            ring2.convert(ubig!(399394418012748758198974935472)),
        ),
        (
            ring2.convert(ubig!(1208925819614629174706176)),
            ring2.convert(ubig!(1208925819614629174706176)),
            ring2.convert(ubig!(203684832716283019655932542976)),
        ),
        (
            ring2.convert(ubig!(1208925819614629174706175)),
            ring2.convert(ubig!(1208925819614629174706175)),
            ring2.convert(ubig!(203682414864643790397583130625)),
        ),
        (ring3.convert(&big - ubig!(1)), ring3.convert(&big - ubig!(1)), ring3.convert(1)),
        (ring3.convert(&big - ubig!(1)), ring3.convert(&big - ubig!(10).pow(10)), ring3.convert(ubig!(10).pow(10))),
        (ring3.convert(&big - ubig!(10).pow(10)), ring3.convert(&big - ubig!(10).pow(10)), ring3.convert(ubig!(10).pow(20))),
    ];

    let all_test_cases = test_cases
        .iter()
        .map(|(a, b, c)| (a, b, c))
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
    let ring = ModuloRing::new(ubig!(100));
    let x = ring.convert(9);
    let y = x.clone().inv().unwrap();
    assert_eq!((x * y).residue(), ubig!(1));

    let x = ring.convert(0);
    assert!(x.inv().is_none());
    let x = ring.convert(10);
    assert!(x.inv().is_none());

    let ring = ModuloRing::new(ubig!(103));
    let x = ring.convert(20);
    let y = x.inv().unwrap();
    assert_eq!(y.residue(), ubig!(67)); // inverse is unique for prime modulus

    // medium ring
    let ring = ModuloRing::new(ubig!(1000000000000000000000000000000));
    let x = ring.convert(ibig!(3333312345678901234567890123456789));
    let y = x.clone().inv().unwrap();
    assert_eq!((x * y).residue(), ubig!(1));

    let x = ring.convert(0);
    assert!(x.inv().is_none());
    let x = ring.convert(10);
    assert!(x.inv().is_none());

    let ring = ModuloRing::new(ubig!(1000000000000000000000000000057)); // prime
    let x = ring.convert(123456789);
    let y = x.inv().unwrap();
    assert_eq!(y.residue(), ubig!(951144331155413413514262063034));

    // large ring
    let ring = ModuloRing::new(ubig!(
        0x100000000000000000000000000000000000000000000000000000000000000000000000000000000
    ));
    let x = ring.convert(123456789);
    let y = x.inv().unwrap();
    assert_eq!(
        y.residue(),
        ubig!(502183094104378158094730467601915490123618665365443345649182408561985048745994978946725109832253)
    );

    let x = ring.convert(0);
    assert!(x.inv().is_none());
    let x = ring.convert(10);
    assert!(x.inv().is_none());

    let x = ring.convert(ubig!(0x123456789123456789123456789));
    let y = x.inv().unwrap();
    assert_eq!(
        y.residue(),
        ubig!(1654687843822646720169408413229830444089197976699429504340681760590766246761104608701978442022585)
    );
    let x = ring.convert(ubig!(0x123456789123456789123456788));
    assert!(x.inv().is_none());

    let x = ring.convert(ubig!(
        0x123456789123456789123456789123456789123456789123456789
    ));
    let y = x.inv().unwrap();
    let x = ring.convert(ubig!(
        0x123456789123456789123456789123456789123456789000000000
    ));
    assert!(x.inv().is_none());
    assert_eq!(
        y.residue(),
        ubig!(77064304169441121490325922823072327980740992335161695976803567323815961864721792027154186059449)
    );
}

#[test]
#[should_panic]
fn test_add_different_rings() {
    let ring1 = ModuloRing::new(ubig!(100));
    let ring2 = ModuloRing::new(ubig!(200));
    let x = ring1.convert(5);
    let y = ring2.convert(5);
    let _ = x + y;
}

#[test]
#[should_panic]
fn test_sub_different_rings() {
    let ring1 = ModuloRing::new(ubig!(100));
    let ring2 = ModuloRing::new(ubig!(200));
    let x = ring1.convert(5);
    let y = ring2.convert(5);
    let _ = x - y;
}

#[test]
fn test_pow() {
    let ring = ModuloRing::new(ubig!(100));
    assert_eq!(ring.convert(0).pow(&ubig!(0)), ring.convert(1));
    assert_eq!(ring.convert(13).pow(&ubig!(0)), ring.convert(1));
    assert_eq!(ring.convert(13).pow(&ubig!(1)), ring.convert(13));
    assert_eq!(ring.convert(13).pow(&ubig!(2)), ring.convert(69));
    assert_eq!(ring.convert(13).pow(&ubig!(12837918273)), ring.convert(53));
    assert_eq!(
        ring.convert(13)
            .pow(&((ubig!(1) << 10000) * ubig!(40) + ubig!(3))),
        ring.convert(97)
    );

    let ring = ModuloRing::new(ubig!(1000000000000000000000000000000));
    let x = ring.convert(ubig!(658571505947767552546868380533));
    assert_eq!(x.pow(&ubig!(0)), ring.convert(1));
    assert_eq!(x.pow(&ubig!(1)), x);
    assert_eq!(
        x.pow(&ubig!(794990856522773482558337459018)),
        ring.convert(ubig!(660533815789733011052086421209))
    );

    // A Mersenne prime.
    let prime = ubig!(2).pow(4423) - ubig!(1);
    let ring = ModuloRing::new(prime.clone());
    // Fermat theorem: a^(p-1) = 1
    assert_eq!(ring.convert(13).pow(&(prime - ubig!(1))), ring.convert(1));
}

#[test]
fn test_format() {
    let ring = ModuloRing::new(ubig!(100));
    let x = ring.convert(105);
    assert_eq!(format!("{}", ring), "mod 100");
    assert_eq!(format!("{}", x), "5 (mod 100)");
    assert_eq!(format!("{:?}", x), "5 (mod 100)");
    assert_eq!(format!("{:=^5}", x), "==5== (mod =100=)");
    assert_eq!(format!("{:b}", x), "101 (mod 1100100)");
    assert_eq!(format!("{:o}", x), "5 (mod 144)");
    assert_eq!(format!("{:#x}", x), "0x5 (mod 0x64)");
    assert_eq!(format!("{:X}", x), "5 (mod 64)");

    let ring = ModuloRing::new(ubig!(1000000000000000000000000000000));
    let x = -ring.convert(1);
    assert_eq!(format!("{}", ring), "mod 1000000000000000000000000000000");
    assert_eq!(
        format!("{:?}", x),
        "999999999999999999999999999999 (mod 1000000000000000000000000000000)"
    );
    assert_eq!(
        format!("{:35}", x),
        "     999999999999999999999999999999 (mod     1000000000000000000000000000000)"
    );
    assert_eq!(format!("{:b}", x),
        "1100100111110010110010011100110100000100011001110100111011011110101000111111111111111111111111111111 (mod 1100100111110010110010011100110100000100011001110100111011011110101001000000000000000000000000000000)");
    assert_eq!(
        format!("{:#o}", x),
        "0o1447626234640431647336507777777777 (mod 0o1447626234640431647336510000000000)"
    );
    assert_eq!(format!("{:x}", x), "c9f2c9cd04674edea3fffffff (mod c9f2c9cd04674edea40000000)");
    assert_eq!(format!("{:X}", x), "C9F2C9CD04674EDEA3FFFFFFF (mod C9F2C9CD04674EDEA40000000)");
}
