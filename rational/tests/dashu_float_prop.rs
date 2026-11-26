//! Property tests for the rounding-aware conversion from rationals to floats (`to_float`).
//!
//! The conversion promises correct rounding, so every result is checked against
//!
//! * an exact reference computed with integer arithmetic, stating each rounding mode's rule in
//!   its own terms — the value, the exactness flag and the reported rounding direction must all
//!   agree — and
//! * the float crate's division kernel, run on the same numerator and denominator.
//!
//! The reference truncates straight to the target digit count, whereas the implementation
//! bounds its division to one guard digit and carries everything below it as sticky
//! information. A double rounding — rounding the remainder before dropping the guard digit —
//! therefore shows up as a disagreement; that was the defect reported in issue #125.

use core::{cmp::Ordering, fmt};

use dashu_base::{Approximation::*, BitTest, DivRem, Sign};
use dashu_float::{
    round::{mode, Round, Rounded, Rounding},
    Context, FBig, Repr as FBigRepr, Word,
};
use dashu_int::{IBig, UBig};
use dashu_ratio::{RBig, Relaxed};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Reference
// ---------------------------------------------------------------------------

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

/// A rounding mode's rule stated in its own terms: given the sign of the value, the parity of
/// the truncated significand, how twice the dropped part compares with the unit of the last
/// kept digit, and whether that part is nonzero, return the adjustment (-1, 0 or 1) to the
/// truncated significand.
type Rule = fn(Sign, bool, Ordering, bool) -> i8;

fn away_from_zero(sign: Sign) -> i8 {
    if sign == Sign::Negative {
        -1
    } else {
        1
    }
}

fn round_zero(_: Sign, _: bool, _: Ordering, _: bool) -> i8 {
    0
}

fn round_away(sign: Sign, _: bool, _: Ordering, nonzero: bool) -> i8 {
    if nonzero {
        away_from_zero(sign)
    } else {
        0
    }
}

fn round_up(sign: Sign, _: bool, _: Ordering, nonzero: bool) -> i8 {
    if nonzero && sign == Sign::Positive {
        1
    } else {
        0
    }
}

fn round_down(sign: Sign, _: bool, _: Ordering, nonzero: bool) -> i8 {
    if nonzero && sign == Sign::Negative {
        -1
    } else {
        0
    }
}

fn round_half_even(sign: Sign, odd: bool, half: Ordering, _: bool) -> i8 {
    match half {
        Ordering::Less => 0,
        Ordering::Greater => away_from_zero(sign),
        Ordering::Equal => {
            if odd {
                away_from_zero(sign)
            } else {
                0
            }
        }
    }
}

fn round_half_away(sign: Sign, _: bool, half: Ordering, nonzero: bool) -> i8 {
    match half {
        Ordering::Less => 0,
        Ordering::Equal | Ordering::Greater if nonzero => away_from_zero(sign),
        _ => 0,
    }
}

fn to_rounding(adjust: i8) -> Rounding {
    match adjust {
        0 => Rounding::NoOp,
        1 => Rounding::AddOne,
        -1 => Rounding::SubOne,
        _ => unreachable!(),
    }
}

/// Whether `mag >= den·B^e`.
fn scaled_at_least<const B: Word>(mag: &UBig, den: &UBig, e: isize) -> bool {
    let base = UBig::from_word(B);
    if e >= 0 {
        mag >= &(den * base.pow(e as usize))
    } else {
        &(mag.clone() * base.pow((-e) as usize)) >= den
    }
}

/// Exact reference for `num / den` rounded to `precision` significant base-`B` digits under
/// `rule`: the rounded significand with its exponent (`significand·B^exponent`), whether the
/// value is representable in `precision` digits, and the adjustment that was applied.
fn reference<const B: Word>(
    num: &IBig,
    den: &UBig,
    precision: usize,
    rule: Rule,
) -> (IBig, isize, bool, i8) {
    assert!(!num.is_zero() && !den.is_zero());
    let base = UBig::from_word(B);
    let (sign, mag) = num.clone().into_parts();

    // The leading digit sits at e = floor(log_B(|num| / den)), the largest e with
    // |num| >= den·B^e. The digit counts of the operands leave only two candidates, and the
    // right one is picked by an exact comparison.
    let estimate = mag.ilog(&base) as isize - den.ilog(&base) as isize;
    let leading = if scaled_at_least::<B>(&mag, den, estimate) {
        estimate
    } else {
        estimate - 1
    };

    // The last kept digit has weight B^s with s = leading − precision + 1.
    let s = leading - precision as isize + 1;
    let (scaled_num, scaled_den) = if s >= 0 {
        (mag.clone(), den * base.pow(s as usize))
    } else {
        (mag * base.pow((-s) as usize), den.clone())
    };

    // |num / den / B^s| = truncated + dropped / scaled_den with 0 <= dropped < scaled_den
    let (truncated, dropped) = scaled_num.div_rem(&scaled_den);
    let representable = dropped.is_zero();
    let half = (&dropped << 1).cmp(&scaled_den);
    let adjust = rule(sign, truncated.bit(0), half, !representable);
    (IBig::from_parts(sign, truncated) + IBig::from(adjust), s, representable, adjust)
}

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

/// Check one conversion against the reference and against the division kernel.
fn verify<R: Round, const B: Word>(
    num: &IBig,
    den: &UBig,
    precision: usize,
    rule: Rule,
    got: Rounded<FBig<R, B>>,
) -> Result<(), Mismatch> {
    let (significand, exponent, representable, adjust) = reference::<B>(num, den, precision, rule);
    let context = Context::<R>::new(precision);
    let expected = FBig::from_repr(FBigRepr::<B>::new(significand, exponent), context);
    let expected = if representable {
        Exact(expected)
    } else {
        Inexact(expected, to_rounding(adjust))
    };
    if got != expected {
        return Err(Mismatch(format!("got {got:?}, expected {expected:?}")));
    }
    if got.value_ref().precision() != precision || got.value_ref().digits() > precision {
        return Err(Mismatch(format!(
            "the result has precision {} and {} digits; expected precision {precision} \
             and at most as many digits",
            got.value_ref().precision(),
            got.value_ref().digits()
        )));
    }

    // The float crate's division kernel decides the same rounding from the two operands.
    let dividend = FBigRepr::<B>::new(num.clone(), 0);
    let divisor = FBigRepr::<B>::new(IBig::from_parts(Sign::Positive, den.clone()), 0);
    let divided = context
        .div(&dividend, &divisor)
        .map_err(|e| Mismatch(format!("the division kernel failed: {e:?}")))?;
    if got != divided {
        return Err(Mismatch(format!("got {got:?}, the division kernel gives {divided:?}")));
    }
    Ok(())
}

/// `check_one!(BASE, MODE, RULE, value, precision)`: check `value` under one mode in one base.
macro_rules! check_one {
    ($base:literal, $mode:ident, $rule:ident, $value:expr, $precision:expr) => {{
        let value = $value;
        let precision = $precision;
        let context = format!(
            "{:?} / {:?} to {precision} digits in base {}, mode {}",
            value.numerator(),
            value.denominator(),
            $base,
            stringify!($mode),
        );
        verify::<mode::$mode, $base>(
            value.numerator(),
            value.denominator(),
            precision,
            $rule,
            value.to_float::<mode::$mode, $base>(precision),
        )
        .map_err(|e| Mismatch(format!("{context}: {e}")))
    }};
}

/// Run the checks under every built-in rounding mode in base `$base`: expands to
/// `$body!($base, MODE, RULE, rest...)` for each mode, joined into one `Result<(), String>`.
macro_rules! for_each_mode {
    ($body:ident, $base:literal, $($arg:tt)*) => {{
        $body!($base, Zero, round_zero, $($arg)*)?;
        $body!($base, Away, round_away, $($arg)*)?;
        $body!($base, Up, round_up, $($arg)*)?;
        $body!($base, Down, round_down, $($arg)*)?;
        $body!($base, HalfEven, round_half_even, $($arg)*)?;
        $body!($base, HalfAway, round_half_away, $($arg)*)?;
        Ok::<(), Mismatch>(())
    }};
}

/// Check `value` under every built-in rounding mode, in every tested base: binary and decimal
/// (the common cases), an odd base, a power-of-two base wider than one bit, and an even base
/// with several trailing zero bits that is not a power of two.
macro_rules! check_all_modes {
    ($value:expr, $precision:expr $(,)?) => {{
        let value = $value;
        let precision = $precision;
        for_each_mode!(check_one, 2, &value, precision)?;
        for_each_mode!(check_one, 3, &value, precision)?;
        for_each_mode!(check_one, 10, &value, precision)?;
        for_each_mode!(check_one, 12, &value, precision)?;
        for_each_mode!(check_one, 16, &value, precision)?;
        Ok::<(), Mismatch>(())
    }};
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

/// A random (numerator, denominator, precision) with operands of up to `max_bits` bits. The
/// operand sizes are drawn independently, so the numerator is sometimes narrower than the
/// precision needs (it is padded with zero digits) and sometimes far wider (its low digits are
/// cut off in bulk).
fn random_case(
    max_bits: usize,
    max_precision: usize,
) -> impl Strategy<Value = (IBig, UBig, usize)> {
    (1..max_bits, 1..max_bits, any::<bool>(), 1..max_precision)
        .prop_flat_map(|(num_bits, den_bits, negative, precision)| {
            (unsigned(num_bits), unsigned(den_bits), Just((negative, precision)))
        })
        .prop_map(|(num, den, (negative, precision))| (signed(num, negative), den, precision))
}

/// Values whose numerator is far wider than the precision needs, so that the low digits of the
/// quotient are cut off in bulk and reach the rounding only as sticky information. The width
/// overshoots `precision + denominator digits` in every tested base (base 16 is the widest).
fn wide_numerator_case() -> impl Strategy<Value = (IBig, UBig, usize)> {
    (1..=120usize, 1..=200usize, 1..=400usize, any::<bool>())
        .prop_flat_map(|(precision, den_bits, excess, negative)| {
            let num_bits = 4 * precision + den_bits + excess;
            (unsigned(num_bits), unsigned(den_bits), Just((negative, precision)))
        })
        .prop_map(|(num, den, (negative, precision))| (signed(num, negative), den, precision))
}

/// Values right above a rounding boundary that lies within the digits cut off in bulk:
/// (k + 1/2 + 1/4)·B^m with `k` of `precision` digits. The integer part of the quotient ends
/// exactly on the boundary, so only the remainder tells that the value lies above it. A
/// randomly chosen wide numerator practically never lands on this pattern.
fn wide_tie<const B: Word>() -> impl Strategy<Value = (IBig, UBig, usize)> {
    (
        1..=40usize,  // precision
        1..=12usize,  // digits cut off
        3..=100usize, // denominator bits
        any::<bool>(),
    )
        .prop_flat_map(move |(precision, m, den_bits, negative)| {
            let span = UBig::from_word(B - 1) * UBig::from_word(B).pow(precision - 1);
            (
                unsigned(span.bit_len()),
                unsigned(den_bits),
                Just((precision, m, negative, span)),
            )
        })
        .prop_map(move |(random, den, (precision, m, negative, span))| {
            let base = UBig::from_word(B);
            let k = base.pow(precision - 1) + (random % &span);
            // (k + 1/2)·B^m + c / den = [(2k + 1)·B^m·den + 2c] / (2·den) with c = den / 4
            let numer = ((&k + &k + UBig::ONE) * base.pow(m)) * &den + (&den >> 1 >> 1);
            (signed(numer, negative), den * UBig::from(2u8), precision)
        })
}

/// Values on and around a rounding boundary of base `B`: (k + 1/2 + offset / 2q)·B^s with `k`
/// having exactly `precision` digits, so that the boundary separates the two candidates for the
/// `precision`-digit result, and the offset (when nonzero) a tiny fraction of the last kept
/// digit's unit. Both candidates and the boundary itself are exercised.
fn near_tie<const B: Word>() -> impl Strategy<Value = (IBig, UBig, usize)> {
    (
        1..=40usize,   // precision
        -12..=12isize, // exponent of the unit
        -1i32..=1,     // position relative to the boundary
        1..=1000u32,   // makes the offset tiny compared with the unit
        any::<bool>(),
    )
        .prop_flat_map(move |(precision, s, offset, q, negative)| {
            // k = B^(precision-1) + (a random value below B^(precision-1)·(B − 1))
            let span = UBig::from_word(B - 1) * UBig::from_word(B).pow(precision - 1);
            let span_bits = span.bit_len();
            (unsigned(span_bits), Just((precision, s, offset, q, negative, span)))
        })
        .prop_map(move |(random, (precision, s, offset, q, negative, span))| {
            let base = UBig::from_word(B);
            let k = base.pow(precision - 1) + (random % &span);
            let numer = (&k + &k + UBig::ONE) * UBig::from(q) + UBig::from(offset.unsigned_abs());
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
            (numer, denom, precision)
        })
}

/// A random (numerator, denominator, common factor, precision) for a value that is stored
/// unreduced.
fn unreduced_case() -> impl Strategy<Value = (IBig, UBig, UBig, usize)> {
    (1..=80usize, 1..=80usize, 1..=40usize, any::<bool>(), 1..=60usize)
        .prop_flat_map(|(num_bits, den_bits, factor_bits, negative, precision)| {
            (
                unsigned(num_bits),
                unsigned(den_bits),
                unsigned(factor_bits),
                (Just(negative), Just(precision)),
            )
        })
        .prop_map(|(num, den, factor, (negative, precision))| {
            (signed(num, negative), den, factor, precision)
        })
}

fn rbig(num: &str, den: &str) -> RBig {
    RBig::from_parts(num.parse().unwrap(), den.parse().unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..Default::default() })]

    /// Random reduced rationals of every shape.
    #[test]
    fn to_float_matches_reference((num, den, precision) in random_case(600, 100)) {
        let value = RBig::from_parts(num, den);
        check_all_modes!(value, precision).unwrap();
    }

    /// Values on both sides of a rounding boundary, and exactly on it.
    #[test]
    fn to_float_matches_reference_near_ties(
        (num, den, precision) in prop_oneof![
            near_tie::<2>(),
            near_tie::<3>(),
            near_tie::<10>(),
            near_tie::<12>(),
            near_tie::<16>()
        ],
    ) {
        let value = RBig::from_parts(num, den);
        check_all_modes!(value, precision).unwrap();
    }

    /// Numerators much wider than the precision: the low digits are cut off in bulk, at random
    /// and right above a boundary within the cut-off digits.
    #[test]
    fn to_float_matches_reference_wide_numerator(
        (num, den, precision) in prop_oneof![
            wide_numerator_case(),
            wide_tie::<2>(),
            wide_tie::<3>(),
            wide_tie::<10>(),
            wide_tie::<12>(),
            wide_tie::<16>()
        ],
    ) {
        let value = RBig::from_parts(num, den);
        check_all_modes!(value, precision).unwrap();
    }

    /// Values stored unreduced, whose numerator and denominator share a common factor.
    #[test]
    fn to_float_matches_reference_unreduced((num, den, factor, precision) in unreduced_case()) {
        let value = Relaxed::from_parts(&num * &factor, &den * &factor);
        check_all_modes!(value, precision).unwrap();
    }
}

/// Fixed cases: the reported regressions, exact midpoints (next to a digit carry or not),
/// values that terminate, and wide numerators whose low digits are cut off in bulk.
#[test]
fn to_float_fixed_cases() -> Result<(), Mismatch> {
    let cases = [
        ("6", "11", 16), // 0.5454…: just above the tie 0.5454545454545454|5
        ("-6", "11", 16),
        ("493801", "4", 4), // 123450.25: just above the tie 1234|50
        ("-493801", "4", 4),
        ("31", "3", 2),   // 1010.0101…b: just above the tie between 1000b and 1100b
        ("28", "3", 3),   // 1001.0101…b: just above the tie between 1000b and 1010b
        ("19", "2", 1),   // 9.5: a tie next to a digit carry
        ("199", "2", 2),  // 99.5
        ("1999", "2", 3), // 999.5
        ("-199", "2", 2),
        ("19", "20", 1),   // 0.95
        ("199", "200", 2), // 0.995
        ("21", "2", 1),    // 10.5: a tie whose even candidate carries a digit
        ("1", "2", 1),
        ("1", "8", 4),
        ("3", "4", 2),
        ("1", "3", 1),
        ("1", "3", 7),
        ("-1", "3", 4),
        ("2", "3", 5),
        ("1", "7", 50),
        ("22", "7", 10),
        ("10", "1", 1),
        ("1000", "1", 4),
        ("123456789", "1", 4),
        ("123450", "1", 4),
        ("123400", "1", 4),
    ];
    for (num, den, precision) in cases {
        check_all_modes!(rbig(num, den), precision)?;
    }

    // (10^100 + 1) / 3^60; a 600-bit numerator over a small denominator; a value far below 1;
    // and a negative wide numerator
    let cases = [
        (IBig::from(&UBig::from(10u8).pow(100) + UBig::ONE), UBig::from(3u8).pow(60), 37),
        (IBig::from((UBig::ONE << 599) + UBig::from(12345u32)), UBig::from(343u32), 50),
        (IBig::from(1u8), (&UBig::ONE << 700) | UBig::ONE, 50),
        (-IBig::from((&UBig::ONE << 1023) + UBig::ONE), UBig::from(1009u32), 9),
    ];
    for (num, den, precision) in cases {
        check_all_modes!(RBig::from_parts(num, den), precision)?;
    }
    Ok(())
}

/// The standard precision widths: 20 bits within the `f32` significand, 50 within the `f64`
/// significand, 100 where the significand fits in a double word, and 500 in the general
/// multi-word range.
#[test]
fn to_float_precision_sweep() -> Result<(), Mismatch> {
    let values = [
        rbig("6", "11"),
        rbig("-493801", "4"),
        RBig::from_parts(IBig::from(UBig::ONE << 517), UBig::from(3u8).pow(300)),
        RBig::from_parts(IBig::from(&UBig::from(10u8).pow(200) + UBig::ONE), UBig::from(997u32)),
    ];
    for precision in [20usize, 50, 100, 500] {
        for value in &values {
            for_each_mode!(check_one, 2, value, precision)?;
        }
    }
    Ok(())
}
