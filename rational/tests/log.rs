use dashu_base::EstimatedLog2;
use dashu_ratio::{RBig, Relaxed};

mod helper_macros;

#[test]
fn test_log2_bounds_contain_log2() {
    // the bounds of the numerator and the denominator are combined by a rounded
    // subtraction, which must not push a bound past the true value
    let mut cases = vec![
        (3u64, 1024u64),
        (3, 1),
        (1, 3),
        (1, 1024),
        (1024, 3),
        (5, 7),
    ];
    for den in [
        1u64,
        2,
        3,
        7,
        10,
        1024,
        1025,
        1 << 20,
        999_983,
        1 << 40,
        (1 << 40) + 1,
    ] {
        for num in [
            1u64,
            2,
            3,
            5,
            6,
            7,
            100,
            255,
            256,
            257,
            65_537,
            1 << 30,
            (1 << 52) + 1,
        ] {
            cases.push((num, den));
        }
    }
    for (num, den) in cases {
        let truth = (num as f64).log2() - (den as f64).log2();
        // tolerance for the error of the f64 reference itself, far below the f32 resolution
        let slack = 1e-12 * truth.abs().max(1.);
        let r = RBig::from_parts(num.into(), den.into());
        let x = Relaxed::from_parts(num.into(), den.into());
        for (name, (lb, ub)) in [
            ("RBig", r.log2_bounds()),
            ("Relaxed", x.log2_bounds()),
            ("-RBig", (-r.clone()).log2_bounds()),
        ] {
            assert!(
                lb as f64 <= truth + slack && truth - slack <= ub as f64,
                "{name} {num}/{den}: ({lb:?}, {ub:?}) does not contain {truth:?}"
            );
            // the bounds stay tight
            assert!(ub - lb <= 1e-5 * (truth.abs() as f32).max(1.), "{name} {num}/{den}");
        }
        let est = r.log2_est() as f64;
        assert!((est - truth).abs() <= 1e-5 * truth.abs().max(1.), "{num}/{den}: {est}");
    }

    // exactly representable logarithms keep exact bounds
    assert_eq!(rbig!(1).log2_bounds(), (0., 0.));
    assert_eq!(rbig!(1 / 1024).log2_bounds(), (-10., -10.));
    assert_eq!(rbig!(~4096/2).log2_bounds(), (11., 11.));
    assert_eq!(rbig!(0).log2_bounds(), (f32::NEG_INFINITY, f32::NEG_INFINITY));
}
