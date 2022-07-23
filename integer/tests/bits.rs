use dashu_int::{ibig, ops::PowerOfTwo, ubig, IBig};

#[test]
#[allow(clippy::bool_assert_comparison)]
fn test_bit() {
    assert_eq!(ubig!(0).bit(0), false);
    assert_eq!(ubig!(0).bit(1000), false);
    assert_eq!(ubig!(0b11101).bit(0), true);
    assert_eq!(ubig!(0b11101).bit(1), false);
    assert_eq!(ubig!(0b11101).bit(4), true);
    assert_eq!(ubig!(0b11101).bit(5), false);
    assert_eq!(ubig!(0b11101).bit(1000), false);

    assert_eq!(ubig!(0xffffffffffffffffffffffffffffffff).bit(127), true);
    assert_eq!(ubig!(0xffffffffffffffffffffffffffffffff).bit(128), false);
    assert_eq!(ubig!(0xffffffffffffffffffffffffffffffffffff).bit(143), true); // 2 ^ 144 - 1
    assert_eq!(
        ubig!(0xffffffffffffffffffffffffffffffffffff).bit(144),
        false
    );
}

#[test]
fn test_set_bit() {
    let mut a = ubig!(0);
    a.set_bit(3);
    assert_eq!(a, ubig!(0b1000));
    a.set_bit(129);
    assert_eq!(a, ubig!(0x200000000000000000000000000000008));
    a.set_bit(1);
    assert_eq!(a, ubig!(0x20000000000000000000000000000000a));
    a.set_bit(1);
    assert_eq!(a, ubig!(0x20000000000000000000000000000000a));
    a.set_bit(127);
    assert_eq!(a, ubig!(0x28000000000000000000000000000000a));
    a.set_bit(194);
    assert_eq!(
        a,
        ubig!(0x400000000000000028000000000000000000000000000000a)
    );
}

#[test]
fn test_clear_bit() {
    let mut a = ubig!(0x400000000000000028000000000000000000000000000000a);
    a.clear_bit(10000);
    assert_eq!(
        a,
        ubig!(0x400000000000000028000000000000000000000000000000a)
    );
    a.clear_bit(194);
    assert_eq!(a, ubig!(0x28000000000000000000000000000000a));
    a.clear_bit(1);
    assert_eq!(a, ubig!(0x280000000000000000000000000000008));
    a.clear_bit(129);
    assert_eq!(a, ubig!(0x80000000000000000000000000000008));
    a.clear_bit(127);
    assert_eq!(a, ubig!(0b1000));
    a.clear_bit(3);
    assert_eq!(a, ubig!(0));
}

#[test]
fn test_trailing_zeros() {
    assert_eq!(ubig!(0).trailing_zeros(), None);
    assert_eq!(ubig!(0xf0000).trailing_zeros(), Some(16));
    assert_eq!(
        ubig!(0xfffffffffffffffffffff00000000000000000000000000000000000000000000000000)
            .trailing_zeros(),
        Some(200)
    );

    assert_eq!(ibig!(0).trailing_zeros(), None);
    assert_eq!(ibig!(0xf0000).trailing_zeros(), Some(16));
    assert_eq!(ibig!(-0xf0000).trailing_zeros(), Some(16));
}

#[test]
fn test_bit_len() {
    assert_eq!(ubig!(0).bit_len(), 0);
    assert_eq!(ubig!(0xf0000).bit_len(), 20);
    assert_eq!(
        ubig!(0xfffffffffffffffffffff00000000000000000000000000000000000000000000000000).bit_len(),
        284
    );
}

#[test]
#[allow(clippy::bool_assert_comparison)]
fn test_is_power_of_two() {
    assert_eq!(ubig!(0).is_power_of_two(), false);
    assert_eq!(ubig!(1).is_power_of_two(), true);
    assert_eq!(ubig!(16).is_power_of_two(), true);
    assert_eq!(ubig!(17).is_power_of_two(), false);
    assert_eq!(
        ubig!(0x4000000000000000000000000000000000000000000000).is_power_of_two(),
        true
    );
    assert_eq!(
        ubig!(0x5000000000000000000000000000000000000000000000).is_power_of_two(),
        false
    );
    assert_eq!(
        ubig!(0x4000000000000000000000010000000000000000000000).is_power_of_two(),
        false
    );
}

#[test]
fn test_next_power_of_two() {
    assert_eq!(ubig!(0).next_power_of_two(), ubig!(1));
    assert_eq!(ubig!(16).next_power_of_two(), ubig!(16));
    assert_eq!(ubig!(17).next_power_of_two(), ubig!(32));
    assert_eq!(ubig!(0xffffffff).next_power_of_two(), ubig!(0x100000000));
    assert_eq!(
        ubig!(0xffffffffffffffff).next_power_of_two(),
        ubig!(0x10000000000000000)
    );
    assert_eq!(
        ubig!(0xffffffffffffffffffffffffffffffff).next_power_of_two(),
        ubig!(0x100000000000000000000000000000000)
    );
    assert_eq!(
        ubig!(0xf0000000000000000000000000000000).next_power_of_two(),
        ubig!(0x100000000000000000000000000000000)
    );
    assert_eq!(
        ubig!(0xffffffffffffffff0000000000000000).next_power_of_two(),
        ubig!(0x100000000000000000000000000000000)
    );
    assert_eq!(
        ubig!(0xffffffffffffffff0000000000000000).next_power_of_two(),
        ubig!(0x100000000000000000000000000000000)
    );
    assert_eq!(
        ubig!(0x100000000000000000000000000000000).next_power_of_two(),
        ubig!(0x100000000000000000000000000000000)
    );
    assert_eq!(
        ubig!(0x100000000000000000000000000000001).next_power_of_two(),
        ubig!(0x200000000000000000000000000000000)
    );
    assert_eq!(
        ubig!(0x100100000000000000000000000000000).next_power_of_two(),
        ubig!(0x200000000000000000000000000000000)
    );
}

#[test]
fn test_and_ubig() {
    let cases = [
        (ubig!(0xf0f0), ubig!(0xff00), ubig!(0xf000)),
        (
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xff),
            ubig!(0xee),
        ),
        (
            ubig!(0xff),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xee),
        ),
        (
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd),
            ubig!(0xcccccccccccccccccccccccccccccccc),
        ),
        (
            ubig!(0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xcccccccccccccccccccccccccccccccc),
        ),
    ];

    for (a, b, c) in cases.iter() {
        assert_eq!(a & b, *c);
        assert_eq!(a.clone() & b, *c);
        assert_eq!(a & b.clone(), *c);
        assert_eq!(a.clone() & b.clone(), *c);

        {
            let mut a1 = a.clone();
            a1 &= b;
            assert_eq!(a1, *c);
        }
        {
            let mut a1 = a.clone();
            a1 &= b.clone();
            assert_eq!(a1, *c);
        }
    }
}

#[test]
fn test_or_ubig() {
    let cases = [
        (ubig!(0xf0f0), ubig!(0xff00), ubig!(0xfff0)),
        (
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xff),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeff),
        ),
        (
            ubig!(0xff),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeff),
        ),
        (
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd),
            ubig!(0xddddddddddddddddddddddddddddddddffffffffffffffffffffffffffffffff),
        ),
        (
            ubig!(0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xddddddddddddddddddddddddddddddddffffffffffffffffffffffffffffffff),
        ),
    ];

    for (a, b, c) in cases.iter() {
        assert_eq!(a | b, *c);
        assert_eq!(a.clone() | b, *c);
        assert_eq!(a | b.clone(), *c);
        assert_eq!(a.clone() | b.clone(), *c);

        {
            let mut a1 = a.clone();
            a1 |= b;
            assert_eq!(a1, *c);
        }
        {
            let mut a1 = a.clone();
            a1 |= b.clone();
            assert_eq!(a1, *c);
        }
    }
}

#[test]
fn test_xor_ubig() {
    let cases = [
        (ubig!(0xf0f0), ubig!(0xff00), ubig!(0xff0)),
        (
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xff),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeee11),
        ),
        (
            ubig!(0xff),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeee11),
        ),
        (
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd),
            ubig!(0xdddddddddddddddddddddddddddddddd33333333333333333333333333333333),
        ),
        (
            ubig!(0xdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd),
            ubig!(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee),
            ubig!(0xdddddddddddddddddddddddddddddddd33333333333333333333333333333333),
        ),
    ];

    for (a, b, c) in cases.iter() {
        assert_eq!(a ^ b, *c);
        assert_eq!(a.clone() ^ b, *c);
        assert_eq!(a ^ b.clone(), *c);
        assert_eq!(a.clone() ^ b.clone(), *c);

        {
            let mut a1 = a.clone();
            a1 ^= b;
            assert_eq!(a1, *c);
        }
        {
            let mut a1 = a.clone();
            a1 ^= b.clone();
            assert_eq!(a1, *c);
        }
    }
}

#[test]
fn test_not_ibig() {
    for a in -20i8..=20i8 {
        let a_big: IBig = a.into();
        let res: IBig = (!a).into();

        assert_eq!(!&a_big, res);
        assert_eq!(!a_big, res);
    }
}

#[test]
fn test_and_ibig() {
    for a in -20i8..=20i8 {
        for b in -20i8..=20i8 {
            let a_big: IBig = a.into();
            let b_big: IBig = b.into();
            let res: IBig = (a & b).into();

            assert_eq!(&a_big & &b_big, res);
            assert_eq!(&a_big & b_big.clone(), res);
            assert_eq!(a_big.clone() & &b_big, res);
            assert_eq!(a_big.clone() & b_big.clone(), res);

            let mut x = a_big.clone();
            x &= &b_big;
            assert_eq!(x, res);

            let mut x = a_big.clone();
            x &= b_big.clone();
            assert_eq!(x, res);
        }
    }
}

#[test]
fn test_or_ibig() {
    for a in -20i8..=20i8 {
        for b in -20i8..=20i8 {
            let a_big: IBig = a.into();
            let b_big: IBig = b.into();
            let res: IBig = (a | b).into();

            assert_eq!(&a_big | &b_big, res);
            assert_eq!(&a_big | b_big.clone(), res);
            assert_eq!(a_big.clone() | &b_big, res);
            assert_eq!(a_big.clone() | b_big.clone(), res);

            let mut x = a_big.clone();
            x |= &b_big;
            assert_eq!(x, res);

            let mut x = a_big.clone();
            x |= b_big.clone();
            assert_eq!(x, res);
        }
    }
}

#[test]
fn test_xor_ibig() {
    for a in -20i8..=20i8 {
        for b in -20i8..=20i8 {
            let a_big: IBig = a.into();
            let b_big: IBig = b.into();
            let res: IBig = (a ^ b).into();

            assert_eq!(&a_big ^ &b_big, res);
            assert_eq!(&a_big ^ b_big.clone(), res);
            assert_eq!(a_big.clone() ^ &b_big, res);
            assert_eq!(a_big.clone() ^ b_big.clone(), res);

            let mut x = a_big.clone();
            x ^= &b_big;
            assert_eq!(x, res);

            let mut x = a_big.clone();
            x ^= b_big.clone();
            assert_eq!(x, res);
        }
    }
}

#[test]
#[allow(clippy::identity_op, clippy::op_ref)]
fn test_bit_ops_ubig_unsigned() {
    assert_eq!(ubig!(0xf0f) & 0xffu8, 0xfu8);
    assert_eq!(ubig!(0xf0f) & &0xffu8, 0xfu8);
    assert_eq!(&ubig!(0xf0f) & 0xffu8, 0xfu8);
    assert_eq!(&ubig!(0xf0f) & &0xffu8, 0xfu8);

    assert_eq!(0xffu8 & ubig!(0xf0f), 0xfu8);
    assert_eq!(0xffu8 & &ubig!(0xf0f), 0xfu8);
    assert_eq!(&0xffu8 & ubig!(0xf0f), 0xfu8);
    assert_eq!(&0xffu8 & &ubig!(0xf0f), 0xfu8);

    let mut x = ubig!(0xf0f);
    x &= 0xffu8;
    assert_eq!(x, ubig!(0xf));

    let mut x = ubig!(0xf0f);
    x &= &0xffu8;
    assert_eq!(x, ubig!(0xf));

    assert_eq!(ubig!(0xf0f) | 0xffu8, ubig!(0xfff));
    assert_eq!(ubig!(0xf0f) | &0xffu8, ubig!(0xfff));
    assert_eq!((&ubig!(0xf0f)) | 0xffu8, ubig!(0xfff));
    assert_eq!((&ubig!(0xf0f)) | &0xffu8, ubig!(0xfff));

    assert_eq!(0xffu8 | ubig!(0xf0f), ubig!(0xfff));
    assert_eq!(0xffu8 | &ubig!(0xf0f), ubig!(0xfff));
    assert_eq!(&0xffu8 | ubig!(0xf0f), ubig!(0xfff));
    assert_eq!(&0xffu8 | &ubig!(0xf0f), ubig!(0xfff));

    let mut x = ubig!(0xf0f);
    x |= 0xffu8;
    assert_eq!(x, ubig!(0xfff));

    let mut x = ubig!(0xf0f);
    x |= &0xffu8;
    assert_eq!(x, ubig!(0xfff));

    assert_eq!(ubig!(0xf0f) ^ 0xffu8, ubig!(0xff0));
    assert_eq!(ubig!(0xf0f) ^ &0xffu8, ubig!(0xff0));
    assert_eq!(&ubig!(0xf0f) ^ 0xffu8, ubig!(0xff0));
    assert_eq!(&ubig!(0xf0f) ^ &0xffu8, ubig!(0xff0));

    assert_eq!(0xffu8 ^ ubig!(0xf0f), ubig!(0xff0));
    assert_eq!(0xffu8 ^ &ubig!(0xf0f), ubig!(0xff0));
    assert_eq!(&0xffu8 ^ ubig!(0xf0f), ubig!(0xff0));
    assert_eq!(&0xffu8 ^ &ubig!(0xf0f), ubig!(0xff0));

    let mut x = ubig!(0xf0f);
    x ^= 0xffu8;
    assert_eq!(x, ubig!(0xff0));

    let mut x = ubig!(0xf0f);
    x ^= &0xffu8;
    assert_eq!(x, ubig!(0xff0));
}

#[test]
#[allow(clippy::identity_op, clippy::op_ref)]
fn test_bit_ops_ibig_primitive() {
    assert_eq!(ibig!(0xf0f) & 0xff, ibig!(0xf));
    assert_eq!(ibig!(0xf0f) & &0xff, ibig!(0xf));
    assert_eq!(&ibig!(0xf0f) & 0xff, ibig!(0xf));
    assert_eq!(&ibig!(0xf0f) & &0xff, ibig!(0xf));
    assert_eq!(ibig!(-1) & -1, ibig!(-1));

    let mut x = ibig!(0xf0f);
    x &= 0xff;
    assert_eq!(x, ibig!(0xf));

    let mut x = ibig!(0xf0f);
    x &= &0xff;
    assert_eq!(x, ibig!(0xf));

    assert_eq!(ibig!(0xf0f) | 0xff, ibig!(0xfff));
    assert_eq!(ibig!(0xf0f) | &0xff, ibig!(0xfff));
    assert_eq!((&ibig!(0xf0f)) | 0xff, ibig!(0xfff));
    assert_eq!((&ibig!(0xf0f)) | &0xff, ibig!(0xfff));

    assert_eq!(0xff | ibig!(0xf0f), ibig!(0xfff));
    assert_eq!(0xff | &ibig!(0xf0f), ibig!(0xfff));
    assert_eq!(&0xff | ibig!(0xf0f), ibig!(0xfff));
    assert_eq!(&0xff | &ibig!(0xf0f), ibig!(0xfff));

    assert_eq!(ibig!(17) | -1, ibig!(-1));

    let mut x = ibig!(0xf0f);
    x |= 0xff;
    assert_eq!(x, ibig!(0xfff));

    let mut x = ibig!(0xf0f);
    x |= &0xff;
    assert_eq!(x, ibig!(0xfff));

    assert_eq!(ibig!(0xf0f) ^ 0xff, ibig!(0xff0));
    assert_eq!(ibig!(0xf0f) ^ &0xff, ibig!(0xff0));
    assert_eq!(&ibig!(0xf0f) ^ 0xff, ibig!(0xff0));
    assert_eq!(&ibig!(0xf0f) ^ &0xff, ibig!(0xff0));

    assert_eq!(ibig!(-1) ^ -1, ibig!(0));

    let mut x = ibig!(0xf0f);
    x ^= 0xff;
    assert_eq!(x, ibig!(0xff0));

    let mut x = ibig!(0xf0f);
    x ^= &0xff;
    assert_eq!(x, ibig!(0xff0));
}
