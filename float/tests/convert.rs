use core::convert::TryFrom;
use dashu_base::Approximation::*;
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
#[rustfmt::skip]
fn test_base_change() {
    // binary -> decimal
    // 5 decimal digits precision < 20 binary digits precision
    assert_eq!(fbig!(0x12345).with_rounding::<HalfAway>().to_decimal(), Exact(dbig!(74565)));
    assert_eq!(fbig!(-0x12345p1).with_rounding::<HalfAway>().to_decimal(), Exact(dbig!(-149130)));
    assert_eq!(fbig!(0x12345p100).with_rounding::<HalfAway>().to_decimal(), Inexact(dbig!(945224e29), AddOne));
    assert_eq!(fbig!(0x12345p-1).with_rounding::<HalfAway>().to_decimal(), Exact(dbig!(372825e-1)));
    assert_eq!(fbig!(-0x12345p-100).with_rounding::<HalfAway>().to_decimal(), Inexact(dbig!(-588214e-31), NoOp));
    assert_eq!(FBig::<HalfAway, 2>::INFINITY.to_decimal(), Inexact(DBig::INFINITY, NoOp));
    assert_eq!(FBig::<HalfAway, 2>::NEG_INFINITY.to_decimal(), Inexact(DBig::NEG_INFINITY, NoOp));

    assert_eq!(fbig!(0x12345).with_rounding::<HalfAway>().with_base_and_precision::<10>(10), Exact(dbig!(74565)));
    assert_eq!(fbig!(-0x12345p1).with_rounding::<HalfAway>().with_base_and_precision::<10>(10), Exact(dbig!(-149130)));
    assert_eq!(fbig!(0x12345p100).with_rounding::<HalfAway>().with_base_and_precision::<10>(10), Inexact(dbig!(9452236701e25), AddOne));
    assert_eq!(fbig!(0x12345p-1).with_rounding::<HalfAway>().with_base_and_precision::<10>(10), Exact(dbig!(372825e-1)));
    assert_eq!(fbig!(-0x12345p-100).with_rounding::<HalfAway>().with_base_and_precision::<10>(10), Inexact(dbig!(-5882141340e-35), SubOne));

    // decimal -> binary
    // 16 binary digits precision < 5 decimal digits precision
    assert_eq!(dbig!(12345).with_rounding::<Zero>().to_binary(), Exact(fbig!(0x3039)));
    assert_eq!(dbig!(-12345e1).with_rounding::<Zero>().to_binary(), Exact(fbig!(-0xf11dp1)));
    assert_eq!(dbig!(12345e100).with_rounding::<Zero>().to_binary(), Inexact(fbig!(0xdc78p330), NoOp));
    assert_eq!(dbig!(12345e-1).with_rounding::<Zero>().to_binary(), Exact(fbig!(0x9a5p-1)));
    assert_eq!(dbig!(-12345e-100).with_rounding::<Zero>().to_binary(), Inexact(fbig!(-0xa8c2p-334), NoOp));
    assert_eq!(DBig::INFINITY.to_binary(), Inexact(FBig::<HalfAway, 2>::INFINITY, NoOp));
    assert_eq!(DBig::NEG_INFINITY.to_binary(), Inexact(FBig::<HalfAway, 2>::NEG_INFINITY, NoOp));

    assert_eq!(dbig!(12345).with_rounding::<Zero>().with_base_and_precision::<2>(30), Exact(fbig!(0x3039)));
    assert_eq!(dbig!(-12345e1).with_rounding::<Zero>().with_base_and_precision::<2>(30), Exact(fbig!(-0xf11dp1)));
    assert_eq!(dbig!(12345e100).with_rounding::<Zero>().with_base_and_precision::<2>(30), Inexact(fbig!(0x371e2de9p316), NoOp));
    assert_eq!(dbig!(12345e-1).with_rounding::<Zero>().with_base_and_precision::<2>(30), Exact(fbig!(0x9a5p-1)));
    assert_eq!(dbig!(-12345e-100).with_rounding::<Zero>().with_base_and_precision::<2>(30), Inexact(fbig!(-0x2a30a4e2p-348), NoOp));

    // binary -> hexadecimal
    assert_eq!(fbig!(0x12345).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x12345), 0)));
    assert_eq!(fbig!(-0x12345p1).with_base::<16>(), Exact(FHex::from_parts(ibig!(-0x2468a), 0)));
    assert_eq!(fbig!(0x12345p100).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x12345), 25)));
    assert_eq!(fbig!(0x54321p111).with_base::<16>(), Inexact(FHex::from_parts(ibig!(0x2a190), 28), NoOp));
    assert_eq!(fbig!(0x12345p-1).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x91a28), -1)));
    assert_eq!(fbig!(-0x12345p-100).with_base::<16>(), Exact(FHex::from_parts(ibig!(-0x12345), -25)));
    assert_eq!(fbig!(-0x12345p-111).with_base::<16>(), Exact(FHex::from_parts(ibig!(-0x2468a), -28)));
    assert_eq!(FBig::<Zero, 2>::INFINITY.with_base::<16>(), Inexact(FHex::INFINITY, NoOp));
    assert_eq!(FBig::<Zero, 2>::NEG_INFINITY.with_base::<16>(), Inexact(FHex::NEG_INFINITY, NoOp));

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
    let _ = dbig!(0x1234p-1).with_precision(0).value().with_base::<2>();
}

#[test]
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
fn test_to_f32() {
    assert_eq!(fbig!(0x0).to_f32(), Exact(0.));
    assert_eq!(fbig!(0x1).to_f32(), Exact(1.));
    assert_eq!(fbig!(-0x1).to_f32(), Exact(-1.));
    assert_eq!(fbig!(-0x1234).to_f32(), Exact(-4660.));
    assert_eq!(fbig!(0x1234p-3).to_f32(), Exact(582.5));
    assert_eq!(fbig!(0x1234p-16).to_f32(), Exact(0.07110596));
    // exact value: 3.8549410571968246689670642581279215812574412414193147924379... × 10^-21
    assert_eq!(fbig!(0x123456789p-100).to_f32(), Inexact(3.854941e-21, AddOne));
    // exact value: -46078879240071936454164480
    assert_eq!(fbig!(-0x987654321p50).to_f32(), Inexact(-4.607888e25, NoOp));

    assert_eq!(FBig::<Zero, 2>::INFINITY.to_f32(), Inexact(f32::INFINITY, NoOp));
    assert_eq!(FBig::<Zero, 2>::NEG_INFINITY.to_f32(), Inexact(f32::NEG_INFINITY, NoOp));
    assert_eq!(fbig!(0x1p200).to_f32(), Inexact(f32::INFINITY, AddOne)); // overflow
    assert_eq!(fbig!(-0x1p200).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(fbig!(0x1p128).to_f32(), Inexact(f32::INFINITY, AddOne)); // boundary for overflow
    assert_eq!(fbig!(-0x1p128).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(fbig!(0xffffffffp96).to_f32(), Inexact(f32::INFINITY, AddOne));
    assert_eq!(fbig!(-0xffffffffp96).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(fbig!(0x1p-140).to_f32(), Exact(f32::from_bits(0x200))); // subnormal
    assert_eq!(fbig!(-0x1p-140).to_f32(), Exact(-f32::from_bits(0x200)));
    assert_eq!(fbig!(0x1p-149).to_f32(), Exact(f32::from_bits(0x1)));
    assert_eq!(fbig!(-0x1p-149).to_f32(), Exact(-f32::from_bits(0x1)));
    assert_eq!(fbig!(0x1p-150).to_f32(), Inexact(0f32, NoOp)); // boundary for underflow
    assert_eq!(fbig!(-0x1p-150).to_f32(), Inexact(-0f32, NoOp));
    assert_eq!(fbig!(0xffffffffp-182).to_f32(), Inexact(0f32, NoOp));
    assert_eq!(fbig!(-0xffffffffp-182).to_f32(), Inexact(-0f32, NoOp));
}

#[test]
fn test_to_f64() {
    assert_eq!(fbig!(0x0).to_f64(), Exact(0.));
    assert_eq!(fbig!(0x1).to_f64(), Exact(1.));
    assert_eq!(fbig!(-0x1).to_f64(), Exact(-1.));
    assert_eq!(fbig!(-0x1234).to_f64(), Exact(-4660.));
    assert_eq!(fbig!(0x1234p-3).to_f64(), Exact(582.5));
    assert_eq!(fbig!(0x1234p-16).to_f64(), Exact(0.07110595703125));
    assert_eq!(fbig!(0x123456789p-100).to_f64(), Exact(3.854941057196825e-21));
    assert_eq!(fbig!(-0x987654321p50).to_f64(), Exact(-4.607887924007194e25));
    // exact value: 3.3436283752161326232549204599099774676691163414240905497490... × 10^-39
    assert_eq!(
        fbig!(0x1234567890123456789p-200).to_f64(),
        Inexact(3.3436283752161324e-39, NoOp)
    );
    // exact value: 72310453210697978489701299687443815627510656356042859969687735028883143242326999040
    assert_eq!(
        fbig!(-0x9876543210987654321p200).to_f64(),
        Inexact(-7.231045321069798e82, SubOne)
    );

    assert_eq!(FBig::<Zero, 2>::INFINITY.to_f64(), Inexact(f64::INFINITY, NoOp));
    assert_eq!(FBig::<Zero, 2>::NEG_INFINITY.to_f64(), Inexact(f64::NEG_INFINITY, NoOp));
    assert_eq!(fbig!(0x1p2000).to_f64(), Inexact(f64::INFINITY, AddOne)); // overflow
    assert_eq!(fbig!(-0x1p2000).to_f64(), Inexact(f64::NEG_INFINITY, SubOne));
    assert_eq!(fbig!(0x1p1024).to_f32(), Inexact(f32::INFINITY, AddOne)); // boundary for overflow
    assert_eq!(fbig!(-0x1p1024).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(fbig!(0xffffffffffffffffp960).to_f32(), Inexact(f32::INFINITY, AddOne));
    assert_eq!(fbig!(-0xffffffffffffffffp960).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(fbig!(0x1p-1060).to_f64(), Exact(f64::from_bits(0x4000))); // subnormal
    assert_eq!(fbig!(-0x1p-1060).to_f64(), Exact(-f64::from_bits(0x4000)));
    assert_eq!(fbig!(0x1p-1074).to_f64(), Exact(f64::from_bits(0x1)));
    assert_eq!(fbig!(-0x1p-1074).to_f64(), Exact(-f64::from_bits(0x1)));
    assert_eq!(fbig!(0x1p-1075).to_f64(), Inexact(0f64, NoOp)); // boundary for underflow
    assert_eq!(fbig!(-0x1p-1075).to_f64(), Inexact(-0f64, NoOp));
    assert_eq!(fbig!(0xffffffffffffffffp-1139).to_f64(), Inexact(0f64, NoOp));
    assert_eq!(fbig!(-0xffffffffffffffffp-1139).to_f64(), Inexact(-0f64, NoOp));
}
