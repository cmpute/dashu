use dashu_int::UBig;
use dashu_ratio::{RBig, Relaxed};
use zeroize::Zeroize;

mod helper_macros;

#[test]
fn test_zeroize_gives_zero() {
    // the zeroized value is a valid zero (0/1), not 0/0
    let mut values = [
        rbig!(3 / 4),
        rbig!(-3 / 4),
        rbig!(5),
        rbig!(0),
        RBig::from_parts(ibig!(-1) << 200, (ubig!(1) << 200) - ubig!(1)),
    ];
    for v in values.iter_mut() {
        v.zeroize();
        assert_eq!(*v, RBig::ZERO);
        assert_eq!(v.numerator(), &ibig!(0));
        assert_eq!(v.denominator(), &UBig::ONE);
        assert_eq!(&*v + rbig!(1), rbig!(1));
        assert_eq!(&*v * rbig!(7), rbig!(0));
        assert_eq!(rbig!(1 / 2) - &*v, rbig!(1 / 2));
        assert_eq!(v.to_string(), "0");
        assert_eq!(v.to_f64().value(), 0.0);
    }

    let mut values = [
        rbig!(~3/4),
        rbig!(~-9/15),
        rbig!(~0),
        Relaxed::from_parts(ibig!(1) << 200, (ubig!(1) << 200) - ubig!(1)),
    ];
    for v in values.iter_mut() {
        v.zeroize();
        assert_eq!(*v, Relaxed::ZERO);
        assert_eq!(v.denominator(), &UBig::ONE);
        assert_eq!(&*v + rbig!(~1), rbig!(~1));
        assert_eq!(&*v * rbig!(~7), rbig!(~0));
        assert_eq!(v.to_string(), "0");
    }
}
