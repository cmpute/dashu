use core::{
    fmt::Debug,
    ops::{Mul, MulAssign},
};

mod helper_macros;

fn test_mul<'a, T>(a: &'a T, b: &'a T, c: &'a T)
where
    T: Mul<T, Output = T>,
    T: Mul<&'a T, Output = T>,
    &'a T: Mul<T, Output = T>,
    &'a T: Mul<&'a T, Output = T>,
    T: MulAssign<T>,
    T: MulAssign<&'a T>,
    T: Clone,
    T: Debug,
    T: Eq,
{
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

#[test]
fn test_mul_ubig() {
    let test_cases = [
        (ubig!(0), ubig!(4), ubig!(0)),
        (ubig!(0), ubig!(1) << 100, ubig!(0)),
        (ubig!(3), ubig!(4), ubig!(12)),
        (
            ubig!(0x123456789abc),
            ubig!(0x444333222111fff),
            ubig!(0x4daae4d8531f8de7e1fb5ae544),
        ),
        (
            ubig!(1),
            ubig!(0x123456789123456789123456789123456789),
            ubig!(0x123456789123456789123456789123456789),
        ),
        (
            ubig!(0x10),
            ubig!(0x123456789123456789123456789123456789),
            ubig!(0x1234567891234567891234567891234567890),
        ),
        (
            ubig!(0x1000000000000000),
            ubig!(0x123456789123456789123456789123456789),
            ubig!(0x123456789123456789123456789123456789000000000000000),
        ),
        (
            ubig!(0x123456789123456789123456789123456789123456789123456789),
            ubig!(0xabcdefabcdefabcdefabcdefabcdef),
            ubig!(0xc379ab6dbd40ef67e528bfffd3039491348e20491348e20491348d5ccf67db24c3a1cca8f7891375de7),
        ),
        (ubig!(5), ubig!(1) << 50, ubig!(5) << 50),
        (ubig!(5), ubig!(1) << 100, ubig!(5) << 100),
    ];

    for (a, b, c) in &test_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);
    }
}

#[test]
fn test_mul_ibig() {
    let test_cases = [
        (ibig!(3), ibig!(4), ibig!(12)),
        (ibig!(-3), ibig!(4), ibig!(-12)),
        (ibig!(-3), ibig!(-4), ibig!(12)),
    ];

    for (a, b, c) in &test_cases {
        test_mul(a, b, c);
        test_mul(b, a, c);
    }
}

#[test]
#[allow(clippy::op_ref, clippy::erasing_op)]
fn test_mul_ubig_primitive() {
    assert_eq!(ubig!(3) * 4u8, ubig!(12));
    assert_eq!(ubig!(3) * &4u8, ubig!(12));
    assert_eq!(&ubig!(3) * 4u8, ubig!(12));
    assert_eq!(&ubig!(3) * &4u8, ubig!(12));
    assert_eq!(4u8 * ubig!(3), ubig!(12));
    assert_eq!(4u8 * &ubig!(3), ubig!(12));
    assert_eq!(&4u8 * ubig!(3), ubig!(12));
    assert_eq!(&4u8 * &ubig!(3), ubig!(12));
    let mut x = ubig!(3);
    x *= 2u8;
    x *= &2u8;
    assert_eq!(x, ubig!(12));
}

#[test]
#[allow(clippy::op_ref)]
fn test_mul_ibig_primitive() {
    assert_eq!(ibig!(-3) * -4, ibig!(12));
    assert_eq!(ibig!(-3) * &-4, ibig!(12));
    assert_eq!(&ibig!(-3) * -4, ibig!(12));
    assert_eq!(&ibig!(-3) * &-4, ibig!(12));
    assert_eq!(-4 * ibig!(-3), ibig!(12));
    assert_eq!(-4 * &ibig!(-3), ibig!(12));
    assert_eq!(&-4 * ibig!(-3), ibig!(12));
    assert_eq!(&-4 * &ibig!(-3), ibig!(12));
    let mut x = ibig!(-3);
    x *= 2;
    x *= &-2;
    assert_eq!(x, ibig!(12));
}

#[test]
fn test_mul_ubig_ibig() {
    let test_cases = [
        (ubig!(0), ibig!(4), ibig!(0)),
        (ubig!(0), ibig!(1) << 100, ibig!(0)),
        (ubig!(4), ibig!(0), ibig!(0)),
        (ubig!(1) << 100, ibig!(0), ibig!(0)),
        (ubig!(3), ibig!(4), ibig!(12)),
        (ubig!(3), ibig!(-4), ibig!(-12)),
        (
            ubig!(0x123456789abc),
            ibig!(0x444333222111fff),
            ibig!(0x4daae4d8531f8de7e1fb5ae544),
        ),
        (
            ubig!(0x123456789abc),
            ibig!(-0x444333222111fff),
            ibig!(-0x4daae4d8531f8de7e1fb5ae544),
        ),
        (
            ubig!(1),
            ibig!(-0x123456789123456789123456789123456789),
            ibig!(-0x123456789123456789123456789123456789),
        ),
        (
            ubig!(0x10),
            ibig!(-0x123456789123456789123456789123456789),
            ibig!(-0x1234567891234567891234567891234567890),
        ),
        (
            ubig!(0x1000000000000000),
            ibig!(-0x123456789123456789123456789123456789),
            ibig!(-0x123456789123456789123456789123456789000000000000000),
        ),
        (
            ubig!(0x123456789123456789123456789123456789123456789123456789),
            ibig!(-0xabcdefabcdefabcdefabcdefabcdef),
            ibig!(-0xc379ab6dbd40ef67e528bfffd3039491348e20491348e20491348d5ccf67db24c3a1cca8f7891375de7),
        ),
        (ubig!(5), ibig!(-1) << 50, ibig!(-5) << 50),
        (ubig!(5), ibig!(-1) << 100, ibig!(-5) << 100),
    ];

    for (a, b, c) in &test_cases {
        assert_eq!(a * b, *c);
        assert_eq!(a.clone() * b, *c);
        assert_eq!(a * b.clone(), *c);
        assert_eq!(a.clone() * b.clone(), *c);

        let mut x = b.clone();
        x *= a;
        assert_eq!(x, *c);

        let mut x = b.clone();
        x *= a.clone();
        assert_eq!(x, *c);
    }
}

#[test]
fn test_sqr() {
    let test_cases = [
        (ubig!(0), ubig!(0)),
        (ubig!(1), ubig!(1)),
        (ubig!(10), ubig!(100)),
        (ubig!(1) << 16, ubig!(1) << 32),
        ((ubig!(1) << 64) - ubig!(1), (ubig!(1) << 128) - (ubig!(1) << 65) + ubig!(1)),
        ((ubig!(1) << 128) - ubig!(2), (ubig!(1) << 256) - (ubig!(1) << 130) + ubig!(4)),
        ((ubig!(1) << 128) - ubig!(2), (ubig!(1) << 256) - (ubig!(1) << 130) + ubig!(4)),
    ];

    for (a, b) in test_cases {
        assert_eq!(a.square(), b);
        assert_eq!((-a).square(), b);
    }

    // 3^[25, 50, 100, 200, 400, 800]
    let pow3 = [
        ubig!(847288609443),
        ubig!(717897987691852588770249),
        ubig!(515377520732011331036461129765621272702107522001),
        ubig!(265613988875874769338781322035779626829233452653394495974574961739092490901302182994384699044001),
        ubig!(70550791086553325712464271575934796216507949612787315762871223209262085551582934156579298529447134158154952334825355911866929793071824566694145084454535257027960285323760313192443283334088001),
        ubig!(4977414122938492192881464029729961679802517669640314331069754317413863193300588672960378941038799444233797200629740876278809425638436874294137213623651683084623545115805694417048191856898335577690331770093271154442020977681305435856437590481321498962517248672813060123683011804992094505499691756946329466238029256908317387659245893361869285485179777099016847012698558309358412176001),
    ];
    for ab in pow3.windows(2) {
        let a = ab.first().unwrap();
        let b = ab.last().unwrap();
        assert_eq!(&a.square(), b);
        assert_eq!(&(-a).square(), b);
    }
}
