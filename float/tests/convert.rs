use core::convert::TryFrom;
use dashu_base::{Approximation::*, ConversionError::*};
use dashu_float::{
    round::{
        mode::{HalfAway, Zero},
        Rounding::*,
    },
    DBig, FBig,
};

mod helper_macros;

type FBin = FBig;
type FHex = FBig<Zero, 16>;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_base_change() {
    // binary -> decimal
    // 5 decimal digits precision < 20 binary digits precision
    assert_eq!(fbig!(0x12345).with_rounding::<HalfAway>().to_decimal(), Exact(dbig!(74565)));
    assert_eq!(
        fbig!(-0x12345p1).with_rounding::<HalfAway>().to_decimal(),
        Exact(dbig!(-149130))
    );
    assert_eq!(
        fbig!(0x12345p100).with_rounding::<HalfAway>().to_decimal(),
        Inexact(dbig!(945224e29), AddOne)
    );
    assert_eq!(
        fbig!(0x12345p-1).with_rounding::<HalfAway>().to_decimal(),
        Exact(dbig!(372825e-1))
    );
    assert_eq!(
        fbig!(-0x12345p-100)
            .with_rounding::<HalfAway>()
            .to_decimal(),
        Inexact(dbig!(-588214e-31), NoOp)
    );
    assert_eq!(FBig::<HalfAway, 2>::INFINITY.to_decimal(), Inexact(DBig::INFINITY, NoOp));
    assert_eq!(
        FBig::<HalfAway, 2>::NEG_INFINITY.to_decimal(),
        Inexact(DBig::NEG_INFINITY, NoOp)
    );

    assert_eq!(
        fbig!(0x12345)
            .with_rounding::<HalfAway>()
            .with_base_and_precision::<10>(10),
        Exact(dbig!(74565))
    );
    assert_eq!(
        fbig!(-0x12345p1)
            .with_rounding::<HalfAway>()
            .with_base_and_precision::<10>(10),
        Exact(dbig!(-149130))
    );
    assert_eq!(
        fbig!(0x12345p100)
            .with_rounding::<HalfAway>()
            .with_base_and_precision::<10>(10),
        Inexact(dbig!(9452236701e25), AddOne)
    );
    assert_eq!(
        fbig!(0x12345p-1)
            .with_rounding::<HalfAway>()
            .with_base_and_precision::<10>(10),
        Exact(dbig!(372825e-1))
    );
    assert_eq!(
        fbig!(-0x12345p-100)
            .with_rounding::<HalfAway>()
            .with_base_and_precision::<10>(10),
        Inexact(dbig!(-5882141340e-35), SubOne)
    );

    // decimal -> binary
    // 16 binary digits precision < 5 decimal digits precision
    assert_eq!(dbig!(12345).with_rounding::<Zero>().to_binary(), Exact(fbig!(0x3039)));
    assert_eq!(dbig!(-12345e1).with_rounding::<Zero>().to_binary(), Exact(fbig!(-0xf11dp1)));
    assert_eq!(
        dbig!(12345e100).with_rounding::<Zero>().to_binary(),
        Inexact(fbig!(0xdc78p330), NoOp)
    );
    assert_eq!(dbig!(12345e-1).with_rounding::<Zero>().to_binary(), Exact(fbig!(0x9a5p-1)));
    assert_eq!(
        dbig!(-12345e-100).with_rounding::<Zero>().to_binary(),
        Inexact(fbig!(-0xa8c2p-334), NoOp)
    );
    assert_eq!(DBig::INFINITY.to_binary(), Inexact(FBig::INFINITY, NoOp));
    assert_eq!(DBig::NEG_INFINITY.to_binary(), Inexact(FBig::NEG_INFINITY, NoOp));

    assert_eq!(
        dbig!(12345)
            .with_rounding::<Zero>()
            .with_base_and_precision::<2>(30),
        Exact(fbig!(0x3039))
    );
    assert_eq!(
        dbig!(-12345e1)
            .with_rounding::<Zero>()
            .with_base_and_precision::<2>(30),
        Exact(fbig!(-0xf11dp1))
    );
    assert_eq!(
        dbig!(12345e100)
            .with_rounding::<Zero>()
            .with_base_and_precision::<2>(30),
        Inexact(fbig!(0x371e2de9p316), NoOp)
    );
    assert_eq!(
        dbig!(12345e-1)
            .with_rounding::<Zero>()
            .with_base_and_precision::<2>(30),
        Exact(fbig!(0x9a5p-1))
    );
    assert_eq!(
        dbig!(-12345e-100)
            .with_rounding::<Zero>()
            .with_base_and_precision::<2>(30),
        Inexact(fbig!(-0x2a30a4e2p-348), NoOp)
    );

    // binary -> hexadecimal
    assert_eq!(fbig!(0x12345).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x12345), 0)));
    assert_eq!(fbig!(-0x12345p1).with_base::<16>(), Exact(FHex::from_parts(ibig!(-0x2468a), 0)));
    assert_eq!(
        fbig!(0x12345p100).with_base::<16>(),
        Exact(FHex::from_parts(ibig!(0x12345), 25))
    );
    assert_eq!(
        fbig!(0x54321p111).with_base::<16>(),
        Inexact(FHex::from_parts(ibig!(0x2a190), 28), NoOp)
    );
    assert_eq!(fbig!(0x12345p-1).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x91a28), -1)));
    assert_eq!(
        fbig!(-0x12345p-100).with_base::<16>(),
        Exact(FHex::from_parts(ibig!(-0x12345), -25))
    );
    assert_eq!(
        fbig!(-0x12345p-111).with_base::<16>(),
        Exact(FHex::from_parts(ibig!(-0x2468a), -28))
    );
    assert_eq!(FBig::<Zero, 2>::INFINITY.with_base::<16>(), Inexact(FHex::INFINITY, NoOp));
    assert_eq!(
        FBig::<Zero, 2>::NEG_INFINITY.with_base::<16>(),
        Inexact(FHex::NEG_INFINITY, NoOp)
    );

    // hexadecimal -> binary
    assert_eq!(FHex::from_parts(ibig!(0x12345), 0).to_binary(), Exact(fbig!(0x12345)));
    assert_eq!(FHex::from_parts(ibig!(-0x12345), 1).to_binary(), Exact(fbig!(-0x12345p4)));
    assert_eq!(FHex::from_parts(ibig!(0x12345), 100).to_binary(), Exact(fbig!(0x12345p400)));
    assert_eq!(FHex::from_parts(ibig!(0x12345), -1).to_binary(), Exact(fbig!(0x12345p-4)));
    assert_eq!(FHex::from_parts(ibig!(-0x12345), 100).to_binary(), Exact(fbig!(-0x12345p400)));
    assert_eq!(FHex::INFINITY.to_binary(), Inexact(FBig::<Zero, 2>::INFINITY, NoOp));
    assert_eq!(FHex::NEG_INFINITY.to_binary(), Inexact(FBig::<Zero, 2>::NEG_INFINITY, NoOp));
}

#[test]
#[should_panic]
fn test_base_change_unlimited_precision() {
    let _ = dbig!(1234e-1).with_precision(0).unwrap().with_base::<2>();
}

#[test]
fn test_precision_change() {
    assert_eq!(FBin::ZERO.with_precision(1), Exact(FBin::ZERO));
    assert_eq!(FBin::ZERO.with_precision(1).unwrap().precision(), 1);

    assert_eq!(fbig!(0x1234).precision(), 16);
    assert_eq!(fbig!(0x1234).with_precision(0), Exact(fbig!(0x1234)));
    assert_eq!(fbig!(0x1234).with_precision(13), Exact(fbig!(0x1234)));
    assert_eq!(fbig!(0x1234).with_precision(8), Inexact(fbig!(0x91p5), NoOp));
    assert_eq!(fbig!(0x1234).with_precision(4), Inexact(fbig!(0x9p9), NoOp));

    assert_eq!(DBig::ONE.with_precision(1), Exact(DBig::ONE));
    assert_eq!(DBig::ONE.with_precision(1).unwrap().precision(), 1);

    assert_eq!(dbig!(1234).precision(), 4);
    assert_eq!(dbig!(1234).with_precision(0), Exact(dbig!(1234)));
    assert_eq!(dbig!(1234).with_precision(4), Exact(dbig!(1234)));
    assert_eq!(dbig!(1234).with_precision(2), Inexact(dbig!(12e2), NoOp));
}

#[test]
fn test_from_unsigned() {
    assert_eq!(FBin::from(0u8), FBin::ZERO);
    assert_eq!(FBin::from(1u8), FBin::ONE);
    assert_eq!(FBin::from(0x10000u32), FBin::from_parts(ibig!(0x10000), 0));
    assert_eq!(FBin::from(0xffffffffu32), FBin::from_parts(ibig!(0xffffffff), 0));
}

#[test]
fn test_to_unsigned() {
    assert_eq!(u8::try_from(FBin::ZERO), Ok(0u8));
    assert_eq!(u8::try_from(FBin::ONE), Ok(1u8));
    assert_eq!(u8::try_from(FBin::ONE << 1), Ok(2u8));
    assert_eq!(u8::try_from(FBin::ONE >> 1), Err(LossOfPrecision));
    assert_eq!(u8::try_from(FBin::NEG_ONE), Err(OutOfBounds));
    assert_eq!(u8::try_from(FBin::from_parts(u8::MAX.into(), 0)), Ok(u8::MAX));
    assert_eq!(u8::try_from(FBin::ONE << 8), Err(OutOfBounds));
    assert_eq!(u128::try_from(FBin::from_parts(u128::MAX.into(), 0)), Ok(u128::MAX));
    assert_eq!(u128::try_from(FBin::ONE << 128), Err(OutOfBounds));
    assert_eq!(u8::try_from(FBin::INFINITY), Err(OutOfBounds));
    assert_eq!(u128::try_from(FBin::NEG_INFINITY), Err(OutOfBounds));
}

#[test]
fn test_from_signed() {
    assert_eq!(FBin::from(0i8), FBin::ZERO);
    assert_eq!(FBin::from(1i8), FBin::ONE);
    assert_eq!(FBin::from(-1i8), FBin::NEG_ONE);
    assert_eq!(FBin::from(-0x10000i32), FBin::from_parts(ibig!(-0x10000), 0));
    assert_eq!(FBin::from(i32::MIN), FBin::from_parts(ibig!(-0x80000000), 0));
}

#[test]
fn test_to_signed() {
    assert_eq!(i8::try_from(FBin::ZERO), Ok(0i8));
    assert_eq!(i8::try_from(FBin::ONE), Ok(1i8));
    assert_eq!(i8::try_from(FBin::NEG_ONE << 1), Ok(-2i8));
    assert_eq!(i8::try_from(FBin::ONE >> 1), Err(LossOfPrecision));
    assert_eq!(i8::try_from(FBin::from_parts(i8::MAX.into(), 0)), Ok(i8::MAX));
    assert_eq!(i8::try_from(FBin::ONE << 7), Err(OutOfBounds));
    assert_eq!(i128::try_from(FBin::from_parts(i128::MAX.into(), 0)), Ok(i128::MAX));
    assert_eq!(i128::try_from(FBin::from_parts(i128::MIN.into(), 0)), Ok(i128::MIN));
    assert_eq!(i128::try_from(FBin::ONE << 127), Err(OutOfBounds));
    assert_eq!(i8::try_from(FBin::INFINITY), Err(OutOfBounds));
    assert_eq!(i128::try_from(FBin::NEG_INFINITY), Err(OutOfBounds));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_from_f32() {
    assert_eq!(FBin::try_from(0f32).unwrap(), fbig!(0x0));
    assert_eq!(FBin::try_from(-0f32).unwrap(), fbig!(0x0));
    assert_eq!(FBin::try_from(1f32).unwrap(), fbig!(0x1));
    assert_eq!(FBin::try_from(-1f32).unwrap(), fbig!(-0x1));
    assert_eq!(FBin::try_from(-1f32).unwrap().precision(), 24);
    assert_eq!(FBin::try_from(1234f32).unwrap(), fbig!(0x4d2));
    assert_eq!(FBin::try_from(-1234f32).unwrap(), fbig!(-0x4d2));
    assert_eq!(12.34f32.to_bits(), 0x414570a4); // exact value: 12.340000152587890625
    assert_eq!(FBin::try_from(12.34f32).unwrap(), fbig!(0x315c29p-18));
    assert_eq!(FBin::try_from(-12.34f32).unwrap(), fbig!(-0x315c29p-18));
    assert_eq!(1e-40_f32.to_bits(), 0x000116c2); // subnormal
    assert_eq!(FBin::try_from(1e-40_f32).unwrap(), fbig!(0x116c2p-149));
    assert_eq!(FBin::try_from(-1e-40_f32).unwrap(), fbig!(-0x116c2p-149));
    assert_eq!(FBin::try_from(-1e-40_f32).unwrap().precision(), 17);
    assert_eq!(FBin::try_from(f32::INFINITY).unwrap(), FBin::INFINITY);
    assert_eq!(FBin::try_from(f32::NEG_INFINITY).unwrap(), FBin::NEG_INFINITY);
    assert!(FBin::try_from(f32::NAN).is_err());
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_from_f64() {
    assert_eq!(FBin::try_from(0f64).unwrap(), fbig!(0x0));
    assert_eq!(FBin::try_from(-0f64).unwrap(), fbig!(0x0));
    assert_eq!(FBin::try_from(1f64).unwrap(), fbig!(0x1));
    assert_eq!(FBin::try_from(-1f64).unwrap(), fbig!(-0x1));
    assert_eq!(FBin::try_from(-1f64).unwrap().precision(), 53);
    assert_eq!(FBin::try_from(1234f64).unwrap(), fbig!(0x4d2));
    assert_eq!(FBin::try_from(-1234f64).unwrap(), fbig!(-0x4d2));
    assert_eq!(12.34f64.to_bits(), 0x4028ae147ae147ae); // exact value: 12.339999999999999857891452847979962825775146484375
    assert_eq!(FBin::try_from(12.34f64).unwrap(), fbig!(0xc570a3d70a3d7p-48));
    assert_eq!(FBin::try_from(-12.34f64).unwrap(), fbig!(-0xc570a3d70a3d7p-48));
    assert_eq!(1e-308_f64.to_bits(), 0x000730d67819e8d2); // subnormal
    assert_eq!(FBin::try_from(1e-308_f64).unwrap(), fbig!(0x730d67819e8d2p-1074));
    assert_eq!(FBin::try_from(-1e-308_f64).unwrap(), fbig!(-0x730d67819e8d2p-1074));
    assert_eq!(FBin::try_from(-1e-308_f64).unwrap().precision(), 51);
    assert_eq!(FBin::try_from(f64::INFINITY).unwrap(), FBin::INFINITY);
    assert_eq!(FBin::try_from(f64::NEG_INFINITY).unwrap(), FBin::NEG_INFINITY);
    assert!(FBin::try_from(f64::NAN).is_err());
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_to_int() {
    assert_eq!(fbig!(0x0).to_int(), Exact(ibig!(0)));
    assert_eq!(fbig!(0x1).to_int(), Exact(ibig!(1)));
    assert_eq!(fbig!(-0x1).to_int(), Exact(ibig!(-1)));
    assert_eq!(fbig!(0x1234).to_int(), Exact(ibig!(0x1234)));
    assert_eq!(fbig!(0x1234p-4).to_int(), Inexact(ibig!(0x123), NoOp));
    assert_eq!(fbig!(-0x1234p-8).to_int(), Inexact(ibig!(-0x12), NoOp));
    assert_eq!(fbig!(0x1234p-16).to_int(), Inexact(ibig!(0), NoOp));
    assert_eq!(fbig!(0x1234p4).to_int(), Exact(ibig!(0x1234) << 4));
    assert_eq!(fbig!(-0x1234p8).to_int(), Exact(ibig!(-0x1234) << 8));

    assert_eq!(dbig!(0).to_int(), Exact(ibig!(0)));
    assert_eq!(dbig!(1).to_int(), Exact(ibig!(1)));
    assert_eq!(dbig!(-1).to_int(), Exact(ibig!(-1)));
    assert_eq!(dbig!(1234).to_int(), Exact(ibig!(1234)));
    assert_eq!(dbig!(1234e-1).to_int(), Inexact(ibig!(123), NoOp));
    assert_eq!(dbig!(-1234e-2).to_int(), Inexact(ibig!(-12), NoOp));
    assert_eq!(dbig!(1234e-4).to_int(), Inexact(ibig!(0), NoOp));
    assert_eq!(dbig!(1234e1).to_int(), Exact(ibig!(12340)));
    assert_eq!(dbig!(-1234e2).to_int(), Exact(ibig!(-123400)));
    assert_eq!(dbig!(255e-1).to_int(), Inexact(ibig!(26), AddOne));
    assert_eq!(dbig!(-255e-1).to_int(), Inexact(ibig!(-26), SubOne));
}

#[test]
#[should_panic]
fn test_inf_to_int() {
    let _ = DBig::INFINITY.to_int();
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_to_f32() {
    let fbig_cases = [
        // fbig value, f32 value, conversion error
        (fbig!(0x0), Exact(0.), None),
        (fbig!(0x1), Exact(1.), None),
        (fbig!(-0x1), Exact(-1.), None),
        (fbig!(-0x1234), Exact(-4660.), None),
        (fbig!(0x1234p-3), Exact(582.5), None),
        (fbig!(0x1234p-16), Exact(0.07110596), None),
        // exact value: 3.8549410571968246689670642581279215812574412414193147924379... × 10^-21
        (fbig!(0x123456789p-100), Inexact(3.8549407e-21, NoOp), Some(LossOfPrecision)),
        (fbig!(-0x987654321p50), Inexact(-4.607888e25, NoOp), Some(LossOfPrecision)),
    ];
    for (big, small, reason) in fbig_cases {
        assert_eq!(big.to_f32(), small);
        if let Some(reason) = reason {
            assert_eq!(f32::try_from(big), Err(reason));
        } else {
            assert_eq!(f32::try_from(big), Ok(small.value()));
        }
    }

    // some Repr cases, all round to even
    assert_eq!(fbig!(0x123456789p-100).repr().to_f32(), Inexact(3.854941e-21, AddOne));

    // boundary cases
    assert_eq!(FBig::<Zero, 2>::INFINITY.to_f32(), Inexact(f32::INFINITY, NoOp));
    assert_eq!(FBig::<Zero, 2>::NEG_INFINITY.to_f32(), Inexact(f32::NEG_INFINITY, NoOp));
    assert_eq!(f32::try_from(FBig::<Zero, 2>::INFINITY), Err(LossOfPrecision));
    assert_eq!(f32::try_from(FBig::<Zero, 2>::NEG_INFINITY), Err(LossOfPrecision));
    assert_eq!(fbig!(0x1p200).to_f32(), Inexact(f32::INFINITY, AddOne)); // overflow
    assert_eq!(fbig!(-0x1p200).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(f32::try_from(fbig!(0x1p200)), Err(OutOfBounds));
    assert_eq!(f32::try_from(fbig!(-0x1p200)), Err(OutOfBounds));
    assert_eq!(fbig!(0x1p128).to_f32(), Inexact(f32::INFINITY, AddOne)); // boundary for overflow
    assert_eq!(fbig!(-0x1p128).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(f32::try_from(fbig!(0x1p128)), Err(OutOfBounds));
    assert_eq!(f32::try_from(fbig!(-0x1p128)), Err(OutOfBounds));
    assert_eq!(fbig!(0xffffffffp96).to_f32(), Inexact(f32::MAX, NoOp));
    assert_eq!(fbig!(0xffffffffp96).repr().to_f32(), Inexact(f32::INFINITY, AddOne));
    assert_eq!(f32::try_from(fbig!(0xffffffffp96)), Err(LossOfPrecision));
    assert_eq!(f32::try_from(fbig!(0xffffffffp96).into_repr()), Err(OutOfBounds));
    assert_eq!(fbig!(-0xffffffffp96).to_f32(), Inexact(f32::MIN, NoOp));
    assert_eq!(fbig!(-0xffffffffp96).repr().to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(f32::try_from(fbig!(0xffffffffp96)), Err(LossOfPrecision));
    assert_eq!(f32::try_from(fbig!(0xffffffffp96).into_repr()), Err(OutOfBounds));
    assert_eq!(fbig!(0x1p-140).to_f32(), Exact(f32::from_bits(0x200))); // subnormal
    assert_eq!(fbig!(-0x1p-140).to_f32(), Exact(-f32::from_bits(0x200)));
    assert_eq!(fbig!(0x1p-149).to_f32(), Exact(f32::from_bits(0x1)));
    assert_eq!(fbig!(-0x1p-149).to_f32(), Exact(-f32::from_bits(0x1)));
    assert_eq!(f32::try_from(fbig!(0x1p-149)).unwrap(), f32::from_bits(0x1));
    assert_eq!(f32::try_from(fbig!(-0x1p-149)).unwrap(), -f32::from_bits(0x1));
    assert_eq!(fbig!(0x1p-150).to_f32(), Inexact(0f32, NoOp)); // boundary for underflow
    assert_eq!(fbig!(-0x1p-150).to_f32(), Inexact(-0f32, NoOp));
    assert_eq!(f32::try_from(fbig!(0x1p-150)), Err(LossOfPrecision));
    assert_eq!(f32::try_from(fbig!(-0x1p-150)), Err(LossOfPrecision));
    assert_eq!(fbig!(0xffffffffp-182).to_f32(), Inexact(0f32, NoOp));
    assert_eq!(fbig!(-0xffffffffp-182).to_f32(), Inexact(-0f32, NoOp));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_to_f64() {
    let fbig_cases = [
        // fbig value, f64 value, conversion error
        (fbig!(0x0), Exact(0.), None),
        (fbig!(0x1), Exact(1.), None),
        (fbig!(-0x1), Exact(-1.), None),
        (fbig!(-0x1234), Exact(-4660.), None),
        (fbig!(0x1234p-3), Exact(582.5), None),
        (fbig!(0x1234p-16), Exact(0.07110595703125), None),
        (fbig!(0x123456789p-100), Exact(3.854941057196825e-21), None),
        (fbig!(-0x987654321p50), Exact(-4.607887924007194e25), None),
        // exact value: 3.3436283752161326232549204599099774676691163414240905497490... × 10^-39
        (
            fbig!(0x1234567890123456789p-200),
            Inexact(3.3436283752161324e-39, NoOp),
            Some(LossOfPrecision),
        ),
        // exact value: 72310453210697978489701299687443815627510656356042859969687735028883143242326999040
        (
            fbig!(-0x9876543210987654321p200),
            Inexact(-7.231045321069798e82, SubOne),
            Some(LossOfPrecision),
        ),
    ];
    for (big, small, reason) in fbig_cases {
        assert_eq!(big.to_f64(), small);
        if let Some(reason) = reason {
            assert_eq!(f64::try_from(big), Err(reason));
        } else {
            assert_eq!(f64::try_from(big), Ok(small.value()));
        }
    }

    assert_eq!(FBig::<Zero, 2>::INFINITY.to_f64(), Inexact(f64::INFINITY, NoOp));
    assert_eq!(FBig::<Zero, 2>::NEG_INFINITY.to_f64(), Inexact(f64::NEG_INFINITY, NoOp));
    assert_eq!(f64::try_from(FBig::<Zero, 2>::INFINITY), Err(LossOfPrecision));
    assert_eq!(f64::try_from(FBig::<Zero, 2>::NEG_INFINITY), Err(LossOfPrecision));
    assert_eq!(fbig!(0x1p2000).to_f64(), Inexact(f64::INFINITY, AddOne)); // overflow
    assert_eq!(fbig!(-0x1p2000).to_f64(), Inexact(f64::NEG_INFINITY, SubOne));
    assert_eq!(f64::try_from(fbig!(0x1p2000)), Err(OutOfBounds));
    assert_eq!(f64::try_from(fbig!(-0x1p2000)), Err(OutOfBounds));
    assert_eq!(fbig!(0x1p1024).to_f64(), Inexact(f64::INFINITY, AddOne)); // boundary for overflow
    assert_eq!(fbig!(-0x1p1024).to_f64(), Inexact(f64::NEG_INFINITY, SubOne));
    assert_eq!(f64::try_from(fbig!(0x1p1024)), Err(OutOfBounds));
    assert_eq!(f64::try_from(fbig!(-0x1p1024)), Err(OutOfBounds));
    assert_eq!(fbig!(0xffffffffffffffffp960).to_f64(), Inexact(f64::INFINITY, AddOne));
    assert_eq!(fbig!(-0xffffffffffffffffp960).to_f64(), Inexact(f64::NEG_INFINITY, SubOne));
    assert_eq!(fbig!(0x1p-1060).to_f64(), Exact(f64::from_bits(0x4000))); // subnormal
    assert_eq!(fbig!(-0x1p-1060).to_f64(), Exact(-f64::from_bits(0x4000)));
    assert_eq!(fbig!(0x1p-1074).to_f64(), Exact(f64::from_bits(0x1)));
    assert_eq!(fbig!(-0x1p-1074).to_f64(), Exact(-f64::from_bits(0x1)));
    assert_eq!(fbig!(0x1p-1075).to_f64(), Inexact(0f64, NoOp)); // boundary for underflow
    assert_eq!(fbig!(-0x1p-1075).to_f64(), Inexact(-0f64, NoOp));
    assert_eq!(f64::try_from(fbig!(0x1p-1075)), Err(LossOfPrecision));
    assert_eq!(f64::try_from(fbig!(-0x1p-1075)), Err(LossOfPrecision));
    assert_eq!(fbig!(0xffffffffffffffffp-1139).to_f64(), Inexact(0f64, NoOp));
    assert_eq!(fbig!(-0xffffffffffffffffp-1139).to_f64(), Inexact(-0f64, NoOp));
    assert_eq!(f64::try_from(fbig!(0xffffffffffffffffp-1139)), Err(LossOfPrecision));
    assert_eq!(f64::try_from(fbig!(-0xffffffffffffffffp-1139)), Err(LossOfPrecision));
}

#[test]
fn test_from_ibig() {
    assert_eq!(FBin::from(ibig!(0)), fbig!(0));
    assert_eq!(FBin::from(ibig!(1)), fbig!(1));
    assert_eq!(FBin::from(ibig!(-1)), fbig!(-1));
    assert_eq!(FBin::from(ibig!(-0x1234)), fbig!(-0x1234));

    // digits test
    assert_eq!(FBin::from(ibig!(-0x1234)).precision(), 13);
    assert_eq!(FBin::from_parts(ibig!(0x1234), 12).precision(), 13);
    assert_eq!(DBig::from(ibig!(-1230)).precision(), 4);
    assert_eq!(DBig::from_parts(ibig!(0x1234), 12).precision(), 4);
    assert_eq!(FHex::from(ibig!(-0x1230)).precision(), 4);
    assert_eq!(FHex::from_parts(ibig!(0x1230), 12).precision(), 4);

    // use addition to test the digits (#28)
    assert_eq!(DBig::from(ubig!(10)) + DBig::from(ubig!(5)), DBig::from(ubig!(15)));
}
