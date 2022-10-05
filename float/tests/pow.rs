use dashu_float::FBig;
use dashu_float::round::{mode::{HalfAway, Zero}, Rounding::*};

type Float10 = FBig<Zero,10>;
#[test]
fn pow_test() {
    let base = Float10::try_from(2)
        .unwrap()
        .with_precision(4)
        .value();
    let pow = Float10::try_from(3)
        .unwrap()
        .with_precision(4)
        .value();
    let res = base.pow(pow);
    let epsilon = Float10::from_str_native("0.004")
        .unwrap()
        .with_precision(4)
        .value();
    assert_eq!(res+epsilon,Float10::try_from(8).unwrap());
}
