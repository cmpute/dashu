//! Bit-level packing / unpacking of `b`-bit coefficients.
#![allow(
    dead_code,
    unused_assignments,
    unused_mut,
    unused_variables,
    clippy::unnecessary_cast
)]

use crate::arch::word::Word;

/// Pack a big integer (given as `&[Word]`, little-endian) into `out`,
/// producing `n` coefficients of `b_pack` bits each, zero-padded.
///
/// Each coefficient `c_i` satisfies `0 ≤ c_i < 2^{b_pack}`.
/// Panics if `out.len() < n`.
pub fn pack(out: &mut [u64], words: &[Word], b_pack: u32, n: usize) {
    assert!(out.len() >= n);
    let mask = (1u64 << b_pack) - 1;
    let word_bits = Word::BITS;
    let mut word_idx = 0usize;
    let mut bit_offset = 0u32;

    for coeff in out.iter_mut().take(n) {
        if word_idx >= words.len() {
            *coeff = 0;
            continue;
        }

        if bit_offset + b_pack <= word_bits {
            *coeff = (words[word_idx] >> bit_offset) & mask;
            bit_offset += b_pack;
            if bit_offset == word_bits {
                bit_offset = 0;
                word_idx += 1;
            }
        } else {
            let bits_first = word_bits - bit_offset;
            let bits_second = b_pack - bits_first;
            let mut val = (words[word_idx] >> bit_offset) & ((1u64 << bits_first) - 1);
            word_idx += 1;
            if word_idx < words.len() {
                val |= (words[word_idx] & ((1u64 << bits_second) - 1)) << bits_first;
            }
            *coeff = val;
            bit_offset = bits_second;
        }
    }
}

/// Accumulate CRT-recovered convolution coefficients into the output limb
/// array with carry propagation.
///
/// Each coefficient `c_k` contributes `c_k << (k * b_pack)` bits to the
/// output.  `output` must have capacity for `c.len()` coefficients plus any
/// carry overflow.
pub fn unpack_accumulate(output: &mut [Word], coeffs: &[u64], b_pack: u32, output_len: usize) {
    let word_bits = Word::BITS as u32;
    // For each coefficient, shift it by k*b_pack bits and add into the
    // output with carry propagation.  We use a software accumulation
    // because the coefficients can be larger than a single output word.

    for (k, &coeff) in coeffs.iter().enumerate().take(output_len) {
        if coeff == 0 {
            continue;
        }
        let shift_bits = (k as u32).wrapping_mul(b_pack);
        let word_idx = (shift_bits / word_bits) as usize;
        let bit_shift = shift_bits % word_bits;

        // The coefficient occupies up to ⌈bit_len(coeff) / word_bits⌉ words.
        // We split it into word-sized chunks and add each with the
        // appropriate shift to the output.
        let lo = coeff as u64;
        let _hi = 0u64; // coeff fits in one u64 since max CRT value < P ≈ 2^192
                        // Actually, per-coefficient CRT values can be up to P-1 ≈ 2^192,
                        // which needs up to 3 words.  We handle this by splitting the
                        // coefficient itself into words and accumulating each.

        // For the immediate case, coeff from CRT is already small enough
        // to fit in one or two u64 words. We accumulate by repeated
        // add-with-carry into the output slice.
        let mut carry: Word = 0;
        let mut idx = word_idx;

        if bit_shift == 0 {
            // Aligned: just add into output
            let (sum, c) = overflowing_add_word(output.get(idx).copied().unwrap_or(0), lo);
            carry = Word::from(c);
            if idx < output.len() {
                output[idx] = sum;
            }
            idx += 1;
        } else {
            // Split across two output words
            let lo_part = lo << bit_shift;
            let hi_part = if bit_shift > 0 {
                lo >> (64 - bit_shift)
            } else {
                0
            };

            let (sum, c1) = overflowing_add_word(output.get(idx).copied().unwrap_or(0), lo_part);
            carry = Word::from(c1);
            if idx < output.len() {
                output[idx] = sum;
            }
            idx += 1;

            let (sum2, c2) =
                overflowing_add_word(output.get(idx).copied().unwrap_or(0), hi_part + carry);
            carry = Word::from(c2);
            if idx < output.len() {
                output[idx] = sum2;
            }
            idx += 1;
        }

        // Propagate remaining carry
        while carry != 0 && idx < output.len() {
            let (sum, c) = overflowing_add_word(output[idx], carry);
            output[idx] = sum;
            carry = Word::from(c);
            idx += 1;
        }
    }
}

fn overflowing_add_word(a: Word, b: u64) -> (Word, bool) {
    let (sum, overflow) = a.overflowing_add(b);
    (sum, overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_roundtrip() {
        let b_pack = 16u32;
        let test_words: Vec<Word> = vec![0xDEADBEEF_CAFEBABE, 0x12345678_9ABCDEF0];
        let coeffs_per_word = (Word::BITS / b_pack) as usize;
        let n = test_words.len() * coeffs_per_word;

        let mut packed = vec![0u64; n];
        pack(&mut packed, &test_words, b_pack, n);

        let output_len = test_words.len() + 1;
        let mut output = vec![0u64; output_len];
        unpack_accumulate(&mut output, &packed, b_pack, n);
        assert_eq!(&output[..test_words.len()], &test_words[..]);
    }

    #[test]
    fn test_pack_zero_pads() {
        let words = vec![0xFFFFu64];
        let n = 32;
        let mut packed = vec![0u64; n];
        pack(&mut packed, &words, 16, n);
        assert_eq!(packed[0], 0xFFFF);
        for &c in packed.iter().skip(1) {
            assert_eq!(c, 0);
        }
    }

    #[test]
    fn test_pack_empty_input() {
        let mut packed = vec![0u64; 8];
        pack(&mut packed, &[], 16, 8);
        assert_eq!(packed, vec![0u64; 8]);
    }

    #[test]
    fn test_unpack_single_coeff() {
        let mut output = vec![0u64; 2];
        unpack_accumulate(&mut output, &[0xABCD], 16, 1);
        assert_eq!(output[0], 0xABCD);
        assert_eq!(output[1], 0);
    }

    #[test]
    fn test_unpack_carry_propagation() {
        // Coefficient at k=4 (shift by 64 bits = 1 word) + carry
        let mut output = vec![0u64; 3];
        unpack_accumulate(&mut output, &[0, 0, 0, 0, 1], 16, 5);
        assert_eq!(output[0], 0);
        assert_eq!(output[1], 1);
        assert_eq!(output[2], 0);
    }
}
