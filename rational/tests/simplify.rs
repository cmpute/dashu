use dashu_base::{Approximation::*, Sign::*};
use dashu_ratio::RBig;

mod helper_macros;

#[test]
fn test_simpliest_in() {
    assert_eq!(RBig::simpliest_in(rbig!(0), rbig!(0)), rbig!(0));
    assert_eq!(RBig::simpliest_in(rbig!(-1), rbig!(1)), rbig!(0));
    assert_eq!(RBig::simpliest_in(rbig!(-1), rbig!(-1)), rbig!(-1));
    assert_eq!(RBig::simpliest_in(rbig!(2/7), rbig!(2/9)), rbig!(1/4));
    assert_eq!(RBig::simpliest_in(rbig!(-20/7), rbig!(-20/9)), rbig!(-5/2));
    assert_eq!(RBig::simpliest_in(rbig!(5), rbig!(7)), rbig!(6));
    assert_eq!(RBig::simpliest_in(rbig!(5), rbig!(6)), rbig!(11/2));
}

#[test]
fn test_simpliest_from_f32() {
    assert_eq!(RBig::simpliest_from_f32(0f32), Some(rbig!(0)));
    assert_eq!(RBig::simpliest_from_f32(f32::INFINITY), None);
    assert_eq!(RBig::simpliest_from_f32(f32::NEG_INFINITY), None);
    assert_eq!(RBig::simpliest_from_f32(f32::NAN), None);

    let cases = [
        // (numerator, denominator)
        // NOTE: make sure each of these numbers fit in a f32
        (1i32, 1u32),
        (2, 1),
        (-3, 1),
        (1, 2),
        (-1, 3),
        (1, 4),
        (-1, 5),

        // convergents of pi
        (22, 7),
        (333, 106),
        (355, 113),
    ];
    for (num, den) in cases {
        let f = num as f32 / den as f32;
        assert_eq!(RBig::simpliest_from_f32(f).unwrap(), RBig::from_parts(num.into(), den.into()));
    }
}

#[test]
fn test_simpliest_from_f64() {
    assert_eq!(RBig::simpliest_from_f64(0f64), Some(rbig!(0)));
    assert_eq!(RBig::simpliest_from_f64(f64::INFINITY), None);
    assert_eq!(RBig::simpliest_from_f64(f64::NEG_INFINITY), None);
    assert_eq!(RBig::simpliest_from_f64(f64::NAN), None);

    let cases = [
        // (numerator, denominator)
        // NOTE: make sure each of these numbers fit in a f64
        (1i64, 1u64),
        (2, 1),
        (-3, 1),
        (1, 2),
        (-1, 3),
        (1, 4),
        (-1, 5),

        // convergents of pi
        (22, 7),
        (333, 106),
        (355, 113),
        (103993, 33102),
        (104348, 33215),
        (208341, 66317),
        (312689, 99532),
        (833719, 265381),
        (1146408, 364913),
        (4272943, 1360120),
        (5419351, 1725033),
        (80143857, 25510582),
        (165707065, 52746197),
        (245850922, 78256779)
    ];
    for (num, den) in cases {
        let f = num as f64 / den as f64;
        assert_eq!(RBig::simpliest_from_f64(f).unwrap(), RBig::from_parts(num.into(), den.into()));
    }
}

#[test]
fn test_nearest() {
    // test trivial cases
    assert_eq!(rbig!(0).nearest(&ubig!(1)), Exact(rbig!(0)));
    assert_eq!(rbig!(1).nearest(&ubig!(2)), Exact(rbig!(1)));
    assert_eq!(rbig!(-1).nearest(&ubig!(3)), Exact(rbig!(-1)));

    // test general cases
    let test_cases = [
        // (value, limit, next down, next up, sign of nearest - value)
        (core::f64::consts::PI, 10u64, rbig!(25/8), rbig!(22/7), Positive),
        (core::f64::consts::PI, 100, rbig!(311/99), rbig!(22/7), Negative),
        (core::f64::consts::PI, 1000, rbig!(2818/897), rbig!(355/113), Positive),
        (core::f64::consts::PI, 10000, rbig!(31218/9937), rbig!(355/113), Positive),
        (core::f64::consts::PI, 100000, rbig!(208341/66317), rbig!(312689/99532), Positive),
        (core::f64::consts::SQRT_2, 10, rbig!(7/5), rbig!(10/7), Negative),
        (core::f64::consts::SQRT_2, 100, rbig!(140/99), rbig!(99/70), Negative),
        (core::f64::consts::SQRT_2, 1000, rbig!(1393/985), rbig!(577/408), Negative),
        (core::f64::consts::SQRT_2, 10000, rbig!(8119/5741), rbig!(11482/8119), Positive),
        (core::f64::consts::SQRT_2, 100000, rbig!(47321/33461), rbig!(114243/80782), Positive),
    ];
    for (value, limit, down, up, cmp) in test_cases {
        let ratio: RBig = value.try_into().unwrap();
        assert_eq!(ratio.next_up(&limit.into()), up.clone());
        assert_eq!(ratio.next_down(&limit.into()), down.clone());
        if cmp == Positive {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(up.clone(), cmp));
        } else {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(down.clone(), cmp));
        }

        let ratio = -ratio;
        assert_eq!(ratio.next_down(&limit.into()), -up.clone());
        assert_eq!(ratio.next_up(&limit.into()), -down.clone());
        if cmp == Positive {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(-up.clone(), -cmp));
        } else {
            assert_eq!(ratio.nearest(&limit.into()), Inexact(-down.clone(), -cmp));
        }
    }
}

#[test]
#[should_panic]
fn test_nearest_zero_limit() {
    let _ = rbig!(1/2).nearest(&ubig!(0));
}
