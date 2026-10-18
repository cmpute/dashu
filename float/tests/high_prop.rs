//! Property tests for short-product and short-quotient rounding against exact oracles.
use dashu_base::Sign;
use dashu_float::{
    round::{mode, Round},
    Context, FBig, Repr, Word,
};
use dashu_int::IBig;
use proptest::prelude::*;
mod helpers;
use helpers::{check_products, quotient_oracle, scale, significand, words_for_precision};

// Fix the width and nonzero ends in the strategy so shrinking preserves valid
// nonzero operands. Signs and exponents are varied independently.
fn operand(words: usize) -> impl Strategy<Value = IBig> {
    (prop::collection::vec(any::<Word>(), words), any::<bool>()).prop_map(
        |(mut words, negative)| {
            words[0] |= 1;
            *words.last_mut().unwrap() |= 1 << (Word::BITS - 1);
            let value = significand(&words);
            if negative {
                -value
            } else {
                value
            }
        },
    )
}

/// Take the low `words` of a generated operand, preserving its sign and forcing
/// the requested width. This keeps shape sweeps valid even during shrinking.
fn operand_prefix(value: &IBig, words: usize) -> IBig {
    let (sign, limbs) = value.as_sign_words();
    let mut limbs = limbs[..words].to_vec();
    *limbs.last_mut().unwrap() |= 1 << (Word::BITS - 1);
    sign * significand(&limbs)
}

fn check_quotients<R: Round, const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
    check_div::<R, B>(p, a, b);
    assert_eq!(
        Context::<R>::new(p).inv(b).unwrap().map(FBig::into_repr),
        quotient_oracle::<R, B>(p, &Repr::one(), b),
        "inv: p={p} b={b:?}"
    );
}

fn check_quotients_all_modes<const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
    check_quotients::<mode::Zero, B>(p, a, b);
    check_quotients::<mode::Away, B>(p, a, b);
    check_quotients::<mode::Up, B>(p, a, b);
    check_quotients::<mode::Down, B>(p, a, b);
    check_quotients::<mode::HalfEven, B>(p, a, b);
    check_quotients::<mode::HalfAway, B>(p, a, b);
}

/// Compare the public result, including exactness and rounding adjustment,
/// with exact integer quotient/remainder arithmetic.
fn check_div<R: Round, const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
    assert_eq!(
        Context::<R>::new(p).div(a, b).unwrap().map(FBig::into_repr),
        quotient_oracle::<R, B>(p, a, b),
        "p={p} a={a:?} b={b:?}"
    );
}

/// Every generated pair covers every precision and operand shape. Randomness
/// varies the digits, signs, and exponents rather than selecting the coverage.
fn check_quotient_shapes<const B: Word>(
    precisions: &[usize],
    a: &IBig,
    b: &IBig,
    ea: isize,
    eb: isize,
) {
    for &p in precisions {
        let words = words_for_precision(p, B);
        // Equal widths, a wider dividend, and a narrower divisor.
        for (aw, bw) in [(words, words), (words + 6, words), (words, words - 2)] {
            check_quotients_all_modes(
                p,
                &Repr::<B>::new(operand_prefix(a, aw), ea),
                &Repr::<B>::new(operand_prefix(b, bw), eb),
            );
        }
    }
}

/// Compare value, exactness, and rounding adjustment under all six modes.
fn check_products_all_modes<const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>, f: &Repr<B>) {
    check_products::<mode::Zero, B>(p, a, b, f);
    check_products::<mode::Away, B>(p, a, b, f);
    check_products::<mode::Up, B>(p, a, b, f);
    check_products::<mode::Down, B>(p, a, b, f);
    check_products::<mode::HalfEven, B>(p, a, b, f);
    check_products::<mode::HalfAway, B>(p, a, b, f);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 16, max_shrink_iters: 512, ..Default::default() })]
    #[test]
    fn division_and_reciprocal_rounding(
        a in operand(words_for_precision(500, 10) + 6),
        b in operand(words_for_precision(500, 10)),
        ea in -30isize..31, eb in -30isize..31,
    ) {
        // Include all four sign combinations for each generated magnitude pair.
        for sa in [Sign::Positive, Sign::Negative] {
            for sb in [Sign::Positive, Sign::Negative] {
                let a = sa * a.clone();
                let b = sb * b.clone();
                check_quotient_shapes::<2>(&[20, 50, 100, 500], &a, &b, ea, eb);
                check_quotient_shapes::<10>(&[20, 50, 100, 500], &a, &b, ea, eb);
            }
        }
    }

    #[test]
    fn wide_division_and_reciprocal_rounding(
        a in operand(words_for_precision(1200, 10) + 6),
        b in operand(words_for_precision(1200, 10)),
        ea in -30isize..31, eb in -30isize..31,
    ) {
        check_quotient_shapes::<2>(&[2200, 2400, 2600, 3500], &a, &b, ea, eb);
        check_quotient_shapes::<10>(&[700, 750, 1200], &a, &b, ea, eb);
    }

    #[test]
    fn products_match_exact_rounding(
        a in operand(scale(90)), b in operand(scale(260)),
        ea in -500isize..501, eb in -500isize..501,
    ) {
        // Equal widths and both directions of imbalance, including a long tail.
        for (aw, bw) in [(64, 64), (64, 72), (64, 90), (90, 64), (64, 260)] {
            let a = operand_prefix(&a, scale(aw));
            let b = operand_prefix(&b, scale(bw));
            for p in [2, 5, 20, 50, 100, 500] {
                let a2 = Repr::<2>::new(a.clone(), ea);
                let b2 = Repr::<2>::new(b.clone(), eb);
                check_products_all_modes(p, &a2, &b2, &a2);
                let a10 = Repr::<10>::new(a.clone(), ea);
                let b10 = Repr::<10>::new(b.clone(), eb);
                check_products_all_modes(p, &a10, &b10, &a10);
            }
        }
    }

    #[test]
    fn equal_precision_products(
        a in operand(scale(310)), b in operand(scale(310)),
    ) {
        for (p, words) in [(2048, 32), (4096, 64)] {
            let a = Repr::<2>::new(operand_prefix(&a, scale(words)), 0);
            let b = Repr::<2>::new(operand_prefix(&b, scale(words)), 0);
            check_products_all_modes(p, &a, &b, &a);
            check_products_all_modes(p, &a, &a, &a);
        }
        // Large decimal operands exercise a window shorter than either input.
        let a = Repr::<10>::new(a, 0);
        let b = Repr::<10>::new(b, 0);
        check_products_all_modes(2000, &a, &b, &a);
        check_products_all_modes(2000, &a, &a, &a);
    }
}
