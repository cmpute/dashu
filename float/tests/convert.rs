use core::convert::TryFrom;
use dashu_float::{DBig, round::{mode::{Zero, HalfAway}, Rounding::*}, FBig};
use dashu_base::Approximation::*;

mod helper_macros;

#[test]
fn test_ceil_floor() {
    assert_eq!(fbig!(0x0).ceil(), fbig!(0x0));
    assert_eq!(fbig!(0x1p1).ceil(), fbig!(0x1p1));
    assert_eq!(fbig!(0x1).ceil(), fbig!(0x1));
    assert_eq!(fbig!(0x1p-1).ceil(), fbig!(0x1));
    assert_eq!(fbig!(-0x1p1).ceil(), fbig!(-0x1p1));
    assert_eq!(fbig!(-0x1).ceil(), fbig!(-0x1));
    assert_eq!(fbig!(-0x1p-1).ceil(), fbig!(0x0));
    
    assert_eq!(fbig!(0x0).floor(), fbig!(0x0));
    assert_eq!(fbig!(0x1p1).floor(), fbig!(0x1p1));
    assert_eq!(fbig!(0x1).floor(), fbig!(0x1));
    assert_eq!(fbig!(0x1p-1).floor(), fbig!(0x0));
    assert_eq!(fbig!(-0x1p1).floor(), fbig!(-0x1p1));
    assert_eq!(fbig!(-0x1).floor(), fbig!(-0x1));
    assert_eq!(fbig!(-0x1p-1).floor(), fbig!(-0x1));
    
    assert_eq!(dbig!(0).ceil(), dbig!(0));
    assert_eq!(dbig!(1e1).ceil(), dbig!(1e1));
    assert_eq!(dbig!(1).ceil(), dbig!(1));
    assert_eq!(dbig!(1e-1).ceil(), dbig!(1));
    assert_eq!(dbig!(-1e1).ceil(), dbig!(-1e1));
    assert_eq!(dbig!(-1).ceil(), dbig!(-1));
    assert_eq!(dbig!(-1e-1).ceil(), dbig!(0));
    
    assert_eq!(dbig!(0).floor(), dbig!(0));
    assert_eq!(dbig!(1e1).floor(), dbig!(1e1));
    assert_eq!(dbig!(1).floor(), dbig!(1));
    assert_eq!(dbig!(1e-1).floor(), dbig!(0));
    assert_eq!(dbig!(-1e1).floor(), dbig!(-1e1));
    assert_eq!(dbig!(-1).floor(), dbig!(-1));
    assert_eq!(dbig!(-1e-1).floor(), dbig!(-1));
}

#[test]
fn test_trunc_fract() {
    // binary
    assert_eq!(fbig!(0x0).trunc(), fbig!(0x0));
    assert_eq!(fbig!(0x12p4).trunc(), fbig!(0x12p4));
    assert_eq!(fbig!(0x12).trunc(), fbig!(0x12));
    assert_eq!(fbig!(0x12p-4).trunc(), fbig!(0x1));
    assert_eq!(fbig!(0x12p-8).trunc(), fbig!(0x0));
    assert_eq!(fbig!(0x12p-12).trunc(), fbig!(0x0));
    assert_eq!(fbig!(-0x12p4).trunc(), fbig!(-0x12p4));
    assert_eq!(fbig!(-0x12).trunc(), fbig!(-0x12));
    assert_eq!(fbig!(-0x12p-4).trunc(), fbig!(-0x1));
    assert_eq!(fbig!(-0x12p-8).trunc(), fbig!(-0x0));
    assert_eq!(fbig!(-0x12p-12).trunc(), fbig!(0x0));
    
    assert_eq!(fbig!(0x0).fract(), fbig!(0x0));
    assert_eq!(fbig!(0x12p4).fract(), fbig!(0x0));
    assert_eq!(fbig!(0x12).fract(), fbig!(0x0));
    assert_eq!(fbig!(0x12p-4).fract(), fbig!(0x2p-4));
    assert_eq!(fbig!(0x12p-8).fract(), fbig!(0x12p-8));
    assert_eq!(fbig!(0x12p-12).fract(), fbig!(0x12p-12));
    assert_eq!(fbig!(-0x12p4).fract(), fbig!(0x0));
    assert_eq!(fbig!(-0x12).fract(), fbig!(0x0));
    assert_eq!(fbig!(-0x12p-4).fract(), fbig!(-0x2p-4));
    assert_eq!(fbig!(-0x12p-8).fract(), fbig!(-0x12p-8));
    assert_eq!(fbig!(-0x12p-12).fract(), fbig!(-0x12p-12));

    // decimal
    assert_eq!(dbig!(0).trunc(), dbig!(0));
    assert_eq!(dbig!(12e1).trunc(), dbig!(12e1));
    assert_eq!(dbig!(12).trunc(), dbig!(12));
    assert_eq!(dbig!(12e-1).trunc(), dbig!(1));
    assert_eq!(dbig!(12e-2).trunc(), dbig!(0));
    assert_eq!(dbig!(12e-3).trunc(), dbig!(0));
    assert_eq!(dbig!(-12e1).trunc(), dbig!(-12e1));
    assert_eq!(dbig!(-12).trunc(), dbig!(-12));
    assert_eq!(dbig!(-12e-1).trunc(), dbig!(-1));
    assert_eq!(dbig!(-12e-2).trunc(), dbig!(-0));
    assert_eq!(dbig!(-12e-3).trunc(), dbig!(0));
    
    assert_eq!(dbig!(0).fract(), dbig!(0));
    assert_eq!(dbig!(12e1).fract(), dbig!(0));
    assert_eq!(dbig!(12).fract(), dbig!(0));
    assert_eq!(dbig!(12e-1).fract(), dbig!(2e-1));
    assert_eq!(dbig!(12e-2).fract(), dbig!(12e-2));
    assert_eq!(dbig!(12e-3).fract(), dbig!(12e-3));
    assert_eq!(dbig!(-12e1).fract(), dbig!(0));
    assert_eq!(dbig!(-12).fract(), dbig!(0));
    assert_eq!(dbig!(-12e-1).fract(), dbig!(-2e-1));
    assert_eq!(dbig!(-12e-2).fract(), dbig!(-12e-2));
    assert_eq!(dbig!(-12e-3).fract(), dbig!(-12e-3));

    // precision test
    assert_eq!(dbig!(12).trunc().precision(), 2);
    assert_eq!(dbig!(12).fract().precision(), 0);
    assert_eq!(dbig!(12e-1).trunc().precision(), 1);
    assert_eq!(dbig!(12e-1).fract().precision(), 1);
    assert_eq!(dbig!(12e-2).trunc().precision(), 0);
    assert_eq!(dbig!(12e-2).fract().precision(), 2);
}

#[test]
#[should_panic]
fn test_floor_inf() {
    let _ = DBig::INFINITY.floor();
}

#[test]
#[should_panic]
fn test_ceil_inf() {
    let _ = DBig::INFINITY.ceil();
}

#[test]
#[should_panic]
fn test_trunc_inf() {
    let _ = DBig::INFINITY.trunc();
}

#[test]
#[should_panic]
fn test_fract_inf() {
    let _ = DBig::INFINITY.fract();
}

#[test]
fn test_base_change() {
    // binary -> decimal
    // 5 decimal digits precision < 20 binary digits precision
    assert_eq!(fbig!(0x12345).with_rounding::<HalfAway>().to_decimal(), Exact(dbig!(74565)));
    assert_eq!(fbig!(-0x12345p1).with_rounding::<HalfAway>().to_decimal(), Exact(dbig!(-149130)));
    assert_eq!(fbig!(0x12345p100).with_rounding::<HalfAway>().to_decimal(), Inexact(dbig!(945224e29), AddOne));
    assert_eq!(fbig!(0x12345p-1).with_rounding::<HalfAway>().to_decimal(), Exact(dbig!(372825e-1)));
    assert_eq!(fbig!(-0x12345p-100).with_rounding::<HalfAway>().to_decimal(), Inexact(dbig!(-588214e-31), NoOp));

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

    assert_eq!(dbig!(12345).with_rounding::<Zero>().with_base_and_precision::<2>(30), Exact(fbig!(0x3039)));
    assert_eq!(dbig!(-12345e1).with_rounding::<Zero>().with_base_and_precision::<2>(30), Exact(fbig!(-0xf11dp1)));
    assert_eq!(dbig!(12345e100).with_rounding::<Zero>().with_base_and_precision::<2>(30), Inexact(fbig!(0x371e2de9p316), NoOp));
    assert_eq!(dbig!(12345e-1).with_rounding::<Zero>().with_base_and_precision::<2>(30), Exact(fbig!(0x9a5p-1)));
    assert_eq!(dbig!(-12345e-100).with_rounding::<Zero>().with_base_and_precision::<2>(30), Inexact(fbig!(-0x2a30a4e2p-348), NoOp));

    type FHex = FBig<Zero, 16>;

    // binary -> hexadecimal
    assert_eq!(fbig!(0x12345).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x12345), 0)));
    assert_eq!(fbig!(-0x12345p1).with_base::<16>(), Exact(FHex::from_parts(ibig!(-0x2468a), 0)));
    assert_eq!(fbig!(0x12345p100).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x12345), 25)));
    assert_eq!(fbig!(0x54321p111).with_base::<16>(), Inexact(FHex::from_parts(ibig!(0x2a190), 28), NoOp));
    assert_eq!(fbig!(0x12345p-1).with_base::<16>(), Exact(FHex::from_parts(ibig!(0x91a28), -1)));
    assert_eq!(fbig!(-0x12345p-100).with_base::<16>(), Exact(FHex::from_parts(ibig!(-0x12345), -25)));
    assert_eq!(fbig!(-0x12345p-111).with_base::<16>(), Exact(FHex::from_parts(ibig!(-0x2468a), -28)));

    // hexadecimal -> binary
    assert_eq!(FHex::from_parts(ibig!(0x12345), 0).to_binary(), Exact(fbig!(0x12345)));
    assert_eq!(FHex::from_parts(ibig!(-0x12345), 1).to_binary(), Exact(fbig!(-0x12345p4)));
    assert_eq!(FHex::from_parts(ibig!(0x12345), 100).to_binary(), Exact(fbig!(0x12345p400)));
    assert_eq!(FHex::from_parts(ibig!(0x12345), -1).to_binary(), Exact(fbig!(0x12345p-4)));
    assert_eq!(FHex::from_parts(ibig!(-0x12345), 100).to_binary(), Exact(fbig!(-0x12345p400)));
}

#[test]
fn test_from_f32() {
    assert_eq!(FBig::try_from(0f32).unwrap(), fbig!(0x0));
    assert_eq!(FBig::try_from(-0f32).unwrap(), fbig!(0x0));
    assert_eq!(FBig::try_from(1f32).unwrap(), fbig!(0x1));
    assert_eq!(FBig::try_from(-1f32).unwrap(), fbig!(-0x1));
    assert_eq!(FBig::try_from(1234f32).unwrap(), fbig!(0x4d2));
    assert_eq!(FBig::try_from(-1234f32).unwrap(), fbig!(-0x4d2));
    assert_eq!(12.34f32.to_bits(), 0x414570a4); // exact value: 12.340000152587890625
    assert_eq!(FBig::try_from(12.34f32).unwrap(), fbig!(0x315c29p-18));
    assert_eq!(FBig::try_from(-12.34f32).unwrap(), fbig!(-0x315c29p-18));
    assert_eq!(1e-40_f32.to_bits(), 0x000116c2); // subnormal
    assert_eq!(FBig::try_from(1e-40_f32).unwrap(), fbig!(0x116c2p-126));
    assert_eq!(FBig::try_from(-1e-40_f32).unwrap(), fbig!(-0x116c2p-126));
    assert_eq!(FBig::<Zero, 2>::try_from(f32::INFINITY).unwrap(), FBig::INFINITY);
    assert_eq!(FBig::<Zero, 2>::try_from(f32::NEG_INFINITY).unwrap(), FBig::NEG_INFINITY);
    assert!(FBig::<Zero, 2>::try_from(f32::NAN).is_err());
}

#[test]
fn test_from_f64() {
    assert_eq!(FBig::try_from(0f64).unwrap(), fbig!(0x0));
    assert_eq!(FBig::try_from(-0f64).unwrap(), fbig!(0x0));
    assert_eq!(FBig::try_from(1f64).unwrap(), fbig!(0x1));
    assert_eq!(FBig::try_from(-1f64).unwrap(), fbig!(-0x1));
    assert_eq!(FBig::try_from(1234f64).unwrap(), fbig!(0x4d2));
    assert_eq!(FBig::try_from(-1234f64).unwrap(), fbig!(-0x4d2));
    assert_eq!(12.34f64.to_bits(), 0x4028ae147ae147ae); // exact value: 12.339999999999999857891452847979962825775146484375
    assert_eq!(FBig::try_from(12.34f64).unwrap(), fbig!(0xc570a3d70a3d7p-48));
    assert_eq!(FBig::try_from(-12.34f64).unwrap(), fbig!(-0xc570a3d70a3d7p-48));
    assert_eq!(1e-308_f64.to_bits(), 0x000730d67819e8d2); // subnormal
    assert_eq!(FBig::try_from(1e-308_f64).unwrap(), fbig!(0x730d67819e8d2p-1022));
    assert_eq!(FBig::try_from(-1e-308_f64).unwrap(), fbig!(-0x730d67819e8d2p-1022));
    assert_eq!(FBig::<Zero, 2>::try_from(f64::INFINITY).unwrap(), FBig::INFINITY);
    assert_eq!(FBig::<Zero, 2>::try_from(f64::NEG_INFINITY).unwrap(), FBig::NEG_INFINITY);
    assert!(FBig::<Zero, 2>::try_from(f64::NAN).is_err());
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
    assert_eq!(fbig!(0x1234p-16).to_f32(), Exact(0.07110595703125));
    // exact value: 3.8549410571968246689670642581279215812574412414193147924379... × 10^-21
    assert_eq!(fbig!(0x123456789p-100).to_f32(), Inexact(3.85494115107127239027e-21, AddOne));
    // exact value: -46078879240071936454164480
    assert_eq!(fbig!(-0x987654321p50).to_f32(), Inexact(-4.60788783382261110732e+25, NoOp));

    assert_eq!(FBig::<Zero, 2>::INFINITY.to_f32(), Inexact(f32::INFINITY, NoOp));
    assert_eq!(FBig::<Zero, 2>::NEG_INFINITY.to_f32(), Inexact(f32::NEG_INFINITY, NoOp));
    assert_eq!((fbig!(0x1) << 200).to_f32(), Inexact(f32::INFINITY, AddOne));
    assert_eq!((fbig!(-0x1) << 200).to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
}

#[test]
fn test_to_f64() {
    assert_eq!(fbig!(0x0).to_f64(), Exact(0.));
    assert_eq!(fbig!(0x1).to_f64(), Exact(1.));
    assert_eq!(fbig!(-0x1).to_f64(), Exact(-1.));
    assert_eq!(fbig!(-0x1234).to_f64(), Exact(-4660.));
    assert_eq!(fbig!(0x1234p-3).to_f64(), Exact(582.5));
    assert_eq!(fbig!(0x1234p-16).to_f64(), Exact(0.07110595703125));
    assert_eq!(fbig!(0x123456789p-100).to_f64(), Exact(3.85494105719682466897e-21));
    assert_eq!(fbig!(-0x987654321p50).to_f64(), Exact(-4.60788792400719364542e+25));
    // exact value: 3.3436283752161326232549204599099774676691163414240905497490... × 10^-39
    assert_eq!(fbig!(0x1234567890123456789p-200).to_f64(), Inexact(3.34362837521613240285e-39, NoOp));
    // exact value: 72310453210697978489701299687443815627510656356042859969687735028883143242326999040
    assert_eq!(fbig!(-0x9876543210987654321p200).to_f64(), Inexact(-7.23104532106979813055e+82, SubOne));

    assert_eq!(FBig::<Zero, 2>::INFINITY.to_f64(), Inexact(f64::INFINITY, NoOp));
    assert_eq!(FBig::<Zero, 2>::NEG_INFINITY.to_f64(), Inexact(f64::NEG_INFINITY, NoOp));
    assert_eq!((fbig!(0x1) << 2000).to_f64(), Inexact(f64::INFINITY, AddOne));
    assert_eq!((fbig!(-0x1) << 2000).to_f64(), Inexact(f64::NEG_INFINITY, SubOne));
}
