use dashu_int::{fast_div::ConstDivisor, ops::DivRem, rand::UniformBits, IBig, UBig};
use rand_v08::{distributions::uniform::Uniform, prelude::*};

mod helper_macros;

// Keep routine coverage bounded even when force_bits changes Word but not usize.
// The 100,000- and 1,000,000-bit campaigns live in fuzz/tests/integer_stress.rs.
const ARITHMETIC_CASES: &[(usize, usize)] = &[(100, 256), (1_000, 64), (10_000, 16)];

#[test]
fn test_uniform_bits() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = UniformBits::new(0);
    let x: UBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x.is_zero());
    let x: IBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x.is_zero());

    let distr = UniformBits::new(2);
    let x: UBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert_eq!(x, UBig::ZERO);
    let x: UBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert_eq!(x, UBig::from(3u8));
    let x: IBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert_eq!(x, IBig::from(-3));
    let x: IBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert_eq!(x, IBig::from(3));

    let distr = UniformBits::new(200);
    let x: UBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < UBig::ONE << 200);
    let x: IBig = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x < IBig::ONE << 200);
    let x: IBig = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x > IBig::NEG_ONE << 200);
}

#[test]
fn test_uniform_ubig() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = Uniform::from(ubig!(3)..ubig!(7));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert_eq!(x, ubig!(3));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert_eq!(x, ubig!(6));

    let distr = Uniform::from(ubig!(3)..=ubig!(7));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert_eq!(x, ubig!(3));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert_eq!(x, ubig!(7));

    let distr = Uniform::from(ubig!(0b100) << 128..ubig!(0b1000) << 128);
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert!(x >= ubig!(0b100) << 128 && x < ubig!(0b101) << 128);
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert!(x >= ubig!(0b111) << 128 && x < ubig!(0b1000) << 128);
}

#[test]
fn test_uniform_ibig() {
    let mut rng = StdRng::seed_from_u64(1);

    let distr = Uniform::from(ibig!(-7)..ibig!(3));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert_eq!(x, ibig!(-7));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert_eq!(x, ibig!(2));

    let distr = Uniform::from(ibig!(-7)..=ibig!(3));
    let x = (&mut rng).sample_iter(&distr).take(1000).min().unwrap();
    assert_eq!(x, ibig!(-7));
    let x = (&mut rng).sample_iter(&distr).take(1000).max().unwrap();
    assert_eq!(x, ibig!(3));
}

#[test]
fn test_random_arithmetic() {
    let mut rng = StdRng::seed_from_u64(3);
    let p = ubig!(1000000007);

    for &(num_bits, num_cases) in ARITHMETIC_CASES {
        for _ in 0..num_cases {
            let len_a = rng.gen_range(10..num_bits);
            let len_b = rng.gen_range(10..num_bits);
            let a = rng.gen_range(ubig!(100)..ubig!(1) << len_a);
            let b = rng.gen_range(ubig!(100)..ubig!(1) << len_b);
            let c = rng.sample(Uniform::new(ubig!(0), &a));

            let radix = rng.gen_range(2..=36);
            assert_eq!(
                UBig::from_str_radix(&a.in_radix(radix).to_string(), radix as u32).unwrap(),
                a
            );

            assert_eq!((&a + &b) % &p, ((&a % &p) + (&b % &p)) % &p);
            assert_eq!(&a + &b - &a, b);
            let product = &a * &b;
            assert_eq!(&product % &p, ((&a % &p) * (&b % &p)) % &p);
            assert_eq!((product + &c).div_rem(&a), (b.clone(), c.clone()));
            assert_eq!((&a + ubig!(1)) * (&a - ubig!(1)), a.sqr() - ubig!(1));
        }
    }
}

#[test]
fn test_random_modular_arithmetic() {
    let mut rng = StdRng::seed_from_u64(3);
    let p = ConstDivisor::new(ubig!(1000000007));
    let one = p.reduce(ubig!(1));
    let zero = p.reduce(UBig::ZERO);

    for &(num_bits, num_cases) in ARITHMETIC_CASES {
        for _ in 0..num_cases {
            let len_a = rng.gen_range(10..num_bits);
            let len_b = rng.gen_range(10..num_bits);
            let a = p.reduce(rng.gen_range(ubig!(100)..ubig!(1) << len_a));
            let b = p.reduce(rng.gen_range(ubig!(100)..ubig!(1) << len_b));

            assert_eq!(&a + &b - &a, b);
            // Reduction can map a nonzero input to zero, which has no inverse.
            if a != zero {
                assert_eq!(&a * &b / &a, b);
            }
            assert_eq!((&a + &one) * (&a - &one), a.sqr() - &one);
        }
    }
}
