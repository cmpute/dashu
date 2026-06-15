//! Verify sqr() == a*a for the simple-schoolbook range and adversarial inputs.

use dashu_int::UBig;

fn lcg_words(seed: u64, words: usize) -> UBig {
    let mut limbs = Vec::with_capacity(words);
    let mut s = seed.wrapping_add(words as u64);
    for _ in 0..words {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        limbs.push(s);
    }
    if words > 0 {
        limbs[words - 1] |= 1; // force full width
    }
    UBig::from_words(&limbs)
}

#[test]
fn sqr_matches_mul_simple_range() {
    // sizes 2..128 span simple directly (<=30) and as the basecase of karatsuba/toom
    for words in 2..=128 {
        for seed in 0..6u64 {
            let a = lcg_words(seed, words);
            assert_eq!(a.sqr(), &a * &a, "sqr != mul at words={words} seed={seed}");
        }
    }
}

#[test]
fn sqr_matches_mul_adversarial() {
    // patterns that stress carry propagation: all-ones, single high bit, mixed
    for words in &[2usize, 3, 4, 5, 7, 8, 15, 16, 17, 30, 31, 32, 48] {
        let ones = vec![u64::MAX; *words];
        let a = UBig::from_words(&ones);
        assert_eq!(a.sqr(), &a * &a, "all-ones mismatch at words={words}");

        let mut hi = vec![0u64; *words];
        hi[*words - 1] = 1;
        let a = UBig::from_words(&hi);
        assert_eq!(a.sqr(), &a * &a, "single-high-bit mismatch at words={words}");

        let mixed: Vec<u64> = (0..*words as u64)
            .map(|k| if k % 2 == 0 { u64::MAX } else { 1 })
            .collect();
        let a = UBig::from_words(&mixed);
        assert_eq!(a.sqr(), &a * &a, "mixed mismatch at words={words}");
    }
}
