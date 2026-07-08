use dashu_float::DBig;
type FBig = dashu_float::FBig;

mod helper_macros;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_sum() {
    let nums = [
        fbig!(-0x1p2),
        fbig!(-0x1p1),
        fbig!(-0x1),
        fbig!(-0x1p-1),
        fbig!(-0x1p-2),
        fbig!(0),
        fbig!(0x1p-2),
        fbig!(0x1p-1),
        fbig!(0x1),
        fbig!(0x1p1),
        fbig!(0x1p2),
    ];

    assert_eq!(nums[..0].iter().sum::<FBig>(), fbig!(0));
    assert_eq!(nums[..1].iter().sum::<FBig>(), fbig!(-0x1p2));
    assert_eq!(nums[..2].iter().sum::<FBig>(), fbig!(-0x3p1));
    assert_eq!(nums[..4].iter().sum::<FBig>(), fbig!(-0xfp-1));
    // The full set is symmetric around zero, so the correctly-rounded sum is exactly 0. (The previous
    // per-step fold accumulated rounding and reported 0x1p-2 here.)
    assert_eq!(nums.iter().sum::<FBig>(), fbig!(0));
    assert_eq!(nums.into_iter().sum::<FBig>(), fbig!(0));

    let nums = [
        dbig!(-0001e2),
        dbig!(-1e1),
        dbig!(-1),
        dbig!(-1e-1),
        dbig!(-1e-2),
        dbig!(0),
        dbig!(1e-2),
        dbig!(1e-1),
        dbig!(1),
        dbig!(1e1),
        dbig!(1e2),
    ];

    assert_eq!(nums[..0].iter().sum::<DBig>(), dbig!(0));
    assert_eq!(nums[..1].iter().sum::<DBig>(), dbig!(-1e2));
    assert_eq!(nums[..2].iter().sum::<DBig>(), dbig!(-11e1));
    assert_eq!(nums[..4].iter().sum::<DBig>(), dbig!(-1111e-1));
    // Symmetric around zero → the correctly-rounded sum is exactly 0 (the previous fold gave 1e-2).
    assert_eq!(nums.iter().sum::<DBig>(), dbig!(0));
    assert_eq!(nums.into_iter().sum::<DBig>(), dbig!(0));
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_prod() {
    let nums = [
        fbig!(-0x1p2),
        fbig!(0x1p1),
        fbig!(-0x1),
        fbig!(0x1p-1),
        fbig!(-0x1p-2),
        fbig!(0),
    ];

    assert_eq!(nums[..0].iter().product::<FBig>(), fbig!(0x1));
    assert_eq!(nums[..1].iter().product::<FBig>(), fbig!(-0x1p2));
    assert_eq!(nums[..2].iter().product::<FBig>(), fbig!(-0x1p3));
    assert_eq!(nums[..4].iter().product::<FBig>(), fbig!(0x1p2));
    assert_eq!(nums.iter().product::<FBig>(), fbig!(0));
    assert_eq!(nums.into_iter().product::<FBig>(), fbig!(0));

    let nums = [
        dbig!(-0001e2),
        dbig!(1e1),
        dbig!(-1),
        dbig!(1e-1),
        dbig!(-1e-2),
        dbig!(0),
    ];

    assert_eq!(nums[..0].iter().product::<DBig>(), dbig!(1));
    assert_eq!(nums[..1].iter().product::<DBig>(), dbig!(-1e2));
    assert_eq!(nums[..2].iter().product::<DBig>(), dbig!(-1e3));
    assert_eq!(nums[..4].iter().product::<DBig>(), dbig!(1e2));
    assert_eq!(nums.iter().product::<DBig>(), dbig!(0));
    assert_eq!(nums.into_iter().product::<DBig>(), dbig!(0));
}
