use core::convert::TryFrom;
use core::str::FromStr;
use dashu_base::{Approximation::*, ConversionError::*};
use dashu_float::{
    round::{
        mode::{HalfAway, HalfEven, Up, Zero},
        Rounding::*,
    },
    DBig, FBig,
};
use dashu_int::{IBig, UBig};

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
#[rustfmt::skip::macros(fbig, dbig)]
fn test_base_change_exact_unlimited_precision() {
    use dashu_int::IBig;

    type FBinH = FBig<HalfAway, 2>;
    type DBinZ = FBig<Zero, 10>;

    // B=10 → B=2, positive exponent (below threshold)
    assert_eq!(
        dbig!(5e1).with_precision(0).unwrap().with_base::<2>(),
        Exact(FBinH::from_str("0x19p1").unwrap())
    );
    // B=10 → B=2, positive exponent exceeding 32-bit threshold
    assert_eq!(
        dbig!(1e20).with_precision(0).unwrap().with_base::<2>(),
        Exact(FBinH::from_str("0x56BC75E2D631p20").unwrap())
    );
    // B=10 → B=2, negative exponent, exact division by r^|exp|
    assert_eq!(
        dbig!(5e-1).with_precision(0).unwrap().with_base::<2>(),
        Exact(FBinH::from_str("0x1p-1").unwrap())
    );
    assert_eq!(
        dbig!(125e-3).with_precision(0).unwrap().with_base::<2>(),
        Exact(FBinH::from_str("0x1p-3").unwrap())
    );
    assert_eq!(
        dbig!(3125e-5).with_precision(0).unwrap().with_base::<2>(),
        Exact(FBinH::from_str("0x1p-5").unwrap())
    );
    // B=10 → B=2, negative exponent exceeding 32-bit threshold, exact division
    // 5^20 = 95367431640625, and 95367431640625 × 10^(-20) = 2^(-20)
    assert_eq!(
        dbig!(95367431640625e-20)
            .with_precision(0)
            .unwrap()
            .with_base::<2>(),
        Exact(FBinH::from_str("0x1p-20").unwrap())
    );
    // B=2 → B=10, positive exponent (NewB is a multiple of B)
    assert_eq!(
        fbig!(0x1p20).with_precision(0).unwrap().with_base::<10>(),
        Exact(DBinZ::from_str("1048576").unwrap())
    );
    assert_eq!(
        fbig!(0x5p10).with_precision(0).unwrap().with_base::<10>(),
        Exact(DBinZ::from_str("5120").unwrap())
    );

    // --- Other base combinations ---

    // B=6 → B=2 (factor_base(6,2)=(1,3), positive exp)
    // 3 × 6^2 = 108 = 27 × 2^2
    {
        type F6 = FBig<HalfAway, 6>;
        assert_eq!(
            F6::from_parts(IBig::from(3), 2)
                .with_precision(0)
                .unwrap()
                .with_base::<2>(),
            Exact(FBinH::from_parts(IBig::from(27), 2))
        );
    }

    // B=6 → B=2 (factor_base(6,2)=(1,3), negative exp, exact)
    // 9 × 6^(-2) = 9/36 = 0.25 = 1 × 2^(-2)
    {
        type F6 = FBig<HalfAway, 6>;
        assert_eq!(
            F6::from_parts(IBig::from(9), -2)
                .with_precision(0)
                .unwrap()
                .with_base::<2>(),
            Exact(FBinH::from_parts(IBig::from(1), -2))
        );
    }

    // B=12 → B=2 (factor_base(12,2)=(2,3), positive exp)
    // 3 × 12^2 = 432 = 27 × 2^4
    {
        type F12 = FBig<HalfAway, 12>;
        assert_eq!(
            F12::from_parts(IBig::from(3), 2)
                .with_precision(0)
                .unwrap()
                .with_base::<2>(),
            Exact(FBinH::from_parts(IBig::from(27), 4))
        );
    }

    // B=12 → B=3 (factor_base(12,3)=(1,4), positive exp)
    // 5 × 12^2 = 720 = 80 × 3^2
    {
        type F12 = FBig<HalfAway, 12>;
        type F3 = FBig<HalfAway, 3>;
        assert_eq!(
            F12::from_parts(IBig::from(5), 2)
                .with_precision(0)
                .unwrap()
                .with_base::<3>(),
            Exact(F3::from_parts(IBig::from(80), 2))
        );
    }

    // B=12 → B=4 (factor_base(12,4)=(1,3), negative exp, exact)
    // 9 × 12^(-1) = 0.75 = 3 × 4^(-1)
    {
        type F12 = FBig<HalfAway, 12>;
        type F4 = FBig<HalfAway, 4>;
        assert_eq!(
            F12::from_parts(IBig::from(9), -1)
                .with_precision(0)
                .unwrap()
                .with_base::<4>(),
            Exact(F4::from_parts(IBig::from(3), -1))
        );
    }

    // B=3 → B=6 (NewB % B == 0, positive exp)
    // 5 × 3^3 = 135
    {
        type F3 = FBig<HalfAway, 3>;
        type F6 = FBig<HalfAway, 6>;
        assert_eq!(
            F3::from_parts(IBig::from(5), 3)
                .with_precision(0)
                .unwrap()
                .with_base::<6>(),
            Exact(F6::from_parts(IBig::from(135), 0))
        );
    }

    // B=3 → B=9 (power of base: ilog_exact(9,3)=2)
    // 5 × 3^3 = 135 = 15 × 9^1
    {
        type F3 = FBig<HalfAway, 3>;
        type F9 = FBig<HalfAway, 9>;
        assert_eq!(
            F3::from_parts(IBig::from(5), 3)
                .with_precision(0)
                .unwrap()
                .with_base::<9>(),
            Exact(F9::from_parts(IBig::from(15), 1))
        );
    }

    // B=64 → B=8 (power of base: ilog_exact(64,8)=2)
    // 7 × 64^2 = 28672 = 7 × 8^4
    {
        type F64 = FBig<HalfAway, 64>;
        type F8 = FBig<HalfAway, 8>;
        assert_eq!(
            F64::from_parts(IBig::from(7), 2)
                .with_precision(0)
                .unwrap()
                .with_base::<8>(),
            Exact(F8::from_parts(IBig::from(7), 4))
        );
    }
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
    // Directed overflow (opendp-num §3): a value beyond f32::MAX saturates to the largest *finite*
    // f32 under toward-zero (fbig! is Zero mode), and to ±∞ only under outward modes (Up/Away).
    assert_eq!(fbig!(0x1p200).to_f32(), Inexact(f32::MAX, NoOp)); // overflow
    assert_eq!(fbig!(-0x1p200).to_f32(), Inexact(f32::MIN, NoOp));
    assert_eq!(
        FBig::<Up, 2>::from_parts(IBig::ONE, 200).to_f32(),
        Inexact(f32::INFINITY, AddOne)
    );
    assert_eq!(f32::try_from(fbig!(0x1p200)), Err(OutOfBounds)); // beyond MAX is out of range (any mode)
    assert_eq!(f32::try_from(fbig!(-0x1p200)), Err(OutOfBounds));
    assert_eq!(fbig!(0x1p128).to_f32(), Inexact(f32::MAX, NoOp)); // boundary for overflow
    assert_eq!(fbig!(-0x1p128).to_f32(), Inexact(f32::MIN, NoOp));
    assert_eq!(f32::try_from(fbig!(0x1p128)), Err(OutOfBounds));
    assert_eq!(f32::try_from(fbig!(-0x1p128)), Err(OutOfBounds));
    assert_eq!(fbig!(0xffffffffp96).to_f32(), Inexact(f32::MAX, NoOp));
    assert_eq!(fbig!(0xffffffffp96).repr().to_f32(), Inexact(f32::INFINITY, AddOne));
    assert_eq!(f32::try_from(fbig!(0xffffffffp96)), Err(OutOfBounds));
    assert_eq!(f32::try_from(fbig!(0xffffffffp96).into_repr()), Err(OutOfBounds));
    assert_eq!(fbig!(-0xffffffffp96).to_f32(), Inexact(f32::MIN, NoOp));
    assert_eq!(fbig!(-0xffffffffp96).repr().to_f32(), Inexact(f32::NEG_INFINITY, SubOne));
    assert_eq!(f32::try_from(fbig!(0xffffffffp96)), Err(OutOfBounds));
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
            Inexact(-7.231045321069797e82, NoOp),
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
    // Directed overflow (opendp-num §3): beyond f64::MAX saturates to the largest finite under
    // toward-zero (fbig! is Zero), to ±∞ only under outward modes.
    assert_eq!(fbig!(0x1p2000).to_f64(), Inexact(f64::MAX, NoOp)); // overflow
    assert_eq!(fbig!(-0x1p2000).to_f64(), Inexact(f64::MIN, NoOp));
    assert_eq!(
        FBig::<Up, 2>::from_parts(IBig::ONE, 2000).to_f64(),
        Inexact(f64::INFINITY, AddOne)
    );
    assert_eq!(f64::try_from(fbig!(0x1p2000)), Err(OutOfBounds));
    assert_eq!(f64::try_from(fbig!(-0x1p2000)), Err(OutOfBounds));
    assert_eq!(fbig!(0x1p1024).to_f64(), Inexact(f64::MAX, NoOp)); // boundary for overflow
    assert_eq!(fbig!(-0x1p1024).to_f64(), Inexact(f64::MIN, NoOp));
    assert_eq!(f64::try_from(fbig!(0x1p1024)), Err(OutOfBounds));
    assert_eq!(f64::try_from(fbig!(-0x1p1024)), Err(OutOfBounds));
    assert_eq!(fbig!(0xffffffffffffffffp960).to_f64(), Inexact(f64::MAX, NoOp));
    assert_eq!(fbig!(-0xffffffffffffffffp960).to_f64(), Inexact(-f64::MAX, NoOp));
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

#[test]
fn test_dbig_to_f64() {
    // Regression test: DBig with significand exceeding 53 bits converting to f64

    // Small values that fit exactly in f64
    assert_eq!(dbig!(0).to_f64(), Exact(0.0));
    assert_eq!(dbig!(1).to_f64(), Exact(1.0));
    assert_eq!(dbig!(-1).to_f64(), Exact(-1.0));
    assert_eq!(dbig!(12345).to_f64(), Exact(12345.0));
    assert_eq!(dbig!(5e-1).to_f64(), Exact(0.5)); // 0.5 is exact in binary
    assert_eq!(dbig!(1e20).to_f64(), Exact(1e20)); // 5^20 is roughly 46 bits, so it's exact in f64

    // Values requiring rounding (more than 53 bits of precision in base 10)
    assert_eq!(
        dbig!(9007199254740993).to_f64(), // 2^53 + 1: tie, HalfAway rounds away from zero
        Inexact(9007199254740994.0, AddOne)
    );
    assert_eq!(dbig!(88888888888888888).to_f64(), Inexact(88888888888888888.0, AddOne));

    // Infinity
    assert_eq!(DBig::INFINITY.to_f64(), Inexact(f64::INFINITY, NoOp));
    assert_eq!(DBig::NEG_INFINITY.to_f64(), Inexact(f64::NEG_INFINITY, NoOp));
}

#[test]
fn test_dbig_to_f32() {
    assert_eq!(dbig!(0).to_f32(), Exact(0.0f32));
    assert_eq!(dbig!(1).to_f32(), Exact(1.0f32));
    assert_eq!(dbig!(-1).to_f32(), Exact(-1.0f32));

    // 1e10 fits exactly in f32 (24 mantissa bits, exponent fits)
    assert_eq!(dbig!(1e10).to_f32(), Exact(1e10f32));

    // 1e20 requires more than 24 bits of precision in binary
    assert_eq!(dbig!(1e20).to_f32(), Inexact(1e20f32, AddOne));

    // Values requiring rounding (more than 24 bits of precision in base 10)
    assert_eq!(dbig!(16777217).to_f32(), Inexact(16777218.0f32, AddOne)); // 2^24 + 1: tie, HalfAway rounds away from zero
    assert_eq!(dbig!(88888888888888888).to_f32(), Inexact(88888888888888888.0f32, AddOne));

    // Infinity
    assert_eq!(DBig::INFINITY.to_f32(), Inexact(f32::INFINITY, NoOp));
    assert_eq!(DBig::NEG_INFINITY.to_f32(), Inexact(f32::NEG_INFINITY, NoOp));
}

// A subnormal that sits just past a subnormal halfway must round straight to the
// target's exponent-dependent precision. Rounding through a fixed 53-bit intermediate
// drops the excess that lifts the value above the halfway, lands on the halfway, and
// the final encoding rounds to even -- one ULP below the correct result. The exact
// value (2m+1)/2^1075 +/- 1/2^(1075+j) is such a halfway nudged by a bit far below the
// 53rd; correctly rounded it is m+1 (nudged up) or m (nudged down), and the subnormal
// k*2^-1074 has bit pattern exactly k. Built in both base 2 and base 10 so the
// base-changing conversion path is exercised too.
#[test]
fn test_to_f64_subnormal_halfway() {
    fn check(m: u64, j: u32) {
        let scale = (1075 + j) as usize;
        let core = IBig::from(2 * m + 1) << j as usize;
        let five: IBig = UBig::from(5u8).pow(scale).into();
        for (delta, want) in [(IBig::ONE, m + 1), (-IBig::ONE, m)] {
            // magnitude, always positive; cover both signs (negation flips only the sign bit)
            let mag = &core + &delta;
            for negative in [false, true] {
                let sig = if negative { -mag.clone() } else { mag.clone() };
                let want = if negative { want | (1u64 << 63) } else { want };
                let base2 = FBig::<HalfEven, 2>::from_parts(sig.clone(), -(scale as isize));
                assert_eq!(base2.to_f64().value().to_bits(), want, "base 2, m={m}, neg={negative}");
                let base10 = DBig::from_parts(&sig * &five, -(scale as isize));
                assert_eq!(
                    base10.to_f64().value().to_bits(),
                    want,
                    "base 10, m={m}, neg={negative}"
                );
            }
        }
    }
    // m spans the subnormal significand width; j puts the nudge well below the 53rd bit
    // (but within the conversion's working precision), where a second rounding would
    // otherwise collapse the value onto the halfway.
    for m in [
        3,
        21,
        (1 << 8) + 5,
        (1 << 20) + 3,
        (1 << 33) + 7,
        (1 << 40) + 9,
    ] {
        check(m, 60);
    }
}

// [test_to_f64_subnormal_halfway] for f32 (subnormal spacing 2^-149, 24-bit mantissa).
#[test]
fn test_to_f32_subnormal_halfway() {
    fn check(m: u32, j: u32) {
        let scale = (150 + j) as usize;
        let core = IBig::from(2 * m + 1) << j as usize;
        let five: IBig = UBig::from(5u8).pow(scale).into();
        for (delta, want) in [(IBig::ONE, m + 1), (-IBig::ONE, m)] {
            // magnitude, always positive; cover both signs (negation flips only the sign bit)
            let mag = &core + &delta;
            for negative in [false, true] {
                let sig = if negative { -mag.clone() } else { mag.clone() };
                let want = if negative { want | (1u32 << 31) } else { want };
                let base2 = FBig::<HalfEven, 2>::from_parts(sig.clone(), -(scale as isize));
                assert_eq!(base2.to_f32().value().to_bits(), want, "base 2, m={m}, neg={negative}");
                let base10 = DBig::from_parts(&sig * &five, -(scale as isize));
                assert_eq!(
                    base10.to_f32().value().to_bits(),
                    want,
                    "base 10, m={m}, neg={negative}"
                );
            }
        }
    }
    for m in [3, 21, (1 << 10) + 5, (1 << 18) + 7] {
        check(m, 30);
    }
}

// High-precision decimals feed an oversized significand into the base-changing
// division; this tripped a debug assertion (and double-rounded in release). Each must
// now convert without panicking to the correctly rounded f64 (oracle: Python float()).
#[test]
fn test_to_f64_high_precision() {
    let cases = [
        ("123456789012345678.9012345678901", 0x437b69b4ba630f35u64),
        ("3915263378237002511617337316730e-19", 0x4256ca327347ecd1),
        ("1234567890123456789012345678901e-5", 0x45246c993044fd55),
        ("9999999999999999999999999999999e-3", 0x45c027e72f1f1281),
        ("27182818284590452353602874713526e-13", 0x43c2dca375e059b1),
    ];
    for (s, want) in cases {
        assert_eq!(DBig::from_str(s).unwrap().to_f64().value().to_bits(), want, "{s}");
        // the negation flips only the sign bit; cover the sign through the over-wide-quotient path
        let neg = format!("-{s}");
        assert_eq!(
            DBig::from_str(&neg).unwrap().to_f64().value().to_bits(),
            want | (1u64 << 63),
            "{neg}"
        );
    }
}

// A value strictly between half the smallest subnormal and the smallest subnormal keeps no
// significand bit of the target format. It used to be rounded to a single bit first (landing
// on the half-way point) and then rounded again by the encoder, so anything in
// `(2^-1075, 1.5·2^-1075)` came out as zero under round-to-nearest.
#[test]
fn test_to_float_below_smallest_subnormal() {
    use dashu_float::round::mode::Down;

    // (significand, exponent, f64 bits under nearest)
    let f64_cases: [(i64, isize, u64); 8] = [
        (1, -1076, 0), // 1/4: below half
        (3, -1077, 0), // 3/8: below half
        (5, -1077, 1), // 5/8 of the smallest subnormal: above half
        (3, -1076, 1), // 3/4
        (7, -1077, 1), // 7/8
        (1, -1074, 1), // the smallest subnormal itself
        (5, -1076, 1), // 1.25 subnormals
        (3, -1075, 2), // 1.5 subnormals: a tie, to even
    ];
    for (m, e, want) in f64_cases {
        for sign in [1i64, -1] {
            let want = want | if sign < 0 { 1 << 63 } else { 0 };
            let he = FBig::<HalfEven, 2>::from_parts(IBig::from(sign * m), e);
            assert_eq!(he.to_f64().value().to_bits(), want, "HalfEven {sign}*{m}*2^{e}");
            assert_eq!(he.repr().to_f64().value().to_bits(), want, "Repr {sign}*{m}*2^{e}");
            if (m, e) != (3, -1075) {
                let ha = FBig::<HalfAway, 2>::from_parts(IBig::from(sign * m), e);
                assert_eq!(ha.to_f64().value().to_bits(), want, "HalfAway {sign}*{m}*2^{e}");
            }
        }
    }
    let f32_cases: [(i32, isize, u32); 6] = [
        (1, -151, 0),
        (3, -152, 0),
        (5, -152, 1),
        (3, -151, 1),
        (5, -151, 1),
        (3, -150, 2),
    ];
    for (m, e, want) in f32_cases {
        for sign in [1i32, -1] {
            let want = want | if sign < 0 { 1 << 31 } else { 0 };
            let he = FBig::<HalfEven, 2>::from_parts(IBig::from(sign * m), e);
            assert_eq!(he.to_f32().value().to_bits(), want, "HalfEven {sign}*{m}*2^{e}");
            assert_eq!(he.repr().to_f32().value().to_bits(), want, "Repr {sign}*{m}*2^{e}");
        }
    }

    // the exact half-way point is a tie: to even gives zero, away from zero the subnormal
    assert_eq!(FBig::<HalfEven, 2>::from_parts(IBig::ONE, -1075).to_f64(), Inexact(0.0, NoOp));
    assert_eq!(
        FBig::<HalfAway, 2>::from_parts(IBig::ONE, -1075).to_f64(),
        Inexact(f64::from_bits(1), AddOne)
    );
    assert_eq!(
        FBig::<HalfAway, 2>::from_parts(IBig::NEG_ONE, -1075).to_f64(),
        Inexact(-f64::from_bits(1), SubOne)
    );
    assert_eq!(
        FBig::<HalfAway, 2>::from_parts(IBig::ONE, -150).to_f32(),
        Inexact(f32::from_bits(1), AddOne)
    );
    // the rounding flag says which way the value moved
    let x = FBig::<HalfEven, 2>::from_parts(IBig::from(5), -1077);
    assert_eq!(x.to_f64(), Inexact(f64::from_bits(1), AddOne));
    let x = FBig::<HalfEven, 2>::from_parts(IBig::from(3), -1077);
    assert_eq!(x.to_f64(), Inexact(0.0, NoOp));
    // directed modes
    let up = FBig::<Up, 2>::from_parts(IBig::from(3), -1077);
    assert_eq!(up.to_f64(), Inexact(f64::from_bits(1), AddOne));
    let down = FBig::<Down, 2>::from_parts(IBig::from(7), -1077);
    assert_eq!(down.to_f64(), Inexact(0.0, NoOp));
    let zero = FBig::<Zero, 2>::from_parts(IBig::from(-7), -152);
    assert_eq!(zero.to_f32().value().to_bits(), 1 << 31);

    // decimal sources agree with the correctly rounded `str::parse`
    for s in [
        "2e-324", "2.4e-324", "2.5e-324", "3e-324", "4e-324", "4.9e-324", "5e-324", "7e-324",
        "8e-324",
    ] {
        let want: f64 = s.parse().unwrap();
        let he = FBig::<HalfEven, 10>::from_str(s).unwrap();
        assert_eq!(he.to_f64().value().to_bits(), want.to_bits(), "HalfEven {s}");
        let d = DBig::from_str(s).unwrap();
        assert_eq!(d.to_f64().value().to_bits(), want.to_bits(), "DBig {s}");
    }
    for s in [
        "7e-46", "8e-46", "1e-45", "1.4e-45", "1.5e-45", "2e-45", "3e-45",
    ] {
        let want: f32 = s.parse().unwrap();
        let he = FBig::<HalfEven, 10>::from_str(s).unwrap();
        assert_eq!(he.to_f32().value().to_bits(), want.to_bits(), "HalfEven {s}");
        let d = DBig::from_str(s).unwrap();
        assert_eq!(d.to_f32().value().to_bits(), want.to_bits(), "DBig {s}");
    }
    // 2^-1075 + 10^-1075: a wide decimal significand a hair above the half-way point
    let wide = UBig::from(5u8).pow(1075) + UBig::ONE;
    for prec in [0usize, 800] {
        let d = DBig::from_parts(wide.clone().into(), -1075);
        let d = if prec == 0 {
            d
        } else {
            d.with_precision(prec).value()
        };
        assert_eq!(d.to_f64().value().to_bits(), 1, "precision {prec}");
    }

    // an inexact conversion is still not a lossless `TryFrom`
    let x = FBig::<HalfEven, 2>::from_parts(IBig::from(5), -1077);
    assert_eq!(f64::try_from(x), Err(LossOfPrecision));
}

// Between `MAX` and `MAX + ½ulp` round-to-nearest gives `MAX`; only at the half-way point (a
// tie whose even neighbor is `2^emax`) and beyond does the conversion overflow. The range
// check used to send everything above `MAX` to the overflow endpoint.
#[test]
fn test_to_float_just_above_max() {
    use dashu_float::round::mode::{Away, Down};

    // f64::MAX = (2^53 - 1)·2^971; in units of 2^969 that is 2^55 - 4
    let f64_at = |quarters: i64| IBig::from((1i64 << 55) - 4 + quarters);
    let he = |quarters: i64| FBig::<HalfEven, 2>::from_parts(f64_at(quarters), 969);
    assert_eq!(he(-1).to_f64(), Inexact(f64::MAX, AddOne));
    assert_eq!(he(0).to_f64(), Exact(f64::MAX));
    assert_eq!(he(1).to_f64(), Inexact(f64::MAX, NoOp));
    assert_eq!(he(2).to_f64(), Inexact(f64::INFINITY, AddOne)); // the tie
    assert_eq!(he(3).to_f64(), Inexact(f64::INFINITY, AddOne));
    assert_eq!(he(1).repr().to_f64(), Inexact(f64::MAX, NoOp));
    assert_eq!((-he(1)).to_f64(), Inexact(f64::MIN, NoOp));
    assert_eq!((-he(2)).to_f64(), Inexact(f64::NEG_INFINITY, SubOne));
    for quarters in [1, 2, 3] {
        let m = f64_at(quarters);
        assert_eq!(
            FBig::<HalfAway, 2>::from_parts(m.clone(), 969)
                .to_f64()
                .value(),
            if quarters == 1 {
                f64::MAX
            } else {
                f64::INFINITY
            }
        );
        assert_eq!(FBig::<Zero, 2>::from_parts(m.clone(), 969).to_f64().value(), f64::MAX);
        assert_eq!(FBig::<Down, 2>::from_parts(m.clone(), 969).to_f64().value(), f64::MAX);
        assert_eq!(FBig::<Up, 2>::from_parts(m.clone(), 969).to_f64().value(), f64::INFINITY);
        assert_eq!(FBig::<Away, 2>::from_parts(m.clone(), 969).to_f64().value(), f64::INFINITY);
        // out of range for the lossless conversion under every mode
        assert_eq!(
            f64::try_from(FBig::<HalfEven, 2>::from_parts(m.clone(), 969)),
            Err(OutOfBounds)
        );
        assert_eq!(f64::try_from(FBig::<Zero, 2>::from_parts(m.clone(), 969)), Err(OutOfBounds));
        assert_eq!(f64::try_from(FBig::<Up, 2>::from_parts(m, 969)), Err(OutOfBounds));
    }

    // f32::MAX = (2^24 - 1)·2^104; in units of 2^102 that is 2^26 - 4
    let f32_at = |quarters: i64| IBig::from((1i64 << 26) - 4 + quarters);
    let he = |quarters: i64| FBig::<HalfEven, 2>::from_parts(f32_at(quarters), 102);
    assert_eq!(he(-1).to_f32(), Inexact(f32::MAX, AddOne));
    assert_eq!(he(1).to_f32(), Inexact(f32::MAX, NoOp));
    assert_eq!(he(2).to_f32(), Inexact(f32::INFINITY, AddOne));
    assert_eq!(he(1).repr().to_f32(), Inexact(f32::MAX, NoOp));
    assert_eq!(FBig::<Up, 2>::from_parts(f32_at(1), 102).to_f32().value(), f32::INFINITY);
    assert_eq!(f32::try_from(he(1)), Err(OutOfBounds));

    // decimal sources agree with the correctly rounded `str::parse`
    for s in [
        "1.79769313486231570e308",
        "1.79769313486231571e308",
        "1.79769313486231580e308",
        "1.79769313486231581e308",
        "1.8e308",
    ] {
        let want: f64 = s.parse().unwrap();
        let d = DBig::from_str(s).unwrap();
        assert_eq!(d.to_f64().value(), want, "DBig {s}");
        assert_eq!(d.repr().to_f64().value(), want, "Repr {s}");
    }
    for s in ["3.40282346e38", "3.40282356e38", "3.40282357e38", "3.5e38"] {
        let want: f32 = s.parse().unwrap();
        assert_eq!(DBig::from_str(s).unwrap().to_f32().value(), want, "DBig {s}");
    }
}

// Negative zero is the integer 0: its sign lives in a `-1` exponent sentinel, which the integer
// conversions used to read as a fractional part (and the unsigned ones as a negative value).
#[test]
fn test_negative_zero_to_int() {
    use dashu_float::{Context, Repr};

    fn check<R: dashu_float::round::Round, const B: dashu_int::Word>(z: FBig<R, B>) {
        assert!(z.repr().is_neg_zero());
        assert!(z.repr().is_int());
        assert_eq!(z.to_int(), Exact(IBig::ZERO));
        assert_eq!(z.repr().to_int(), Exact(IBig::ZERO));
        assert_eq!(IBig::try_from(z.clone()), Ok(IBig::ZERO));
        assert_eq!(UBig::try_from(z.clone()), Ok(UBig::ZERO));
        assert_eq!(i8::try_from(z.clone()), Ok(0));
        assert_eq!(i32::try_from(z.clone()), Ok(0));
        assert_eq!(i64::try_from(z.clone()), Ok(0));
        assert_eq!(i128::try_from(z.clone()), Ok(0));
        assert_eq!(isize::try_from(z.clone()), Ok(0));
        assert_eq!(u8::try_from(z.clone()), Ok(0));
        assert_eq!(u32::try_from(z.clone()), Ok(0));
        assert_eq!(u64::try_from(z.clone()), Ok(0));
        assert_eq!(u128::try_from(z.clone()), Ok(0));
        assert_eq!(usize::try_from(z.clone()), Ok(0));
        assert_eq!(u64::try_from(z.into_repr()), Ok(0));
    }

    check(DBig::from_str("-0").unwrap());
    check(DBig::from_str("-0.000").unwrap());
    check(-DBig::ZERO);
    check(FBig::<Zero, 2>::try_from(-0.0f64).unwrap());
    check(FBig::<HalfEven, 2>::try_from(-0.0f32).unwrap());
    check(-FBig::<Zero, 2>::ZERO);
    for precision in [20, 50, 100, 500] {
        check(FBig::<HalfEven, 2>::from_repr(Repr::neg_zero(), Context::new(precision)));
        check(FBig::<Up, 3>::from_repr(Repr::neg_zero(), Context::new(precision)));
    }

    // positive zero and ordinary values are unaffected
    assert!(Repr::<2>::zero().is_int());
    assert!(Repr::<10>::neg_one().is_int());
    assert!(!Repr::<10>::new(IBig::from(-5), -1).is_int());
    assert!(!Repr::<2>::infinity().is_int());
    assert_eq!(DBig::ZERO.to_int(), Exact(IBig::ZERO));
    assert_eq!(u8::try_from(DBig::from_str("-1").unwrap()), Err(OutOfBounds));
    assert_eq!(i8::try_from(DBig::from_str("-0.5").unwrap()), Err(LossOfPrecision));
}

// `with_base` documents its precision as the largest p with NewB^p <= B^old, which the old
// floating point estimate missed by one whenever B^old was an exact power of NewB (binary
// precision 128 gave 31 hex digits) and rounded down to 0 — "unlimited", a panic — whenever
// the old significand was narrower than one digit of the new base.
#[test]
fn test_with_base_precision() {
    fn check<const B: dashu_int::Word, const NEWB: dashu_int::Word>() {
        for precision in (1usize..=140).chain([255, 256, 257, 500, 1000]) {
            let x = FBig::<Zero, B>::ONE.with_precision(precision).value();
            let got = x.with_base::<NEWB>().value().precision();
            // the largest p with NEWB^p <= B^precision, but at least one digit
            let limit = UBig::from(B).pow(precision);
            let mut expected = 0;
            let mut power = UBig::from(NEWB);
            while power <= limit {
                expected += 1;
                power *= UBig::from(NEWB);
            }
            assert_eq!(got, expected.max(1), "base {B} -> {NEWB} at precision {precision}");
        }
        // unlimited precision stays unlimited (the value 1 converts exactly)
        assert_eq!(FBig::<Zero, B>::ONE.with_base::<NEWB>().value().precision(), 0);
    }
    check::<2, 2>();
    check::<2, 10>();
    check::<10, 2>();
    check::<2, 16>();
    check::<16, 2>();
    check::<4, 8>();
    check::<8, 4>();
    check::<3, 10>();
    check::<10, 3>();
    check::<10, 100>();
    check::<100, 10>();
    check::<7, 36>();

    // the reported cases
    let x = FBig::<Zero, 2>::ONE.with_precision(128).value();
    assert_eq!(x.clone().with_base::<2>().value().precision(), 128);
    assert_eq!(x.with_base::<16>().value().precision(), 32);
    let y = FBig::<Zero, 2>::from_parts(IBig::from(u128::MAX), 0);
    assert_eq!(y.precision(), 128);
    assert_eq!(
        y.clone().with_base::<16>(),
        Exact(FBig::<Zero, 16>::from_parts(IBig::from(u128::MAX), 0))
    );
    assert_eq!(y.clone().with_base::<2>(), Exact(y));
}

// A binary float with one to three bits of precision used to get zero decimal digits, that is
// unlimited precision, and then panicked because the value was not exactly convertible.
#[test]
fn test_base_change_narrow_precision() {
    for (sig, exp) in [
        (1i64, -1isize),
        (5, -1),
        (3, -2),
        (7, 0),
        (-1, -3),
        (1, -1074),
        (7, -1074),
    ] {
        let a = FBig::<Zero, 2>::from_parts(IBig::from(sig), exp);
        assert!((1..=3).contains(&a.precision()));
        let d = a.clone().with_base::<10>().value();
        assert_eq!(d.precision(), 1);
        assert_eq!(d.repr().digits(), 1);
        // one digit, rounded towards zero: within a factor of two of the input
        let back = d.with_base_and_precision::<2>(60).value();
        let (lo, hi) = if sig > 0 {
            (a.clone() >> 1, a.clone())
        } else {
            (a.clone(), a.clone() >> 1)
        };
        assert!(lo <= back && back <= hi, "{sig} * 2^{exp}");
        assert_eq!(a.to_decimal().value().precision(), 1);
    }
    assert_eq!(
        FBig::<Zero, 2>::from_parts(IBig::from(5), -1).with_base::<10>(),
        Inexact(FBig::<Zero, 10>::from_parts(IBig::from(2), 0), NoOp)
    );
    assert_eq!(
        FBig::<HalfAway, 2>::from_parts(IBig::ONE, -1).with_base::<10>(),
        Exact(dbig!(5e-1))
    );
    // the smallest subnormals, as produced by the float conversions
    for bits in 1u64..=8 {
        let f = FBig::<HalfEven, 2>::try_from(f64::from_bits(bits)).unwrap();
        let d = f.to_decimal().value();
        assert!(d > DBig::ZERO && d.repr().digits() <= d.precision().max(1), "bits {bits}");
    }
    for bits in 1u32..=8 {
        let f = FBig::<HalfEven, 2>::try_from(f32::from_bits(bits)).unwrap();
        assert!(f.to_decimal().value() > DBig::ZERO, "bits {bits}");
    }
}

// A zero converts to a zero of the same sign in every base, at every precision. Negative zero
// used to panic (unlimited precision) or come back positive, depending on the pair of bases.
#[test]
fn test_base_change_zero() {
    use dashu_float::{Context, Repr};

    fn check<const B: dashu_int::Word, const NEWB: dashu_int::Word>() {
        for precision in [0usize, 1, 20, 50, 100, 500] {
            for target in [1usize, 20, 50, 100, 500] {
                let neg = FBig::<Zero, B>::from_repr(Repr::neg_zero(), Context::new(precision));
                let r = neg.clone().with_base_and_precision::<NEWB>(target);
                assert!(matches!(r, Exact(_)), "{B} -> {NEWB}");
                assert!(r.value().repr().is_neg_zero(), "{B} -> {NEWB}");
                let r = neg.with_base::<NEWB>();
                assert!(matches!(r, Exact(_)), "{B} -> {NEWB}");
                assert!(r.value().repr().is_neg_zero(), "{B} -> {NEWB}");

                let pos = FBig::<Zero, B>::from_repr(Repr::zero(), Context::new(precision));
                let r = pos.with_base_and_precision::<NEWB>(target);
                assert!(matches!(r, Exact(_)), "{B} -> {NEWB}");
                assert!(r.value().repr().is_pos_zero(), "{B} -> {NEWB}");
            }
        }
    }
    check::<2, 2>();
    check::<2, 3>();
    check::<2, 4>();
    check::<2, 10>();
    check::<16, 2>();
    check::<12, 2>();
    check::<10, 2>();
    check::<4, 8>();
    check::<3, 7>();

    for mode_neg_zero in [-0.0f64, -0.0f32 as f64] {
        let f = FBig::<HalfAway, 2>::try_from(mode_neg_zero).unwrap();
        assert_eq!(f.precision(), 0);
        let d = f.to_decimal();
        assert!(matches!(d, Exact(_)));
        assert!(d.value().repr().is_neg_zero());
    }
    let z = FBig::<HalfEven, 2>::try_from(-0.0f32).unwrap();
    assert!(z.to_decimal().value().repr().is_neg_zero());
    assert!(DBig::from_str("-0.0000000")
        .unwrap()
        .to_binary()
        .value()
        .repr()
        .is_neg_zero());
    assert!(FBig::<Zero, 2>::try_from(0.0f64)
        .unwrap()
        .to_decimal()
        .value()
        .repr()
        .is_pos_zero());
}

// Converting to the same base only changes the precision, so it has to round like
// `with_precision` does; it used to hand the significand back unrounded and flagged exact.
#[test]
fn test_base_change_same_base() {
    let x = DBig::from_str("11").unwrap();
    assert_eq!(x.clone().with_base_and_precision::<10>(1), Inexact(dbig!(1e1), NoOp));
    assert_eq!(x.clone().with_base_and_precision::<10>(1), x.clone().with_precision(1));
    assert_eq!(x.clone().with_base_and_precision::<10>(2), Exact(x.clone()));
    assert_eq!(x.clone().with_base_and_precision::<10>(0), Exact(x.clone()));
    assert_eq!(x.clone().with_base::<10>(), Exact(x));
    assert_eq!(
        DBig::from_str("-19")
            .unwrap()
            .with_base_and_precision::<10>(1),
        Inexact(dbig!(-2e1), SubOne)
    );

    for precision in [20usize, 50, 100, 500] {
        // 2^(precision + 3) - 1 does not fit: the three low bits are rounded off
        let x = FBig::<Zero, 2>::from_parts((IBig::ONE << (precision + 3)) - 1, -7);
        let r = x.clone().with_base_and_precision::<2>(precision);
        assert_eq!(r, x.clone().with_precision(precision));
        assert!(matches!(r, Inexact(_, NoOp)));
        let v = r.value();
        assert_eq!(v.precision(), precision);
        assert_eq!(v.repr().digits(), precision);
        let x = x.with_rounding::<Up>();
        let r = x.clone().with_base_and_precision::<2>(precision);
        assert_eq!(r, x.with_precision(precision));
        assert!(matches!(r, Inexact(_, AddOne)));
    }

    // infinities stay exact in the same base
    assert_eq!(DBig::INFINITY.with_base_and_precision::<10>(3), Exact(DBig::INFINITY));
}

// Bases that are powers of a common root (but not of each other) convert exactly at every
// exponent, in particular the negative ones that used to go through the inexact path.
#[test]
fn test_base_change_common_root() {
    for exp in [-1000isize, -65, -39, -3, -2, -1, 0, 1, 2, 3, 40, 1001] {
        for sig in [1i32, -3, 7] {
            // sig * 4^exp = sig * 2^(2 exp) = (sig * 2^r) * 8^q with 2 exp = 3 q + r
            let (q, r) = ((2 * exp).div_euclid(3), (2 * exp).rem_euclid(3));
            let x = FBig::<Zero, 4>::from_parts(IBig::from(sig), exp);
            let expected = FBig::<Zero, 8>::from_parts(IBig::from(sig) << r as usize, q);
            assert_eq!(x.clone().with_base_and_precision::<8>(4), Exact(expected.clone()));
            // unlimited precision is fine as well, the conversion is lossless
            let x0 = x.with_precision(0).value();
            assert_eq!(x0.clone().with_base::<8>(), Exact(expected.clone()));
            // and back
            assert_eq!(expected.with_base_and_precision::<4>(4), Exact(x0));
        }
    }
    // rounding still applies when the target precision is short
    let x = FBig::<Zero, 4>::from_parts(IBig::from(0x3f), -50); // six bits set
    assert!(matches!(x.with_base_and_precision::<8>(1), Inexact(_, NoOp)));
}

// Base conversion for exponents beyond what fits a double word (|e| > 38 on 64-bit) estimated
// the result from logarithms at twice the *target* precision, which is nowhere near enough
// when the target precision is short: 2^-39 came out as 4e-12. It is now correctly rounded,
// and reports exactness, at every exponent. The oracles here are exact: the value is built
// directly in the target base when it has a finite expansion there, and obtained from a
// single correctly rounded division otherwise.
#[test]
fn test_base_change_large_exponent() {
    use dashu_float::{
        round::{mode::Down, Round},
        Context, Repr,
    };

    // base 2 -> base 10: m * 2^e is the decimal m * 5^-e * 10^e, or the integer m * 2^e
    fn bin_to_dec<R: Round>(m: i64, e: isize, precision: usize) {
        let exact = if e < 0 {
            FBig::<R, 10>::from_parts(IBig::from(m) * IBig::from(5u8).pow((-e) as usize), e)
        } else {
            FBig::<R, 10>::from_parts(IBig::from(m) << e as usize, 0)
        };
        let expected = exact.with_precision(0).value().with_precision(precision);
        let got =
            FBig::<R, 2>::from_parts(IBig::from(m), e).with_base_and_precision::<10>(precision);
        assert_eq!(got, expected, "{m} * 2^{e} to {precision} decimal digits");
        assert!(got.value().repr().digits() <= precision);
    }
    // base B -> base NEWB through an integer or a single division
    fn by_division<R: Round, const B: dashu_int::Word, const NEWB: dashu_int::Word>(
        m: i64,
        e: isize,
        precision: usize,
    ) {
        let context = Context::<R>::new(precision);
        let power = Repr::<NEWB>::new(UBig::from(B).pow(e.unsigned_abs()).into(), 0);
        let m_repr = Repr::<NEWB>::new(IBig::from(m), 0);
        let expected = if e < 0 {
            context.div(&m_repr, &power)
        } else {
            context.mul(&m_repr, &power)
        }
        .unwrap();
        let got =
            FBig::<R, B>::from_parts(IBig::from(m), e).with_base_and_precision::<NEWB>(precision);
        assert_eq!(got, expected, "{m} * {B}^{e} to {precision} digits of base {NEWB}");
    }
    fn sweep<R: Round>() {
        for precision in [1usize, 2, 3, 20, 50, 100, 500] {
            for e in [
                -20000isize,
                -5000,
                -1000,
                -333,
                -100,
                -65,
                -64,
                -40,
                -39,
                -38,
                -1,
                0,
                1,
                38,
                39,
                40,
                64,
                65,
                100,
                333,
                1000,
                5000,
                20000,
            ] {
                for m in [1i64, -1, 7, 11, 12345, -0x1f_ffff_ffff_ffff] {
                    bin_to_dec::<R>(m, e, precision);
                    by_division::<R, 2, 10>(m, e, precision);
                    by_division::<R, 10, 2>(m, e, precision);
                }
                by_division::<R, 3, 10>(5, e, precision);
                by_division::<R, 10, 3>(-5, e, precision);
                by_division::<R, 12, 18>(7, e, precision);
            }
        }
    }
    sweep::<Zero>();
    sweep::<HalfAway>();
    sweep::<HalfEven>();
    sweep::<Up>();
    sweep::<Down>();

    // the reported cases: 2^-39 = 1.8189...e-12
    type FDec = FBig<Zero, 10>;
    let x = FBin::from_parts(IBig::ONE, -39);
    assert_eq!(
        x.clone().with_base_and_precision::<10>(1),
        Inexact(FDec::from_str("1e-12").unwrap(), NoOp)
    );
    // ... which is exactly 1818989403545856475830078125e-39: 28 digits hold it, and with 27
    // digits the dropped digit is a tie, which rounds away from zero under HalfAway
    assert_eq!(
        x.clone().with_base_and_precision::<10>(28),
        Exact(FDec::from_str("1818989403545856475830078125e-39").unwrap())
    );
    assert_eq!(
        x.with_rounding::<HalfAway>()
            .with_base_and_precision::<10>(27),
        Inexact(DBig::from_str("181898940354585647583007813e-38").unwrap(), AddOne)
    );
    assert_eq!(
        FBin::from_parts(IBig::from(7), -39).with_base_and_precision::<10>(29),
        Exact(FDec::from_str("12732925824820995330810546875e-39").unwrap())
    );
    // 1e-39 = 5.44... * 2^-132
    let d = DBig::from_str("1e-39").unwrap();
    assert_eq!(d.to_binary(), Inexact(FBin::from_parts(IBig::from(5), -132), NoOp));
    assert_eq!(
        d.with_rounding::<Zero>().with_base_and_precision::<2>(4),
        Inexact(FBin::from_parts(IBig::from(10), -133), NoOp)
    );
    // 3^39 = 4052555153018976267
    assert_eq!(
        FBig::<Zero, 3>::from_parts(IBig::ONE, 39).with_base_and_precision::<10>(1),
        Inexact(FDec::from_str("4e18").unwrap(), NoOp)
    );
}

// Exponents far too large for the power of the base to be written out are converted by
// bracketing, in time that depends on the precision and not on the exponent.
#[test]
#[cfg(target_pointer_width = "64")]
fn test_base_change_huge_exponent() {
    use dashu_float::round::mode::Down;

    for precision in [1usize, 20, 50, 100, 500] {
        for e in [
            1_000_000_000_000isize,
            -1_000_000_000_000,
            123_456_789_012_345,
        ] {
            let down = FBig::<Down, 2>::from_parts(IBig::from(3), e)
                .with_base_and_precision::<10>(precision);
            let up = FBig::<Up, 2>::from_parts(IBig::from(3), e)
                .with_base_and_precision::<10>(precision);
            let near = FBig::<HalfEven, 2>::from_parts(IBig::from(3), e)
                .with_base_and_precision::<10>(precision);
            assert!(matches!(down, Inexact(_, NoOp)));
            assert!(matches!(up, Inexact(_, AddOne)));
            let (down, near) = (down.value(), near.value().with_rounding::<Down>());
            let up = up.value().with_rounding::<Down>();
            // the two directed results are neighbors, and the nearest one is one of them
            assert!(down.repr().digits() <= precision);
            assert_eq!(down.clone() + down.ulp(), up);
            assert!(near == down || near == up);
            // log10(3 * 2^e) = e * log10(2) + log10(3)
            let expected = (e as f64) * core::f64::consts::LOG10_2 + 3f64.log10();
            let got = down.repr().exponent() as f64 + down.repr().digits() as f64;
            assert!((got - expected).abs() <= 1.0 + expected.abs() * 1e-12);
        }
    }

    // beyond the exponent range of the target base the result saturates
    let huge = FBig::<Zero, 10>::from_parts(IBig::ONE, isize::MAX / 2);
    assert!(huge
        .with_base_and_precision::<2>(20)
        .value()
        .repr()
        .is_infinite());
    let tiny = FBig::<Zero, 10>::from_parts(IBig::NEG_ONE, isize::MIN / 2);
    assert!(tiny
        .with_base_and_precision::<2>(20)
        .value()
        .repr()
        .is_neg_zero());
}

#[test]
fn test_from_parts_zero_significand() {
    use dashu_base::Sign;
    use dashu_float::Repr;

    // `0 * B^e` is `+0` for every exponent, including the ones that encode `-0` (-1) and the
    // infinities (`isize::MAX` / `isize::MIN`) next to a zero significand.
    let exponents = [
        -2,
        -1,
        0,
        1,
        5,
        isize::MAX - 1,
        isize::MAX,
        isize::MIN + 1,
        isize::MIN,
    ];
    for e in exponents {
        let r2 = Repr::<2>::new(IBig::ZERO, e);
        assert!(r2.is_pos_zero(), "Repr::<2>::new(0, {e}) = {r2:?}");
        let r10 = Repr::<10>::new(IBig::ZERO, e);
        assert!(r10.is_pos_zero(), "Repr::<10>::new(0, {e}) = {r10:?}");
        assert_eq!(r10, Repr::<10>::new_const(Sign::Positive, 0, e));

        let a = FHex::from_parts(IBig::ZERO, e);
        assert!(a.repr().is_pos_zero(), "from_parts(0, {e}) = {a:?}");
        assert_eq!(a.precision(), 1);
        let d = DBig::from_parts(IBig::ZERO, e);
        assert!(d.repr().is_pos_zero(), "from_parts(0, {e}) = {d:?}");
        assert_eq!(d.sign(), Sign::Positive);
        assert_eq!(d.precision(), 1);
        let c = DBig::from_parts_const(Sign::Positive, 0, e, None);
        assert_eq!(d.repr(), c.repr());
    }

    // so the reciprocal of a computed zero is `+inf`, not `-inf`
    let z = FBig::<Zero, 10>::from_parts(IBig::ZERO, -1);
    assert_eq!(FBig::<Zero, 10>::ONE / z, FBig::<Zero, 10>::INFINITY);

    // the special values still survive a trip through their own constructors
    assert!(Repr::<10>::neg_zero().is_neg_zero());
    assert!(Repr::<10>::infinity().is_infinite());
    assert!(Repr::<10>::neg_infinity().is_infinite());
}

#[test]
#[cfg(target_pointer_width = "64")]
fn test_to_float_exponent_beyond_binary_range() {
    use dashu_float::round::mode::Down;

    // The value is finite, but its binary exponent (the decimal one times log2(10), or four
    // times the hexadecimal one) does not fit an `isize`: the base conversion saturates to an
    // infinity, which `to_f32` / `to_f64` have to treat as the overflow it is.
    let dec = DBig::from_parts(IBig::ONE, 3_000_000_000_000_000_000);
    assert_eq!(dec.to_f64(), Inexact(f64::INFINITY, AddOne));
    assert_eq!(dec.to_f32(), Inexact(f32::INFINITY, AddOne));
    assert_eq!(dec.repr().to_f64().value(), f64::INFINITY);
    assert_eq!((-dec.clone()).to_f64().value(), f64::NEG_INFINITY);
    assert_eq!((-dec.clone()).to_f32().value(), f32::NEG_INFINITY);

    let hex = FBig::<HalfEven, 16>::from_parts(IBig::ONE, isize::MAX / 2);
    assert_eq!(hex.to_f64().value(), f64::INFINITY);
    assert_eq!(hex.to_f32().value(), f32::INFINITY);

    // an inward rounding mode saturates at the largest finite float instead
    let down = dec.clone().with_rounding::<Down>();
    assert_eq!(down.to_f64(), Inexact(f64::MAX, NoOp));
    assert_eq!(down.to_f32(), Inexact(f32::MAX, NoOp));
    assert_eq!((-down.clone()).to_f64().value(), f64::NEG_INFINITY);

    // the mirrored tiny values underflow without a panic
    let tiny = DBig::from_parts(IBig::ONE, -3_000_000_000_000_000_000);
    assert_eq!(tiny.to_f64().value(), 0.0);
    assert_eq!(tiny.to_f32().value(), 0.0);
    let tiny = FBig::<HalfEven, 16>::from_parts(IBig::ONE, isize::MIN / 2);
    assert_eq!(tiny.to_f64().value(), 0.0);
}

#[test]
fn test_precision_change_near_max_exponent() {
    use dashu_float::round::mode::{Away, Down};
    use dashu_float::{round::Round, Word};

    // Dropping places raises the exponent. A result that reaches `isize::MAX` or goes beyond
    // it is an overflow, which saturates the way the rounding mode says: a mode that rounds
    // toward zero (or toward the other infinity) never turns a finite number into an
    // infinity, and never returns a value beyond its input.
    fn check<R: Round, const B: Word>(inward_pos: bool, inward_neg: bool) {
        for precision in [1usize, 20, 50, 100, 500] {
            // `precision + 1` places, all of them `B - 1`: rounding drops the lowest one,
            // and rounding away from zero carries into a new place
            let wide = UBig::from_word(B).pow(precision + 1) - UBig::ONE;
            for negative in [false, true] {
                let sig = if negative {
                    -IBig::from(wide.clone())
                } else {
                    IBig::from(wide.clone())
                };
                let inward = if negative { inward_neg } else { inward_pos };

                // the result needs the exponent isize::MAX, or one more after a carry
                let x = FBig::<R, B>::from_parts(sig.clone(), isize::MAX - 1);
                assert!(x.repr().is_finite());
                let r = x.clone().with_precision(precision).value();
                assert_eq!(r.precision(), precision);
                let same_base = x.clone().with_base_and_precision::<B>(precision).value();
                assert_eq!(r.repr(), same_base.repr());
                if inward {
                    assert!(r.repr().is_finite(), "{:?} -> {:?}", x, r);
                    assert_eq!(r.repr().exponent(), isize::MAX);
                    assert_eq!(r.repr().digits(), precision);
                    // toward zero: not beyond the input in magnitude
                    assert!(if negative { r >= x } else { r <= x }, "{:?} -> {:?}", x, r);
                } else {
                    // the carry leaves the range
                    assert!(r.repr().is_infinite(), "{:?} -> {:?}", x, r);
                    assert_eq!(r.repr().sign(), x.repr().sign());
                }

                // Two exponents lower the carry gives `B^(precision + 1)` at the exponent
                // `isize::MAX - 3`, a single place at `isize::MAX - 2 + precision`, which is
                // below the reserved exponent at one place of precision only.
                let x = FBig::<R, B>::from_parts(sig.clone(), isize::MAX - 3);
                let r = x.clone().with_precision(precision).value();
                if inward {
                    assert!(r.repr().is_finite(), "{:?} -> {:?}", x, r);
                    assert_eq!(r.repr().exponent(), isize::MAX - 2);
                    assert!(if negative { r >= x } else { r <= x });
                } else if precision < 2 {
                    assert!(r.repr().is_finite(), "{:?} -> {:?}", x, r);
                    assert_eq!(r.repr().digits(), 1);
                    assert_eq!(r.repr().exponent(), isize::MAX - 2 + precision as isize);
                    assert!(if negative { r <= x } else { r >= x });
                } else {
                    assert!(r.repr().is_infinite(), "{:?} -> {:?}", x, r);
                }

                // an input already at the largest exponent
                let x = FBig::<R, B>::from_parts(sig.clone(), isize::MAX);
                let r = x.clone().with_precision(precision).value();
                if inward {
                    // the largest finite number of that precision
                    assert!(r.repr().is_finite());
                    assert_eq!(r.repr().exponent(), isize::MAX);
                    assert_eq!(r.repr().digits(), precision);
                    assert!(if negative { r >= x } else { r <= x });
                } else {
                    assert!(r.repr().is_infinite());
                }
            }
        }
    }
    check::<Zero, 2>(true, true);
    check::<Down, 2>(true, false);
    check::<Up, 2>(false, true);
    check::<Away, 2>(false, false);
    check::<HalfEven, 2>(false, false);
    check::<HalfAway, 2>(false, false);
    check::<Zero, 10>(true, true);
    check::<Down, 10>(true, false);
    check::<Up, 10>(false, true);
    check::<HalfAway, 10>(false, false);
    check::<Zero, 16>(true, true);

    // the reported cases
    let x = FBig::<Down, 2>::from_parts(IBig::from(3), isize::MAX - 1);
    assert_eq!(
        x.clone().with_precision(1),
        Inexact(FBig::from_parts(IBig::ONE, isize::MAX), NoOp)
    );
    let x = FBig::<Zero, 2>::from_parts(IBig::from(-3), isize::MAX - 1);
    assert_eq!(
        x.clone().with_precision(1),
        Inexact(FBig::from_parts(IBig::NEG_ONE, isize::MAX), NoOp)
    );
    // The rounded value is kept when it is below the largest finite number: 5/4 rounds down
    // to 1, and 3/2 (all ones at two places) would be above the input.
    let x = FBig::<Down, 2>::from_parts(IBig::from(5), isize::MAX - 2);
    assert_eq!(
        x.clone().with_precision(2),
        Inexact(FBig::from_parts(IBig::ONE, isize::MAX), NoOp)
    );
    assert!(x.clone().with_precision(2).value() <= x);
    // a mode that rounds up there overflows to the infinity, as it does in arithmetic
    let x = FBig::<Up, 2>::from_parts(IBig::from(3), isize::MAX - 2);
    assert_eq!(x.with_precision(1), Inexact(FBig::INFINITY.with_precision(1).value(), AddOne));
    let x = FBig::<HalfEven, 2>::from_parts(IBig::from(5), isize::MAX - 2);
    assert!(x.with_precision(2).value().repr().is_infinite());
    // an unchanged precision, or a wider one, leaves any finite value alone
    let x = FBig::<Zero, 2>::from_parts(IBig::from(3), isize::MAX);
    assert_eq!(x.clone().with_precision(2), Exact(x.clone()));
    assert_eq!(x.clone().with_precision(7).value().repr(), x.repr());
}
