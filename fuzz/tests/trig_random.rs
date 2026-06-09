use core::str::FromStr;
use dashu_float::math::FpResult;
use dashu_float::ops::Abs;
use dashu_float::round::mode::HalfEven;
use dashu_float::{DBig, FBig};
use rand::prelude::*;
use rug::Float;

/// Reproduction case for a bug discovered during fuzzing where very small
/// numbers with many digits triggered an assertion failure in the rounding logic.
#[test]
#[ignore]
fn test_reproduce_assertion_failure() {
    let x_str = "-5.525474318981006776603409487767135633516667011547942409467e-3";
    let prec = 100;
    let x_dashu = DBig::from_str(x_str).unwrap().with_rounding::<HalfEven>();
    let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);
    let _sin_d = dashu_ctx.sin(x_dashu.repr()).value(&dashu_ctx);
}

#[test]
#[ignore]
fn test_pi_fuzz() {
    for prec in (10..10000).step_by(37) {
        let pi_dashu = DBig::pi(prec).with_rounding::<HalfEven>();
        let bits = (prec * 3322).div_ceil(1000) + 32;
        let pi_rug = Float::with_val(bits as u32, rug::float::Constant::Pi);
        let s_r_val = DBig::from_str(&pi_rug.to_string_radix(10, Some(prec)))
            .unwrap()
            .with_rounding::<HalfEven>();
        assert_eq!(
            pi_dashu, s_r_val,
            "Pi mismatch at prec={prec}: dashu={}, rug={}",
            pi_dashu, s_r_val
        );
    }
}

#[test]
#[ignore]
fn test_pi_fuzz_concurrent() {
    use std::thread;

    let mut handles = vec![];

    // Spawn 16 threads
    for thread_id in 0..16 {
        let handle = thread::spawn(move || {
            // Seed a local random generator so each thread tests different precisions
            let mut rng = rand::rngs::StdRng::seed_from_u64(thread_id as u64 + 42);
            for _ in 0..5000 {
                // Random precision between 10 and 5000
                let prec = rng.random_range(10..5000);

                let pi_dashu = DBig::pi(prec).with_rounding::<HalfEven>();
                let bits = (prec * 3322).div_ceil(1000) + 32;
                let pi_rug = Float::with_val(bits as u32, rug::float::Constant::Pi);
                let s_r_val = DBig::from_str(&pi_rug.to_string_radix(10, Some(prec)))
                    .unwrap()
                    .with_rounding::<HalfEven>();

                assert_eq!(
                    pi_dashu, s_r_val,
                    "Pi mismatch at prec={prec} (thread {thread_id}): dashu={}, rug={}",
                    pi_dashu, s_r_val
                );
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

/// Generates a truly arbitrary `DBig` value for testing.
fn random_dbig<R: Rng>(rng: &mut R, large_exp: bool) -> DBig {
    let sign = if rng.random_bool(0.5) { 1 } else { -1 };
    let num_digits = rng.random_range(1..100);
    let mut s = String::new();
    if sign == -1 {
        s.push('-');
    }
    for _ in 0..num_digits {
        s.push(char::from_digit(rng.random_range(0..10), 10).unwrap());
    }
    let exponent = if large_exp {
        rng.random_range(-2000..2000)
    } else {
        rng.random_range(-10..10)
    };
    s.push_str(&format!("e{exponent}"));
    DBig::from_str(&s).unwrap_or(DBig::ZERO)
}

#[test]
#[ignore]
fn test_trig_fuzz_comprehensive() {
    let mut rng = StdRng::seed_from_u64(42);
    let precisions = [10, 20, 50, 100];

    for i in 0..2000 {
        let x_dashu = random_dbig(&mut rng, true).with_rounding::<HalfEven>();
        let x_str = format!("{x_dashu:e}");

        for &prec in &precisions {
            let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);
            let x_f_repr = x_dashu.repr().clone();

            // Sin
            let sin_d =
                match std::panic::catch_unwind(|| dashu_ctx.sin(&x_f_repr).value(&dashu_ctx)) {
                    Ok(v) => v,
                    Err(_) => {
                        panic!("PANIC at iteration {i}, prec {prec}, x = {x_str}");
                    }
                };

            // Rug baseline
            let x_bits = ((x_dashu.repr().exponent().abs() as f64 * 3.322).ceil() as u32) + 500;
            let bits = (((prec as f64).max(100.0) * 3.322).ceil() as u32) + x_bits;
            let x_rug = match Float::parse(&x_str) {
                Ok(parsed) => Float::with_val(bits, parsed),
                Err(_) => continue,
            };

            let sin_r = x_rug.clone().sin();
            let s_r_val = DBig::from_str(&sin_r.to_string_radix(10, Some(prec)))
                .unwrap()
                .with_rounding::<HalfEven>();
            assert!(
                (sin_d.clone() - s_r_val).abs()
                    <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
                "Sin mismatch at iteration {i}, x={x_str}, prec={prec}: dashu={sin_d}, rug={sin_r}"
            );
        }
    }
}

#[test]
#[ignore]
fn test_atan2_fuzz_comprehensive() {
    let mut rng = StdRng::seed_from_u64(45);
    let precisions = [20, 50];

    for i in 0..500 {
        let y_dashu = random_dbig(&mut rng, true).with_rounding::<HalfEven>();
        let x_dashu = random_dbig(&mut rng, true).with_rounding::<HalfEven>();
        let y_str = format!("{y_dashu:e}");
        let x_str = format!("{x_dashu:e}");

        for &prec in &precisions {
            let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);

            let atan2_d = std::panic::catch_unwind(|| {
                dashu_ctx
                    .atan2(y_dashu.repr(), x_dashu.repr())
                    .value(&dashu_ctx)
            })
            .unwrap_or_else(|_| {
                panic!("PANIC at iteration {i}, prec {prec}, y = {y_str}, x = {x_str}");
            });

            let bits = (u32::try_from(prec).unwrap() * 4) + 1000;
            let y_rug = Float::with_val(bits, Float::parse(&y_str).unwrap());
            let x_rug = Float::with_val(bits, Float::parse(&x_str).unwrap());
            let atan2_r = y_rug.atan2(&x_rug);

            let a_r_val = DBig::from_str(&atan2_r.to_string_radix(10, Some(prec)))
                .unwrap()
                .with_rounding::<HalfEven>();
            assert!(
                (atan2_d.clone() - a_r_val).abs()
                    <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
                "Atan2 mismatch at iteration {i}, y={y_str}, x={x_str}, prec={prec}: dashu={atan2_d}, rug={atan2_r}"
            );
        }
    }
}

/// Generates a random `DBig` within [min, max] range.
fn random_dbig_range<R: Rng>(rng: &mut R, min: f64, max: f64) -> DBig {
    let val: f64 = rng.random_range(min..max);
    DBig::from_str(&format!("{val:.15}")).unwrap()
}

#[test]
#[ignore]
fn test_inv_trig_fuzz() {
    let mut rng = StdRng::seed_from_u64(43);
    let precisions = [20, 50];

    for i in 0..200 {
        // Test asin/acos within [-1, 1]
        let x_dashu = random_dbig_range(&mut rng, -1.0, 1.0).with_rounding::<HalfEven>();
        let x_str = format!("{x_dashu:e}");

        for &prec in &precisions {
            let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);

            // Asin
            let asin_d = dashu_ctx.asin(x_dashu.repr()).value(&dashu_ctx);
            let bits = (u32::try_from(prec).unwrap() * 4) + 128;
            let x_rug = Float::with_val(bits, Float::parse(&x_str).unwrap());
            let asin_r = x_rug.clone().asin();
            let a_r_val = DBig::from_str(&asin_r.to_string_radix(10, Some(prec)))
                .unwrap()
                .with_rounding::<HalfEven>();
            assert!(
                (asin_d.clone() - a_r_val).abs()
                    <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
                "Asin mismatch at iteration {i}, x={x_str}, prec={prec}: dashu={asin_d}, rug={asin_r}"
            );

            // Acos
            let acos_d = dashu_ctx.acos(x_dashu.repr()).value(&dashu_ctx);
            let acos_r = x_rug.acos();
            let a_r_val = DBig::from_str(&acos_r.to_string_radix(10, Some(prec)))
                .unwrap()
                .with_rounding::<HalfEven>();
            assert!(
                (acos_d.clone() - a_r_val).abs()
                    <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
                "Acos mismatch at iteration {i}, x={x_str}, prec={prec}: dashu={acos_d}, rug={acos_r}"
            );
        }
    }
}

#[test]
#[ignore]
fn test_edge_cases_fuzz() {
    let mut rng = StdRng::seed_from_u64(46);
    let precisions = [30, 100];

    for _ in 0..50 {
        for &prec in &precisions {
            let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);

            // Numbers very close to 1.0 (test asin/acos precision)
            let epsilon = 10.0f64.powi(-(rng.random_range(1..15)));
            let x_val = 1.0 - epsilon;
            let x_dashu = DBig::from_str(&format!("{x_val:.16}"))
                .unwrap()
                .with_rounding::<HalfEven>();
            let x_str = format!("{x_dashu:e}");

            let asin_d = dashu_ctx.asin(x_dashu.repr()).value(&dashu_ctx);
            let bits = (u32::try_from(prec).unwrap() * 4) + 256;
            let x_rug = Float::with_val(bits, Float::parse(&x_str).unwrap());
            let asin_r = x_rug.asin();
            let a_r_val = DBig::from_str(&asin_r.to_string_radix(10, Some(prec)))
                .unwrap()
                .with_rounding::<HalfEven>();
            assert!(
                (asin_d.clone() - a_r_val).abs()
                    <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
                "Edge Asin mismatch: x={x_str}, prec={prec}"
            );
        }
    }
}

#[test]
#[ignore]
fn test_tan_large_exponent_regression() {
    let x_str = "-3.67225387623341113999117300261402819219640608e511";
    for prec in [20usize, 50] {
        let x_dashu = DBig::from_str(x_str).unwrap().with_rounding::<HalfEven>();
        let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);
        let tan_d = dashu_ctx.tan(x_dashu.repr()).value(&dashu_ctx);

        let bits = (u32::try_from(prec).unwrap() * 4) + 512 + 1700; // extra bits for large exponent
        let x_rug = Float::with_val(bits, Float::parse(x_str).unwrap());
        let tan_r = x_rug.tan();
        let t_r_val = DBig::from_str(&tan_r.to_string_radix(10, Some(prec)))
            .unwrap()
            .with_rounding::<HalfEven>();
        assert!(
            (tan_d.clone() - t_r_val).abs()
                <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
            "Large-exponent tan regression failed at prec={prec}: dashu={tan_d}, rug={tan_r}"
        );
    }
}

#[test]
#[ignore]
fn test_pythagorean_identity_fuzz() {
    let mut rng = StdRng::seed_from_u64(99);
    let precisions = [20usize, 50, 100];

    for i in 0..1000 {
        let x_dashu = random_dbig(&mut rng, true).with_rounding::<HalfEven>();

        for &prec in &precisions {
            let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);
            let (s, c) = dashu_ctx.sin_cos(x_dashu.repr());
            if let (FpResult::Normal(s_r), FpResult::Normal(c_r)) = (s, c) {
                let s_f = FBig::from_repr(s_r.value(), dashu_ctx);
                let c_f = FBig::from_repr(c_r.value(), dashu_ctx);
                let sum = s_f.clone() * &s_f + c_f.clone() * &c_f;
                let one = DBig::ONE
                    .with_precision(prec)
                    .value()
                    .with_rounding::<HalfEven>();
                assert!(
                    (sum.clone() - one).abs()
                        <= DBig::from_parts(1000.into(), -(isize::try_from(prec).unwrap())),
                    "sin²+cos²≠1 at iteration {i}, prec={prec}, x={x_dashu:e}, sum={sum}"
                );
            }
        }
    }
}

#[test]
#[ignore]
fn test_cos_fuzz_comprehensive() {
    let mut rng = StdRng::seed_from_u64(47);
    let precisions = [10usize, 20, 50, 100];

    for i in 0..2000 {
        let x_dashu = random_dbig(&mut rng, true).with_rounding::<HalfEven>();
        let x_str = format!("{x_dashu:e}");

        for &prec in &precisions {
            let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);
            let cos_d =
                std::panic::catch_unwind(|| dashu_ctx.cos(x_dashu.repr()).value(&dashu_ctx))
                    .unwrap_or_else(|_| panic!("PANIC at iteration {i}, prec {prec}, x = {x_str}"));

            let x_bits = ((x_dashu.repr().exponent().abs() as f64 * 3.322).ceil() as u32) + 500;
            let bits = (((prec as f64).max(100.0) * 3.322).ceil() as u32) + x_bits;
            let x_rug = match Float::parse(&x_str) {
                Ok(parsed) => Float::with_val(bits, parsed),
                Err(_) => continue,
            };
            let cos_r = x_rug.cos();
            let c_r_val = DBig::from_str(&cos_r.to_string_radix(10, Some(prec)))
                .unwrap()
                .with_rounding::<HalfEven>();
            assert!(
                (cos_d.clone() - c_r_val).abs()
                    <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
                "Cos mismatch at iteration {i}, x={x_str}, prec={prec}: dashu={cos_d}, rug={cos_r}"
            );
        }
    }
}

#[test]
#[ignore]
fn test_tan_fuzz_strict() {
    let mut rng = StdRng::seed_from_u64(44);
    let precisions = [20usize, 50];

    for i in 0..500 {
        let x_dashu = random_dbig(&mut rng, true).with_rounding::<HalfEven>();
        let x_str = format!("{x_dashu:e}");

        for &prec in &precisions {
            let dashu_ctx = dashu_float::Context::<HalfEven>::new(prec);

            // Only skip if we can verify it's actually near a singularity (|cos| < 10^-5)
            let cos_d = dashu_ctx.cos(x_dashu.repr()).value(&dashu_ctx);
            if cos_d.abs() < DBig::from_parts(1.into(), -5).with_rounding::<HalfEven>() {
                continue;
            }

            let tan_d =
                std::panic::catch_unwind(|| dashu_ctx.tan(x_dashu.repr()).value(&dashu_ctx))
                    .unwrap_or_else(|_| panic!("PANIC at iteration {i}, prec {prec}, x = {x_str}"));

            let x_bits = ((x_dashu.repr().exponent().abs() as f64 * 3.322).ceil() as u32) + 500;
            let bits = (((prec as f64).max(100.0) * 3.322).ceil() as u32) + x_bits;
            let x_rug = match Float::parse(&x_str) {
                Ok(parsed) => Float::with_val(bits, parsed),
                Err(_) => continue,
            };
            let tan_r = x_rug.tan();
            let t_r_val = DBig::from_str(&tan_r.to_string_radix(10, Some(prec)))
                .unwrap()
                .with_rounding::<HalfEven>();

            assert!(
                (tan_d.clone() - t_r_val).abs()
                    <= DBig::from_parts(100.into(), -(isize::try_from(prec).unwrap())),
                "Tan mismatch at iteration {i}, x={x_str}, prec={prec}: dashu={tan_d}, rug={tan_r}"
            );
        }
    }
}
