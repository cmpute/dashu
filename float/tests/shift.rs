use dashu_float::DBig;

mod helper_macros;

#[test]
fn test_shift() {
    assert_eq!(fbig!(0x0) << 1, fbig!(0x0));
    assert_eq!(fbig!(0x0) >> 1, fbig!(0x0));
    assert_eq!(fbig!(0x1) << 1, fbig!(0x1p1));
    assert_eq!(fbig!(0x1) >> 1, fbig!(0x1p-1));
    assert_eq!(fbig!(-0x1) << 1, fbig!(-0x1p1));
    assert_eq!(fbig!(-0x1) >> 1, fbig!(-0x1p-1));

    assert_eq!(dbig!(0) << 1, dbig!(0));
    assert_eq!(dbig!(0) >> 1, dbig!(0));
    assert_eq!(dbig!(1) << 1, dbig!(1e1));
    assert_eq!(dbig!(1) >> 1, dbig!(1e-1));
    assert_eq!(dbig!(-1) << 1, dbig!(-1e1));
    assert_eq!(dbig!(-1) >> 1, dbig!(-1e-1));
}

#[test]
#[should_panic]
fn test_shift_inf() {
    let _ = DBig::INFINITY >> 1;
}
