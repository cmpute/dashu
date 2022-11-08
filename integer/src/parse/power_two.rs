//! Parse in a power-of-two radix.

use crate::{
    arch::word::Word,
    buffer::Buffer,
    primitive::{WORD_BITS, WORD_BITS_USIZE},
    radix::{self, Digit},
    repr::Repr,
    ubig::UBig,
};

use dashu_base::ParseError;

/// Parse an unsigned string to [UBig].
pub fn parse(src: &str, radix: Digit) -> Result<UBig, ParseError> {
    debug_assert!(radix::is_radix_valid(radix) && radix.is_power_of_two());

    let digits_per_word = (WORD_BITS / radix.trailing_zeros()) as usize;
    if src.len() <= digits_per_word {
        Ok(parse_word(src, radix)?.into())
    } else {
        parse_large(src, radix)
    }
}

/// Parse an unsigned string to `Word`.
///
/// The length of the string must be at most digits_per_word(radix).
fn parse_word(src: &str, radix: Digit) -> Result<Word, ParseError> {
    debug_assert!(radix::is_radix_valid(radix) && radix.is_power_of_two());
    debug_assert!(src.len() <= (WORD_BITS / radix.trailing_zeros()) as usize);

    let log_radix = radix.trailing_zeros();
    let mut word = 0;
    let mut bits = 0;
    for byte in src.as_bytes().iter().rev() {
        if *byte == b'_' {
            continue;
        }
        let digit = radix::digit_from_ascii_byte(*byte, radix).ok_or(ParseError::InvalidDigit)?;
        word |= (digit as Word) << bits;
        bits += log_radix;
    }
    Ok(word)
}

/// Parse an unsigned string to [UBig].
///
/// The result will usually not fit in a single word.
fn parse_large(src: &str, radix: Digit) -> Result<UBig, ParseError> {
    debug_assert!(radix::is_radix_valid(radix) && radix.is_power_of_two());

    let log_radix = radix.trailing_zeros();
    #[allow(clippy::redundant_closure)]
    let num_bits = src
        .len()
        .checked_mul(log_radix as usize)
        .expect("the number to be parsed is too large");
    let mut buffer = Buffer::allocate((num_bits - 1) / WORD_BITS_USIZE + 1);
    let mut bits = 0;
    let mut word = 0;
    for byte in src.as_bytes().iter().rev() {
        if *byte == b'_' {
            continue;
        }
        let digit = radix::digit_from_ascii_byte(*byte, radix).ok_or(ParseError::InvalidDigit)?;
        word |= (digit as Word) << bits;
        let new_bits = bits + log_radix;
        if new_bits >= WORD_BITS {
            buffer.push(word);
            word = (digit as Word) >> (WORD_BITS - bits);
            bits = new_bits - WORD_BITS;
        } else {
            bits = new_bits;
        }
    }
    if bits > 0 {
        buffer.push(word);
    }
    Ok(UBig(Repr::from_buffer(buffer)))
}
