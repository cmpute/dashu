//! Tests for the logarithm estimations on primitive types.
//!
//! The estimations have separate implementations with and without the `std`
//! feature, run these tests under both configurations.

use dashu_base::EstimatedLog2;

/// Assert that the bounds contain the exact value of the logarithm.
///
/// `reference` must be the f64 nearest to the exact logarithm, the comparison leaves
/// room for its rounding error only (which is far below the resolution of a f32).
fn assert_brackets((lb, ub): (f32, f32), reference: f64) {
    let slack = reference.abs() * 4. * f64::EPSILON;
    assert!(
        lb as f64 <= reference + slack && reference - slack <= ub as f64,
        "({}, {}) does not contain {}",
        lb,
        ub,
        reference
    );
    // the bounds should still be useful
    assert!(
        (ub - lb) as f64 <= reference.abs() * 1e-4 + 2e-2,
        "({}, {}) is too wide",
        lb,
        ub
    );
}

#[test]
fn test_u128_log2_est_near_max() {
    // these values round to 2^128 when converted to f32
    let below = u128::MAX - (1u128 << 103);
    for x in [below, below + 1, u128::MAX - 1, u128::MAX] {
        let est = x.log2_est();
        let (lb, ub) = x.log2_bounds();
        assert!(est.is_finite(), "{:#x}: log2_est = {}", x, est);
        assert!(lb <= est && est <= ub, "{:#x}: {} not in ({}, {})", x, est, lb, ub);
        // without std the estimate is the midpoint of the (coarser) table based bounds
        assert!((est - 128.).abs() < 2e-2);
    }
    assert!((i128::MIN.log2_est() - 127.).abs() < 2e-2);
    assert!((u64::MAX.log2_est() - 64.).abs() < 2e-2);
}

#[test]
fn test_float_log2_bounds_subnormal() {
    // log2(3), log2(5), log2(7)
    const LOG2_3: f64 = 1.584962500721156;
    const LOG2_5: f64 = 2.321928094887362;
    const LOG2_7: f64 = 2.807354922057604;

    for (man, log2_man) in [(3u32, LOG2_3), (5, LOG2_5), (7, LOG2_7), (6, LOG2_3 + 1.)] {
        assert_brackets(f32::from_bits(man).log2_bounds(), log2_man - 149.);
        assert_brackets((-f32::from_bits(man)).log2_bounds(), log2_man - 149.);
        assert_brackets(f64::from_bits(man as u64).log2_bounds(), log2_man - 1074.);
        assert_brackets((man as f32).log2_bounds(), log2_man);
        assert_brackets((man as f64).log2_bounds(), log2_man);
        assert_brackets((man as f64 * 2f64.powi(900)).log2_bounds(), log2_man + 900.);
        assert_brackets((man as f32 * 2f32.powi(-100)).log2_bounds(), log2_man - 100.);
    }

    // exact powers of two
    for f in [
        f32::from_bits(1),
        f32::from_bits(4),
        f32::MIN_POSITIVE,
        1.,
        2f32.powi(100),
    ] {
        let (lb, ub) = f.log2_bounds();
        assert!(lb <= ub);
        assert_brackets((lb, ub), (f as f64).log2());
    }
    assert_brackets(f64::from_bits(1).log2_bounds(), -1074.);
    assert_brackets(f64::MIN_POSITIVE.log2_bounds(), -1022.);
}
