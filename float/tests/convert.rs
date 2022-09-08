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

fn test_to_int() {
    todo!()
}
