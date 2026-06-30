//! Tests for the `rand` integration: the `Standard` distribution (unit square `[0,1)²`) and the
//! `UniformCBig` box sampler. Uses `rand_v08` (the `rand` feature default).

use dashu_cmplx::rand::UniformCBig;
use dashu_cmplx::CBig;
use dashu_float::round::mode::HalfEven;
use rand_v08::distributions::Distribution;
use rand_v08::{rngs::StdRng, Rng, SeedableRng};

type C = CBig<HalfEven, 2>;
type F = dashu_float::FBig<HalfEven, 2>;

#[test]
fn standard_is_in_unit_square() {
    let mut rng = StdRng::seed_from_u64(1);
    for _ in 0..1024 {
        let z: C = rng.gen();
        let (re, im) = z.into_parts();
        // each part uniform in [0, 1) — the unit square
        assert!(re >= F::ZERO && re < F::ONE, "real part {re:?} outside [0,1)");
        assert!(im >= F::ZERO && im < F::ONE, "imag part {im:?} outside [0,1)");
    }
}

#[test]
fn uniform_cbig_box() {
    let mut rng = StdRng::seed_from_u64(7);
    let low = C::from_parts(F::from(2), F::from(-3));
    let high = C::from_parts(F::from(5), F::from(7));
    let dist = UniformCBig::new(&low, &high, 53);
    for _ in 0..1024 {
        let z = dist.sample(&mut rng);
        let (re, im) = z.into_parts();
        // re ∈ [2, 5), im ∈ [-3, 7)
        assert!(re >= F::from(2) && re < F::from(5), "real part {re:?} outside [2,5)");
        assert!(im >= F::from(-3) && im < F::from(7), "imag part {im:?} outside [-3,7)");
    }
}

#[test]
fn open01_excludes_zero() {
    use rand_v08::distributions::Open01;
    let mut rng = StdRng::seed_from_u64(3);
    for _ in 0..256 {
        let z: C = rng.sample(Open01);
        let (re, im) = z.into_parts();
        assert!(re > F::ZERO && im > F::ZERO, "Open01 produced a zero part");
    }
}
