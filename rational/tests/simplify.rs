use dashu_ratio::RBig;

mod helper_macros;

#[test]
fn test_simpliest_in() {
    assert_eq!(RBig::simpliest_in(rbig!(0), rbig!(0)), rbig!(0));
    assert_eq!(RBig::simpliest_in(rbig!(-1), rbig!(1)), rbig!(0));
    assert_eq!(RBig::simpliest_in(rbig!(-1), rbig!(-1)), rbig!(-1));
    assert_eq!(RBig::simpliest_in(rbig!(2/7), rbig!(2/9)), rbig!(1/4));
    assert_eq!(RBig::simpliest_in(rbig!(-20/7), rbig!(-20/9)), rbig!(-5/2));
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
