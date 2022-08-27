use dashu_base::Log2Bounds;

mod helper_macros;

#[test]
fn test_log_ubig() {
    let test_cases = [
        // trivial cases
        (ubig!(1), ubig!(2), 0),
        (ubig!(1), ubig!(10), 0),
        (ubig!(1), ubig!(1000), 0),
        (ubig!(2), ubig!(2), 1),
        (ubig!(3), ubig!(2), 1),
        (ubig!(1) << 100, ubig!(2), 100),
        (ubig!(1) << 1000, ubig!(2), 1000),
        (ubig!(123456), ubig!(123456), 1),
        ((ubig!(1) << 100) - 1u8, (ubig!(1) << 100) - 1u8, 1),
        (ubig!(1) << 1000, ubig!(1) << 1000, 1),
        (ubig!(123456), ubig!(123457), 0),
        (ubig!(1) << 100, ubig!(1) << 101, 0),
        (ubig!(1) << 1000, ubig!(1) << 1001, 0),
        // small bases
        (ubig!(4), ubig!(3), 1),
        (ubig!(10), ubig!(3), 2),
        (ubig!(1) << 20, ubig!(3), 12),
        (ubig!(1) << 50, ubig!(3), 31),
        (ubig!(1) << 100, ubig!(3), 63),
        (ubig!(1) << 500, ubig!(3), 315),
        (ubig!(1) << 5000, ubig!(3), 3154),
        (ubig!(1) << 20, ubig!(10), 6),
        (ubig!(1) << 50, ubig!(10), 15),
        (ubig!(1) << 100, ubig!(10), 30),
        (ubig!(1) << 500, ubig!(10), 150),
        (ubig!(1) << 5000, ubig!(10), 1505),
        ((ubig!(1) << 20) - 1u8, ubig!(31), 4),
        ((ubig!(1) << 50) - 1u8, ubig!(31), 10),
        ((ubig!(1) << 100) - 1u8, ubig!(31), 20),
        ((ubig!(1) << 500) - 1u8, ubig!(31), 100),
        ((ubig!(1) << 5000) - 1u8, ubig!(31), 1009),
        (ubig!(7).pow(11) - 1u8, ubig!(7), 10),
        (ubig!(7).pow(20), ubig!(3).pow(20) + 2u8, 1),
        (ubig!(7).pow(200), ubig!(3).pow(20) + 2u8, 17),
        (ubig!(7).pow(2000), ubig!(3).pow(20) + 2u8, 177),
        (ubig!(7).pow(40), ubig!(3).pow(40) + 2u8, 1),
        (ubig!(7).pow(400), ubig!(3).pow(40) + 2u8, 17),
        (ubig!(7).pow(4000), ubig!(3).pow(40) + 2u8, 177),
        // large bases
        (ubig!(2).pow(4000), ubig!(2).pow(400), 10),
        (ubig!(3).pow(4000), ubig!(2).pow(400), 15),
        (ubig!(5).pow(4000), ubig!(2).pow(400), 23),
        (ubig!(7).pow(4000), ubig!(2).pow(400), 28),
        (ubig!(3).pow(4000), ubig!(3).pow(400), 10),
        (ubig!(5).pow(4000), ubig!(3).pow(400), 14),
        (ubig!(7).pow(4000), ubig!(3).pow(400), 17),
        (ubig!(5).pow(4000), ubig!(5).pow(400), 10),
        (ubig!(7).pow(4000), ubig!(5).pow(400), 12),
        (ubig!(7).pow(4000), ubig!(7).pow(400), 10),
        // large bases with near perfect power
        (ubig!(2).pow(4000) - 1u8, ubig!(2).pow(400), 9),
        (ubig!(3).pow(4000) - 1u8, ubig!(3).pow(400), 9),
        (ubig!(5).pow(4000) - 1u8, ubig!(5).pow(400), 9),
        (ubig!(7).pow(4000) - 1u8, ubig!(7).pow(400), 9),
        (ubig!(2).pow(4000) + 1u8, ubig!(2).pow(400), 10),
        (ubig!(3).pow(4000) + 1u8, ubig!(3).pow(400), 10),
        (ubig!(5).pow(4000) + 1u8, ubig!(5).pow(400), 10),
        (ubig!(7).pow(4000) + 1u8, ubig!(7).pow(400), 10),
    ];
    for (pow, base, exp) in test_cases {
        assert_eq!(pow.ilog(&base), exp, "{}, {}, {}", pow, base, exp);
    }
}

#[test]
fn test_log2_ubig() {
    // log2 should be exact when the result is an inlined integer
    for i in [0, 1, 2, 4, 10, 31] {
        let (lb, ub) = ubig!(2).pow(i).log2_bounds();
        assert_eq!(lb, i as f32);
        assert_eq!(ub, i as f32);
    }

    let test_cases = [
        (ubig!(3), 1.584962500721156),
        (ubig!(5), 2.321928094887362),
        (ubig!(7), 2.807354922057604),
        (ubig!(10), 3.321928094887362),
        ((ubig!(1) << 8) - 1u8, 7.994353436858858),
        ((ubig!(1) << 16) - 1u8, 15.999977986052736),
        ((ubig!(1) << 16) - (ubig!(1) << 11), 15.954196310386875),
        ((ubig!(1) << 32) - 1u8, 31.999999999664098),
        ((ubig!(1) << 32) - (ubig!(1) << 22), 31.998590429745327),
        (ubig!(1) << 50, 50.),
        (ubig!(1) << 100, 100.),
        (ubig!(1) << 5000, 5000.),
        (ubig!(3).pow(5), 7.924812503605781),
        (ubig!(5).pow(7), 16.253496664211536),
        (ubig!(7).pow(11), 30.880904142633646),
        (ubig!(3).pow(4000), 6339.850002884625),
        (ubig!(5).pow(4000), 9287.71237954945),
        (ubig!(7).pow(4000), 11229.419688230417),
        (ubig!(10).pow(4000), 13287.71237954945),
        (ubig!(10).pow(100000), 332192.8094887362),
    ];
    const ERR_BOUND: f64 = 1. / 256.;
    for (n, log2) in test_cases {
        let (lb, ub) = n.log2_bounds();
        let (lb, ub) = (lb as f64, ub as f64);
        assert!(lb <= log2 && (log2 - lb) / log2 < ERR_BOUND);
        assert!(ub >= log2 && (ub - log2) / log2 < ERR_BOUND);
    }
}

#[test]
#[should_panic]
fn test_log_base_0() {
    let _ = ubig!(1234).ilog(&ubig!(0));
}

#[test]
#[should_panic]
fn test_log_base_1() {
    let _ = ubig!(1234).ilog(&ubig!(1));
}

#[test]
#[should_panic]
fn test_log_0() {
    let _ = ubig!(0).ilog(&ubig!(1234));
}
