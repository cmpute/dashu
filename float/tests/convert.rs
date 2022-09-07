use dashu_float::DBig;


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
