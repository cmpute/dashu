//! Exact-integer validation of the public short-division error contract.
use dashu_base::{Abs, BitTest};
use dashu_int::{high::div_high, IBig, UBig, Word};
use proptest::prelude::*;

fn pattern_words(len: usize, step: u64) -> Vec<Word> {
    (0..len)
        .map(|i| {
            let w = (i as u64 + 1).wrapping_mul(step).wrapping_add(i as u64) as Word;
            if i + 1 == len && w == 0 {
                1
            } else {
                w
            }
        })
        .collect()
}

/// Fixed (denominator, numerator) step pairs: complementary patterns, so
/// the two operands of a case never align.
const CASES: [(u64, u64); 3] = [
    (0x0123_4567_89ab_cdef, 0x3333_3333_3333_3335),
    (0xfedc_ba98_7654_3211, 0x5eed_5eed_5eed_5eed),
    (0x1357_9bdf_2468_ace1, 0x0f0f_0f0f_0f0f_0f0f),
];

/// Check the full public contract with an exact integer comparison:
/// `|q·denom·2^down − numer·2^up| <= E·denom·2^down` is precisely the
/// documented band `|q − (numer/denom)·2^sigma| <= E` scaled to integers
/// (multiplying by `denom·2^down`), with `up = max(sigma, 0)` and
/// `down = max(−sigma, 0)`.
fn check_case(numer: &[Word], denom: &[Word], n: usize) -> (IBig, IBig) {
    let (numer, denom) = (UBig::from_words(numer), UBig::from_words(denom));
    let q = div_high(&numer, &denom, n).expect("in-band inputs must be accepted");

    let sigma = n as isize * (Word::BITS as usize) as isize + denom.bit_len() as isize
        - numer.bit_len() as isize;
    let (up, down) = (sigma.max(0) as usize, (-sigma).max(0) as usize);
    let bound = IBig::from(2 * n as u64 + 2) * IBig::from(&denom << down);
    let diff = (IBig::from((&q * &denom) << down) - IBig::from(&numer << up)).abs();
    assert!(diff <= bound, "error bound violated: |diff|={diff:?} n={n} sigma={sigma}");
    (diff, bound)
}

/// A numerator pinned to a band edge: [`in_band_numer`] shifted so that
/// its bit-length offset from the denominator is exactly
/// `n·Word::BITS − sigma`, i.e. `sigma ∈ [-2, 2]` covers the whole band.
fn band_edge_numer(step: u64, denom: &[Word], n: usize, sigma: isize) -> Vec<Word> {
    let base = UBig::from_words(&in_band_numer(step, denom, n));
    let numer = if sigma >= 0 {
        &base >> sigma as usize
    } else {
        &base << (-sigma) as usize
    };
    numer.as_words().to_vec()
}

/// A numerator whose bit length exceeds the denominator's by exactly
/// `n·Word::BITS`: `n` pattern words on top of the divisor words, with the
/// top word adjusted to carry the same bit length as the divisor's top
/// word (so the band holds for any word size).
fn in_band_numer(step: u64, denom: &[Word], n: usize) -> Vec<Word> {
    let d_top = *denom.last().unwrap();
    let d_bits = Word::BITS - d_top.leading_zeros();
    let mut numer = pattern_words(n, step);
    numer[n - 1] = (d_top >> 1) | (1 << (d_bits - 1));
    numer.extend_from_slice(denom);
    numer
}

#[test]
fn test_div_high_exact_band() {
    // The base-case band (n <= threshold) and the first recursion levels,
    // with the numerator pinned to every band edge (sigma ∈ [-2, 2]).
    for &n in &[2, 3, 5, 8, 12, 15, 16, 17, 20, 24, 30, 45, 60, 90, 130, 200] {
        for &(dstep, nstep) in &CASES {
            let denom = pattern_words((n / 2 + 2).min(n), dstep);
            for sigma in -2..=2isize {
                check_case(&band_edge_numer(nstep, &denom, n, sigma), &denom, n);
            }
        }
    }
}

#[test]
fn test_div_high_full_width_denominators() {
    // Denominators with a full top word exercise the normalization shifts
    // in the kernel (and the `numer_shift < wn·Word::BITS` copy path —
    // reached on the negative band edges, where the numerator grows past
    // 2n words).
    for &n in &[6, 16, 24, 40, 70] {
        let mut denom = pattern_words(n / 2 + 2, 0x5eed + n as u64);
        let last = denom.len() - 1;
        denom[last] |= 1 << (Word::BITS - 1);
        for sigma in -2..=2isize {
            check_case(&band_edge_numer(0x7000 + n as u64, &denom, n, sigma), &denom, n);
        }
    }
}

#[test]
fn test_div_high_carry_quotient() {
    // Numerators at and above the block-quotient carry edge: the exact
    // block sees the top of its dividend at least as large as the top of
    // the divisor.
    for &n in &[4, 15, 16, 24, 40, 70] {
        let mut denom = pattern_words(n / 2 + 2, 0x5eed);
        let last = denom.len() - 1;
        denom[last] |= 1 << (Word::BITS - 1);

        // A numerator extended to n extra words, with the top word full.
        let mut numer = denom.clone();
        numer.resize(n + denom.len(), 0);
        numer[n + denom.len() - 1] = Word::MAX;
        check_case(&numer, &denom, n);

        // The same frame with the smallest possible full-width top word.
        let mut numer = denom.clone();
        numer.resize(n + denom.len(), 0);
        numer[n + denom.len() - 1] = 1 << (Word::BITS - 1);
        check_case(&numer, &denom, n);
    }
}

#[test]
fn test_div_high_edge_patterns() {
    // All-ones words: maximal borrow and carry chains.
    let ones = |len: usize| vec![Word::MAX; len];
    for &n in &[3, 12, 16, 24, 48, 80] {
        check_case(&ones(n + n / 2 + 2), &ones(n / 2 + 2), n);
    }

    // A sparse denominator with a single high bit.
    let mut denom = vec![0 as Word; 9];
    denom[8] = 1;
    check_case(&in_band_numer(0x0dd, &denom, 12), &denom, 12);

    // Out-of-band and malformed inputs must be declined.
    let d = UBig::from_words(&[1, 2, 3]);
    let small = &d << 100;
    assert!(div_high(&small, &d, 2).is_none()); // window too small for the quotient
    assert!(div_high(&small, &d, 0).is_none());
    assert!(div_high(&d, &UBig::from(2u32), 4).is_none()); // single-word divisor
    assert!(div_high(&UBig::ZERO, &d, 4).is_none());
    assert!(div_high(&d, &UBig::ZERO, 4).is_none());

    // A divisor wider than the window is declined.
    let wide = &d << 300;
    assert!(div_high(&(&wide << 10), &wide, 2).is_none());
}

/// The empirical worst-case error must stay far inside the documented
/// `2n + 2` band — the analysis on the recursive kernel predicts about half
/// of it at most.
#[test]
fn test_div_high_empirical_error_headroom() {
    // Eight complementary step pairs, all distinct from `CASES`.
    const HEADROOM_STEPS: [(u64, u64); 8] = [
        (0x0fed_cba9_8765_4321, 0x7777_7777_7777_7777),
        (0x5eed_5eed_5eed_5eed, 0x1357_9bdf_2468_ace1),
        (0x3333_3333_3333_3335, 0x0123_4567_89ab_cdef),
        (0x0f0f_0f0f_0f0f_0f0f, 0xfedc_ba98_7654_3211),
        (0x7777_7777_7777_7777, 0x0fed_cba9_8765_4321),
        (0x1357_9bdf_2468_ace1, 0x5eed_5eed_5eed_5eed),
        (0x0123_4567_89ab_cdef, 0x3333_3333_3333_3335),
        (0x2468_ace0_2468_ace1, 0x9999_9999_9999_9999),
    ];
    let mut worst = IBig::ZERO;
    for &n in &[16, 24, 33, 48, 64, 96] {
        for &(dstep, nstep) in &HEADROOM_STEPS {
            let denom = pattern_words(n / 2 + 2, dstep);
            let (diff, bound) = check_case(&in_band_numer(nstep, &denom, n), &denom, n);
            // fixed-point ratio in thousandths, for a readable maximum
            let ratio = (&diff * 1000) / &bound;
            if ratio > worst {
                worst = ratio;
            }
        }
    }
    assert!(
        worst <= IBig::from(700),
        "empirical error crept above 70% of the bound: {worst:?} thousandths"
    );
}

// Generate the denominator directly at a valid width. The numerator is resized
// to the required bit length at every band edge, including after shrinking.
proptest! {
    #![proptest_config(ProptestConfig { cases: 64, max_shrink_iters: 1024, ..Default::default() })]
    #[test]
    fn quotient_error_bound(
        (n, mut denominator, numerator) in (2usize..97).prop_flat_map(|n| (
            Just(n),
            prop::collection::vec(any::<Word>(), 2..=n),
            prop::collection::vec(any::<Word>(), 2 * n + 1),
        ))
    ) {
        *denominator.last_mut().unwrap() |= 1;
        let denom = UBig::from_words(&denominator);
        let raw = UBig::from_words(&numerator);
        for sigma in -2..=2isize {
            let bits = (denom.bit_len() + n * Word::BITS as usize) as isize - sigma;
            let top = UBig::ONE << (bits as usize - 1);
            let numer = (&raw & (&top - UBig::ONE)) | top;
            check_case(numer.as_words(), &denominator, n);
        }
    }
}
