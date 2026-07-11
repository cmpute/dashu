//! Consolidated unit tests for the architecture-specific primitives.
//!
//! `arch::add` and `arch::digits` are re-exported (in `mod.rs`) from whichever
//! `arch_impl` is active — `generic_*_bit` under `force_bits`, or `x86`/`x86_64`
//! on those native targets. So this single suite exercises the *active* backend
//! across the whole CI matrix, instead of duplicating the tests inside every
//! arch backend's `add.rs` / `digits.rs`.

use super::add::{add_with_carry, sub_with_borrow};
use super::digits::{digit_chunk_raw_to_ascii, DIGIT_CHUNK_LEN};
use super::word::{DoubleWord, Word};
use crate::radix::DigitCase;

// ---- add_with_carry / sub_with_borrow ----

/// Reference: split the full (a + b + carry) into low word + carry-out.
///
/// Carry/borrow inputs are always 0 or 1 (the documented contract; the x86/x86_64
/// intrinsics mask the input with `& 1`).
fn add_ref(a: Word, b: Word, carry: Word) -> (Word, Word) {
    let total = a as DoubleWord + b as DoubleWord + carry as DoubleWord;
    let low = (total & Word::MAX as DoubleWord) as Word;
    let high = (total >> Word::BITS) as Word;
    (low, high)
}

/// Reference: a - b - borrow via wrapping on a double-wide type.
fn sub_ref(a: Word, b: Word, borrow: Word) -> (Word, Word) {
    let lhs = a as DoubleWord;
    let rhs = b as DoubleWord + borrow as DoubleWord;
    let out = Word::from(lhs < rhs);
    (lhs.wrapping_sub(rhs) as Word, out)
}

#[test]
fn add_with_carry_small() {
    for a in 0u32..256 {
        for b in 0u32..256 {
            for c in [0u32, 1] {
                let (a, b, c) = (a as Word, b as Word, c as Word);
                assert_eq!(add_with_carry(a, b, c), add_ref(a, b, c));
            }
        }
    }
}

#[test]
fn sub_with_borrow_small() {
    for a in 0u32..256 {
        for b in 0u32..256 {
            for br in [0u32, 1] {
                let (a, b, br) = (a as Word, b as Word, br as Word);
                assert_eq!(sub_with_borrow(a, b, br), sub_ref(a, b, br));
            }
        }
    }
}

#[test]
fn add_sub_boundary() {
    let m = Word::MAX;
    for &(a, b, c) in &[
        (m, m, 1),
        (m, m, 0),
        (m, 0, 1),
        (0, m, 1),
        (m >> 1, m >> 1, 1),
        (1, 0, 0),
        (0, 0, 0),
    ] {
        assert_eq!(add_with_carry(a, b, c), add_ref(a, b, c));
    }
    for &(a, b, br) in &[
        (0, 0, 1),
        (0, m, 0),
        (0, m, 1),
        (m, m, 1),
        (1, 2, 0),
        (m >> 1, m, 0),
    ] {
        assert_eq!(sub_with_borrow(a, b, br), sub_ref(a, b, br));
    }
}

// ---- digit_chunk_raw_to_ascii (SWAR digit conversion) ----

/// Expected ASCII for a raw digit value under a given letter case — scalar
/// arithmetic that mirrors the spec the SWAR implements, so it independently
/// cross-checks the parallel byte-lane logic.
fn expected(digit: u8, case: DigitCase) -> u8 {
    let offset = if digit < 10 { 0 } else { case as u8 };
    b'0' + digit + offset
}

#[test]
fn raw_to_ascii_all_digits() {
    // NoLetters is only meaningful for decimal digits (0..10).
    let cases = [
        (DigitCase::NoLetters, 10u8),
        (DigitCase::Lower, 16),
        (DigitCase::Upper, 16),
    ];
    for (case, limit) in cases {
        for d in 0..limit {
            let mut chunk = [d; DIGIT_CHUNK_LEN];
            digit_chunk_raw_to_ascii(&mut chunk, case);
            for (i, &out) in chunk.iter().enumerate() {
                assert_eq!(out, expected(d, case), "byte {i}, digit {d}");
            }
        }
    }
}

#[test]
fn raw_to_ascii_mixed_chunk() {
    let mut chunk = [0u8; DIGIT_CHUNK_LEN];
    for (i, b) in chunk.iter_mut().enumerate() {
        *b = (i % 16) as u8;
    }
    digit_chunk_raw_to_ascii(&mut chunk, DigitCase::Lower);
    for (i, &out) in chunk.iter().enumerate() {
        assert_eq!(out, expected((i % 16) as u8, DigitCase::Lower), "byte {i}");
    }
}
