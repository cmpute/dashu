//! Parse in a non-power-of-two radix.

use crate::{
    arch::word::Word,
    buffer::Buffer,
    mul,
    radix::{self, Digit},
    repr::Repr,
    ubig::UBig,
};
use alloc::vec;
use dashu_base::ParseError;

/// Parse in chunks of CHUNK_LEN * digits_per_word.
const CHUNK_LEN: usize = 256;

/// Parse an unsigned string to [UBig].
pub fn parse(src: &str, radix: Digit) -> Result<UBig, ParseError> {
    debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
    let radix_info = radix::radix_info(radix);
    let mut bytes = src.as_bytes();
    let stripped: vec::Vec<u8>;

    // strip all underscores if detected
    if bytes.contains(&b'_') {
        stripped = bytes.iter().copied().filter(|&c| c != b'_').collect();
        bytes = &stripped;
    }

    if bytes.len() <= radix_info.digits_per_word {
        Ok(parse_word(bytes, radix)?.into())
    } else if bytes.len() <= CHUNK_LEN * radix_info.digits_per_word {
        parse_chunk(bytes, radix)
    } else {
        parse_large(bytes, radix)
    }
}

/// Parse an unsigned string to `Word`.
///
/// The length of the string must be at most `digits_per_word`.
fn parse_word(src: &[u8], radix: Digit) -> Result<Word, ParseError> {
    debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
    debug_assert!(src.len() <= radix::radix_info(radix).digits_per_word);

    let mut word: Word = 0;
    for byte in src.iter() {
        let digit = radix::digit_from_ascii_byte(*byte, radix).ok_or(ParseError::InvalidDigit)?;
        word = word * (radix as Word) + (digit as Word);
    }
    Ok(word)
}

/// Parse an unsigned string to [UBig].
///
/// The length of input is limited to `CHUNK_LEN * digits_per_word`.
fn parse_chunk(bytes: &[u8], radix: Digit) -> Result<UBig, ParseError> {
    debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
    let radix_info = radix::radix_info(radix);
    debug_assert!(bytes.len() <= CHUNK_LEN * radix_info.digits_per_word);

    let groups = bytes.rchunks(radix_info.digits_per_word);
    let mut buffer = Buffer::allocate(groups.len());
    for group in groups.rev() {
        let next = parse_word(group, radix)?;
        let carry = mul::mul_word_in_place_with_carry(&mut buffer, radix_info.range_per_word, next);
        if carry != 0 {
            buffer.push(carry);
        }
    }
    Ok(UBig(Repr::from_buffer(buffer)))
}

/// Parse an unsigned string to [UBig].
///
/// This result will usually not fit in CHUNK_LEN words.
fn parse_large(bytes: &[u8], radix: Digit) -> Result<UBig, ParseError> {
    debug_assert!(radix::is_radix_valid(radix) && !radix.is_power_of_two());
    let radix_info = radix::radix_info(radix);
    let chunk_bytes = CHUNK_LEN * radix_info.digits_per_word;
    debug_assert!(bytes.len() > chunk_bytes);

    // Calculate radix^(CHUNK_LEN<<i).
    let mut radix_powers = vec![UBig::from(radix_info.range_per_word).pow(CHUNK_LEN)];

    // while (chunk_bytes << radix_powers.len()) < bytes.len()
    // To avoid overflow:
    while chunk_bytes <= (bytes.len() - 1) >> radix_powers.len() {
        let prev = radix_powers.last().unwrap();
        let new = prev * prev;
        radix_powers.push(new);
    }

    parse_large_divide_conquer(bytes, radix, chunk_bytes, &radix_powers)
}

/// Convert an unsigned string to [UBig].
///
/// `radix_powers` contains radix^n for n = chunk digits << i
fn parse_large_divide_conquer(
    bytes: &[u8],
    radix: Digit,
    chunk_bytes: usize,
    radix_powers: &[UBig],
) -> Result<UBig, ParseError> {
    debug_assert!(bytes.len() <= chunk_bytes << radix_powers.len());

    match radix_powers.split_last() {
        None => parse_chunk(bytes, radix),
        Some((radix_power, radix_powers)) => {
            let bytes_lo_len = chunk_bytes << radix_powers.len();
            if bytes.len() <= bytes_lo_len {
                parse_large_divide_conquer(bytes, radix, chunk_bytes, radix_powers)
            } else {
                let (bytes_hi, bytes_lo) = bytes.split_at(bytes.len() - bytes_lo_len);
                let res_hi =
                    parse_large_divide_conquer(bytes_hi, radix, chunk_bytes, radix_powers)?;
                let res_lo =
                    parse_large_divide_conquer(bytes_lo, radix, chunk_bytes, radix_powers)?;
                Ok(res_hi * radix_power + res_lo)
            }
        }
    }
}
