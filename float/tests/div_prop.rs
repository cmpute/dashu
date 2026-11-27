//! Property tests for division against an exact oracle ([`helpers::quotient_oracle`]): the
//! integer quotient and remainder of the operands scaled to the target precision decide the
//! rounding, with no floating-point division involved.
//!
//! `high_prop.rs` covers the certified short-quotient path on wide operands. The strategies
//! here target the exact path behind it:
//!
//! * operands of every width and both signs, with random exponents;
//! * values on and around a rounding boundary, and exactly on it;
//! * an operand far wider than the result — a wide dividend has its low digits cut off in
//!   bulk, and they must stay sticky through the rounding (at random, and right above a
//!   boundary within them); a wide divisor leaves the dividend padded with zero digits;
//! * exact divisions, whose quotient can be wider than the precision;
//! * exponents next to the limits of the range, where the result is an error.
//!
//! [`Context::div_int`] is the same division with both exponents zero, and is checked
//! against the oracle separately.

use core::fmt;

use dashu_base::{BitTest, UnsignedAbs};
use dashu_float::{
    round::{mode, Round},
    Context, FBig, FpError, FpResult, Repr, Word,
};
use dashu_int::{IBig, UBig};
use proptest::prelude::*;

// only the division oracle is used here; the product oracles serve `high.rs`
#[allow(dead_code)]
mod helpers;
use helpers::quotient_oracle;

/// A failed check, describing the case and the disagreement.
struct Mismatch(String);

impl fmt::Debug for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Mismatch {}

/// A case: the dividend with its exponent, the divisor with its exponent, and the precision.
type Case = (IBig, isize, IBig, isize, usize);

// ---------------------------------------------------------------------------
// Oracle
// ---------------------------------------------------------------------------

/// The correctly rounded quotient `lhs / rhs` to `precision` digits, errors included.
///
/// The oracle decides the significand from the significands alone; the operands' exponents
/// only shift its exponent. A result is representable iff the exponent of its normalized form
/// lies strictly inside the range: at the bottom, a quotient that was padded with zero digits
/// down past the limit folds back into it when those digits are folded away.
fn expected<R: Round, const B: Word>(
    lhs: &Repr<B>,
    rhs: &Repr<B>,
    precision: usize,
) -> FpResult<Repr<B>> {
    let integers = quotient_oracle::<R, B>(
        precision,
        &Repr::new(lhs.significand().clone(), 0),
        &Repr::new(rhs.significand().clone(), 0),
    );
    let sign = integers.value_ref().sign();
    // (the oracle returns the normalized form)
    let normalized =
        integers.value_ref().exponent() as i128 + (lhs.exponent() as i128 - rhs.exponent() as i128);

    if normalized <= isize::MIN as i128 {
        Err(FpError::Underflow(sign))
    } else if normalized >= isize::MAX as i128 {
        Err(FpError::Overflow(sign))
    } else {
        Ok(integers.map(|repr| {
            let (significand, _) = repr.into_parts();
            Repr::new(significand, normalized as isize)
        }))
    }
}

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

/// Check `lhs / rhs` under one mode against the oracle.
fn check_div<R: Round, const B: Word>(
    lhs: &Repr<B>,
    rhs: &Repr<B>,
    precision: usize,
) -> Result<(), Mismatch> {
    let got = Context::<R>::new(precision)
        .div(lhs, rhs)
        .map(|rounded| rounded.map(FBig::into_repr));
    let expected = expected::<R, B>(lhs, rhs, precision);
    if got != expected {
        return Err(Mismatch(format!(
            "{lhs:?} / {rhs:?} to {precision} digits in base {B}: \
             got {got:?}, expected {expected:?}",
        )));
    }
    if let Ok(rounded) = &got {
        let digits = rounded.value_ref().digits();
        if digits > precision {
            return Err(Mismatch(format!(
                "the result {rounded:?} has {digits} digits at precision {precision}",
            )));
        }
    }
    Ok(())
}

/// Check `numerator / denominator` under one mode against the oracle.
fn check_div_int<R: Round, const B: Word>(
    numerator: &IBig,
    denominator: &UBig,
    precision: usize,
) -> Result<(), Mismatch> {
    let lhs = Repr::<B>::new(numerator.clone(), 0);
    let rhs = Repr::<B>::new(IBig::from(denominator.clone()), 0);
    let got = Context::<R>::new(precision)
        .div_int::<B>(numerator, denominator)
        .map(|rounded| rounded.map(FBig::into_repr));
    let expected = expected::<R, B>(&lhs, &rhs, precision);
    if got != expected {
        return Err(Mismatch(format!(
            "{numerator:?} / {denominator:?} to {precision} digits in base {B}: \
             got {got:?}, expected {expected:?}",
        )));
    }
    Ok(())
}

/// Check one division under every built-in rounding mode in base `$base`.
macro_rules! check_all_modes {
    ($base:tt, $lhs:expr, $rhs:expr, $precision:expr) => {{
        let (lhs, rhs, precision) = (&$lhs, &$rhs, $precision);
        check_div::<mode::Zero, $base>(lhs, rhs, precision)
            .and_then(|()| check_div::<mode::Away, $base>(lhs, rhs, precision))
            .and_then(|()| check_div::<mode::Up, $base>(lhs, rhs, precision))
            .and_then(|()| check_div::<mode::Down, $base>(lhs, rhs, precision))
            .and_then(|()| check_div::<mode::HalfEven, $base>(lhs, rhs, precision))
            .and_then(|()| check_div::<mode::HalfAway, $base>(lhs, rhs, precision))
    }};
}

/// Check one integer ratio under every built-in rounding mode in base `$base`.
macro_rules! check_int_all_modes {
    ($base:tt, $num:expr, $den:expr, $precision:expr) => {{
        let (num, den, precision) = (&$num, &$den, $precision);
        check_div_int::<mode::Zero, $base>(num, den, precision)
            .and_then(|()| check_div_int::<mode::Away, $base>(num, den, precision))
            .and_then(|()| check_div_int::<mode::Up, $base>(num, den, precision))
            .and_then(|()| check_div_int::<mode::Down, $base>(num, den, precision))
            .and_then(|()| check_div_int::<mode::HalfEven, $base>(num, den, precision))
            .and_then(|()| check_div_int::<mode::HalfAway, $base>(num, den, precision))
    }};
}

/// Check one case in base `B`.
fn check_case<const B: Word>(case: &Case) -> Result<(), Mismatch> {
    let (num, exp_num, den, exp_den, precision) = case;
    let lhs = Repr::<B>::new(num.clone(), *exp_num);
    let rhs = Repr::<B>::new(den.clone(), *exp_den);
    check_all_modes!(B, lhs, rhs, *precision)
        .map_err(|e| Mismatch(format!("exponents {exp_num}, {exp_den}: {e}")))
}

/// Check one integer ratio in base `B`.
fn check_int_case<const B: Word>(num: &IBig, den: &UBig, precision: usize) -> Result<(), Mismatch> {
    check_int_all_modes!(B, num, den, precision)
}

/// Check a case in every tested base, under every mode: binary and decimal (the common cases),
/// two odd bases, a power-of-two base wider than one bit, and an even base with several trailing
/// zero bits that is not a power of two.
fn check_case_all_bases(case: &Case) -> Result<(), Mismatch> {
    check_case::<2>(case)
        .and_then(|()| check_case::<3>(case))
        .and_then(|()| check_case::<7>(case))
        .and_then(|()| check_case::<10>(case))
        .and_then(|()| check_case::<12>(case))
        .and_then(|()| check_case::<16>(case))
}

/// Check an integer ratio in every tested base, under every mode.
fn check_int_all_bases(num: &IBig, den: &UBig, precision: usize) -> Result<(), Mismatch> {
    check_int_case::<2>(num, den, precision)
        .and_then(|()| check_int_case::<3>(num, den, precision))
        .and_then(|()| check_int_case::<7>(num, den, precision))
        .and_then(|()| check_int_case::<10>(num, den, precision))
        .and_then(|()| check_int_case::<12>(num, den, precision))
        .and_then(|()| check_int_case::<16>(num, den, precision))
}

// ---------------------------------------------------------------------------
// Input strategies
// ---------------------------------------------------------------------------

/// A random integer of exactly `bits` bits (the leading bit is set).
fn unsigned(bits: usize) -> impl Strategy<Value = UBig> {
    let full_words = (bits - 1) / Word::BITS as usize;
    let top_bits = (bits - 1) % Word::BITS as usize; // random bits below the leading one
    (prop::collection::vec(any::<Word>(), full_words..=full_words), any::<Word>()).prop_map(
        move |(mut words, top)| {
            let keep = if top_bits == 0 {
                0
            } else {
                Word::MAX >> (Word::BITS as usize - top_bits)
            };
            words.push((top & keep) | (1 << top_bits));
            UBig::from_words(&words)
        },
    )
}

fn signed(num: UBig, negative: bool) -> IBig {
    let num = IBig::from(num);
    if negative {
        -num
    } else {
        num
    }
}

/// An exponent that keeps every intermediate of the division well inside the range.
fn exponent() -> impl Strategy<Value = isize> {
    -300isize..=300isize
}

/// Random nonzero operands of up to `max_bits` bits with independent signs and exponents. The
/// operand sizes are drawn independently, so the dividend is sometimes narrower than the
/// precision needs (it is padded with zero digits) and sometimes far wider (its low digits are
/// cut off in bulk).
fn random_case(max_bits: usize, max_precision: usize) -> impl Strategy<Value = Case> {
    (
        1..max_bits,
        1..max_bits,
        any::<bool>(),
        any::<bool>(),
        exponent(),
        exponent(),
        1..max_precision,
    )
        .prop_flat_map(|(num_bits, den_bits, neg_num, neg_den, exp_num, exp_den, precision)| {
            (
                unsigned(num_bits),
                unsigned(den_bits),
                Just((neg_num, neg_den, exp_num, exp_den, precision)),
            )
        })
        .prop_map(|(num, den, (neg_num, neg_den, exp_num, exp_den, precision))| {
            (signed(num, neg_num), exp_num, signed(den, neg_den), exp_den, precision)
        })
}

/// One operand far wider than the precision needs, overshooting `precision + other operand`
/// digits in every tested base (base 16 is the widest). Either the dividend is the wide one,
/// or the divisor is.
fn wide_case() -> impl Strategy<Value = Case> {
    (1..=120usize, 1..=200usize, 1..=400usize, any::<bool>(), any::<bool>())
        .prop_flat_map(|(precision, narrow_bits, excess, wide_divisor, negative)| {
            let wide_bits = 4 * precision + narrow_bits + excess;
            let (num_bits, den_bits) = if wide_divisor {
                (narrow_bits, wide_bits)
            } else {
                (wide_bits, narrow_bits)
            };
            ((unsigned(num_bits), unsigned(den_bits)), Just((negative, precision)))
        })
        .prop_map(|((num, den), (negative, precision))| {
            (signed(num, negative), 0, den.into(), 0, precision)
        })
}

/// Values right above a rounding boundary that lies within the digits cut off in bulk from the
/// dividend: (k + 1/2 + 1/4)·B^m with `k` of `precision` digits. The integer part of the
/// quotient ends exactly on the boundary, so only the sticky digits tell that the value lies
/// above it. A randomly chosen wide dividend practically never lands on this pattern.
fn wide_tie<const B: Word>() -> impl Strategy<Value = Case> {
    (
        1..=40usize,  // precision
        1..=12usize,  // digits cut off
        3..=100usize, // divisor bits
        any::<bool>(),
        exponent(),
        exponent(),
    )
        .prop_flat_map(move |(precision, m, den_bits, negative, exp_num, exp_den)| {
            let span = UBig::from_word(B - 1) * UBig::from_word(B).pow(precision - 1);
            (
                unsigned(span.bit_len()),
                unsigned(den_bits),
                Just((precision, m, negative, exp_num, exp_den, span)),
            )
        })
        .prop_map(move |(random, den, (precision, m, negative, exp_num, exp_den, span))| {
            let base = UBig::from_word(B);
            let k = base.pow(precision - 1) + (random % &span);
            // (k + 1/2)·B^m + c / den = [(2k + 1)·B^m·den + 2c] / (2·den) with c = den / 4
            let numer = ((&k + &k + UBig::ONE) * base.pow(m)) * &den + (&den >> 1 >> 1);
            (
                signed(numer, negative),
                exp_num,
                (den * UBig::from(2u8)).into(),
                exp_den,
                precision,
            )
        })
}

/// Values on and around a rounding boundary of base `B`: (k + 1/2 + offset / 2q)·B^s with `k`
/// having exactly `precision` digits, so that the boundary separates the two candidates for
/// the `precision`-digit result, and the offset (when nonzero) a tiny fraction of the last kept
/// digit's unit. Both candidates and the boundary itself are exercised.
fn near_tie<const B: Word>() -> impl Strategy<Value = Case> {
    (
        1..=40usize,   // precision
        -12..=12isize, // exponent of the unit
        -1i32..=1,     // position relative to the boundary
        1..=1000u32,   // makes the offset tiny compared with the unit
        any::<bool>(),
        exponent(),
        exponent(),
    )
        .prop_flat_map(move |(precision, s, offset, q, negative, exp_num, exp_den)| {
            // k = B^(precision-1) + (a random value below B^(precision-1)·(B − 1))
            let span = UBig::from_word(B - 1) * UBig::from_word(B).pow(precision - 1);
            (
                unsigned(span.bit_len()),
                Just((precision, s, offset, q, negative, exp_num, exp_den, span)),
            )
        })
        .prop_map(
            move |(random, (precision, s, offset, q, negative, exp_num, exp_den, span))| {
                let base = UBig::from_word(B);
                let k = base.pow(precision - 1) + (random % &span);
                let numer =
                    (&k + &k + UBig::ONE) * UBig::from(q) + UBig::from(offset.unsigned_abs());
                let numer = if offset < 0 {
                    numer - UBig::from(2u8)
                } else {
                    numer
                };
                let mut numer = signed(numer, negative);
                let mut denom = UBig::from(2u8) * UBig::from(q);
                if s >= 0 {
                    numer *= base.pow(s as usize);
                } else {
                    denom *= base.pow((-s) as usize);
                }
                (numer, exp_num, denom.into(), exp_den, precision)
            },
        )
}

/// An exact division: the dividend is a multiple of the divisor. The quotient can be wider
/// than the precision, in which case it has to be rounded although nothing was divided away.
fn exact_case() -> impl Strategy<Value = Case> {
    (
        1..=300usize,
        1..=400usize,
        any::<bool>(),
        any::<bool>(),
        exponent(),
        exponent(),
        1..=300usize,
    )
        .prop_flat_map(|(q_bits, den_bits, negative, neg_den, exp_num, exp_den, precision)| {
            (
                unsigned(q_bits),
                unsigned(den_bits),
                Just((negative, neg_den, exp_num, exp_den, precision)),
            )
        })
        .prop_map(|(q, den, (negative, neg_den, exp_num, exp_den, precision))| {
            let den = signed(den, neg_den);
            (signed(q, negative) * den.clone(), exp_num, den, exp_den, precision)
        })
}

/// Operands of up to `bits` bits with an exponent next to a limit of the range, or far from
/// both, so that the quotient's exponent can land anywhere — including outside the range.
fn extreme_exponent_case(bits: usize) -> impl Strategy<Value = Case> {
    let near_max = isize::MAX - 64..=isize::MAX - 1;
    let near_min = isize::MIN..=isize::MIN + 64;
    let far = (-3000isize..=3000isize).prop_filter("far from the limits", |e| e.abs() >= 100);
    (
        1..bits,
        1..bits,
        any::<bool>(),
        any::<bool>(),
        prop_oneof![near_max.clone(), near_min.clone(), far.clone()],
        prop_oneof![near_max, near_min, far],
        1..=500usize,
    )
        .prop_flat_map(|(num_bits, den_bits, neg_num, neg_den, exp_num, exp_den, precision)| {
            (
                unsigned(num_bits),
                unsigned(den_bits),
                Just((neg_num, neg_den, exp_num, exp_den, precision)),
            )
        })
        .prop_map(|(num, den, (neg_num, neg_den, exp_num, exp_den, precision))| {
            (signed(num, neg_num), exp_num, signed(den, neg_den), exp_den, precision)
        })
}

fn integers(num: &str, den: &str) -> (IBig, IBig) {
    (num.parse().unwrap(), den.parse().unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..Default::default() })]

    /// Random operands of every shape, sign and exponent.
    #[test]
    fn div_matches_oracle(case in random_case(600, 100)) {
        check_case_all_bases(&case).unwrap();
    }

    /// Values on both sides of a rounding boundary, and exactly on it.
    #[test]
    fn div_matches_oracle_near_ties(
        case in prop_oneof![
            near_tie::<2>(),
            near_tie::<3>(),
            near_tie::<10>(),
            near_tie::<12>(),
            near_tie::<16>()
        ],
    ) {
        check_case_all_bases(&case).unwrap();
    }

    /// An operand far wider than the precision: at random, and a wide dividend right above a
    /// boundary within its cut-off digits.
    #[test]
    fn div_matches_oracle_wide_operands(
        case in prop_oneof![
            wide_case(),
            wide_tie::<2>(),
            wide_tie::<3>(),
            wide_tie::<10>(),
            wide_tie::<12>(),
            wide_tie::<16>()
        ],
    ) {
        check_case_all_bases(&case).unwrap();
    }

    /// Exact divisions, including quotients wider than the precision.
    #[test]
    fn div_matches_oracle_exact(case in exact_case()) {
        check_case_all_bases(&case).unwrap();
    }

    /// Exponents next to the limits of the range: the result is a value or an error.
    #[test]
    fn div_matches_oracle_extreme_exponents(case in extreme_exponent_case(400)) {
        check_case_all_bases(&case).unwrap();
    }

    /// Integer ratios of every shape.
    #[test]
    fn div_int_matches_oracle(case in random_case(600, 100)) {
        let (num, _, den, _, precision) = case;
        let den = den.unsigned_abs();
        check_int_all_bases(&num, &den, precision).unwrap();
    }

    /// Integer ratios with an operand far wider than the precision.
    #[test]
    fn div_int_matches_oracle_wide_operands(
        case in prop_oneof![
            wide_case(),
            wide_tie::<2>(),
            wide_tie::<3>(),
            wide_tie::<10>(),
            wide_tie::<12>(),
            wide_tie::<16>()
        ],
    ) {
        let (num, _, den, _, precision) = case;
        let den = den.unsigned_abs();
        check_int_all_bases(&num, &den, precision).unwrap();
    }
}

/// Fixed cases in every base: exact quotients of every width relative to the precision, exact
/// midpoints next to a digit carry or not, values just above a midpoint (where a double
/// rounding would land on the midpoint and round the other way), and signed operands at
/// non-zero exponents.
#[test]
fn div_fixed_cases() -> Result<(), Mismatch> {
    let cases = [
        // (numerator, numerator exponent, denominator, denominator exponent, precision)
        ("1", 0, "1", 0, 1),
        ("1", 0, "1", 0, 5),
        ("1", 0, "2", 0, 1),
        ("1", 0, "8", 0, 4),
        ("3", 0, "4", 0, 2),
        ("1", 0, "3", 0, 1),
        ("1", 0, "3", 0, 7),
        ("-1", 0, "3", 0, 4),
        ("2", 0, "3", 0, 5),
        ("1", 0, "7", 0, 50),
        ("22", 0, "7", 0, 10),
        ("10", 0, "1", 0, 1),
        ("1000", 0, "1", 0, 4),
        ("123456789", 0, "1", 0, 4),
        ("123450", 0, "1", 0, 4),
        ("123400", 0, "1", 0, 4),
        ("19", 0, "2", 0, 1),   // 9.5: a midpoint next to a digit carry
        ("199", 0, "2", 0, 2),  // 99.5
        ("1999", 0, "2", 0, 3), // 999.5
        ("-199", 0, "2", 0, 2),
        ("19", 0, "20", 0, 1),   // 0.95
        ("199", 0, "200", 0, 2), // 0.995
        ("21", 0, "2", 0, 1),    // 10.5: a midpoint whose even candidate carries a digit
        ("6", 0, "11", 0, 16),   // 0.5454…: just above the midpoint 0.5454545454545454|5
        ("-6", 0, "11", 0, 16),
        ("493801", 0, "4", 0, 4), // 123450.25: just above the midpoint 1234|50
        ("-493801", 0, "4", 0, 4),
        ("31", 0, "3", 0, 2), // 1010.0101…b: just above the midpoint between 1000b and 1100b
        ("28", 0, "3", 0, 3), // 1001.0101…b: just above the midpoint between 1000b and 1010b
        ("1234499", 0, "10", 0, 4), // 123449.9: just below the midpoint 1234|50
        ("-1234499", 0, "10", 0, 4),
        ("15", -1, "5", 0, 1), // 1.5: an exact midpoint
        ("25", -1, "5", 0, 1), // 2.5
        ("11", 0, "1", 0, 2),  // 1011b: the low digit is cut off
        ("7", 0, "-1", 0, 1),  // negative divisor
        ("-7", 0, "-4", 0, 2), // 1.75
        ("1", 300, "3", 0, 8), // far apart exponents
        ("-1", 0, "3", -300, 8),
        ("12345", 40, "6789", -40, 6),
    ];
    for (num, exp_num, den, exp_den, precision) in cases {
        let (num, den) = integers(num, den);
        check_case_all_bases(&(num, exp_num, den, exp_den, precision))?;
    }

    // (10^100 + 1) / 3^60; a 600-bit dividend over a small divisor; a value far below 1;
    // and a negative wide dividend
    let cases = [
        (
            IBig::from(&UBig::from(10u8).pow(100) + UBig::ONE),
            0,
            IBig::from(UBig::from(3u8).pow(60)),
            0,
            37,
        ),
        (
            IBig::from((UBig::ONE << 599) + UBig::from(12345u32)),
            0,
            IBig::from(343u32),
            0,
            50,
        ),
        (IBig::from(1u8), 0, IBig::from((&UBig::ONE << 700) | UBig::ONE), 0, 50),
        (-IBig::from((&UBig::ONE << 1023) + UBig::ONE), 0, IBig::from(1009u32), 0, 9),
    ];
    for (num, exp_num, den, exp_den, precision) in cases {
        check_case_all_bases(&(num, exp_num, den, exp_den, precision))?;
    }
    Ok(())
}

/// The same fixed cases as integer ratios.
#[test]
fn div_int_fixed_cases() -> Result<(), Mismatch> {
    let cases = [
        ("6", "11", 16),
        ("-6", "11", 16),
        ("493801", "4", 4),
        ("31", "3", 2),
        ("28", "3", 3),
        ("1234499", "10", 4),
        ("19", "2", 1),
        ("21", "2", 1),
        ("1", "8", 4),
        ("10", "1", 1),
        ("123456789", "1", 4),
    ];
    for (num, den, precision) in cases {
        let (num, den) = integers(num, den);
        check_int_all_bases(&num, &den.unsigned_abs(), precision)?;
    }

    let cases = [
        (IBig::from(&UBig::from(10u8).pow(100) + UBig::ONE), UBig::from(3u8).pow(60), 37),
        (IBig::from((UBig::ONE << 599) + UBig::from(12345u32)), UBig::from(343u32), 50),
        (-IBig::from((&UBig::ONE << 1023) + UBig::ONE), UBig::from(1009u32), 9),
    ];
    for (num, den, precision) in cases {
        check_int_all_bases(&num, &den, precision)?;
    }
    Ok(())
}

/// The standard precision widths: 20 bits within the `f32` significand, 50 within the `f64`
/// significand, 100 where the significand fits in a double word, and 500 in the general
/// multi-word range.
#[test]
fn div_precision_sweep() -> Result<(), Mismatch> {
    let (a, b) = integers("6", "11");
    let (c, d) = integers("-493801", "4");
    let cases = [
        (a, 0isize, b, 0isize),
        (c, 0, d, 0),
        (IBig::from(UBig::ONE << 517), 0, IBig::from(UBig::from(3u8)).pow(300), 0),
        (IBig::from(&UBig::from(10u8).pow(200) + UBig::ONE), 0, IBig::from(997u32), 0),
        (
            IBig::from((UBig::ONE << 1499) / 7u8),
            -3,
            IBig::from((&UBig::ONE << 1031) / 11u8),
            5,
        ),
    ];
    for precision in [20usize, 50, 100, 500] {
        for (num, exp_num, den, exp_den) in &cases {
            check_case::<2>(&(num.clone(), *exp_num, den.clone(), *exp_den, precision))?;
            check_int_case::<2>(num, &den.unsigned_abs(), precision)?;
        }
    }
    Ok(())
}
