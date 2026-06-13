//! Bit-level packing / unpacking of `b`-bit coefficients.

use crate::arch::ntt::Lane;
use crate::arch::word::Word;

/// Pack a big integer (given as `&[Word]`, little-endian) into `out`,
/// producing `n` coefficients of `b_pack` bits each, zero-padded.
///
/// Each coefficient `c_i` satisfies `0 ≤ c_i < 2^{b_pack}`.
/// Panics if `out.len() < n`.
pub fn pack(out: &mut [Lane], words: &[Word], b_pack: u32, n: usize) {
    assert!(out.len() >= n);

    // Fast path: one coefficient per word, no bit shifting needed.
    if b_pack == Word::BITS {
        let len = words.len().min(n);
        #[allow(clippy::unnecessary_cast)]
        // SAFETY: NTT path requires Word and Lane have the same size.
        let words_lane = unsafe { &*(words as *const [Word] as *const [Lane]) };
        out[..len].copy_from_slice(&words_lane[..len]);
        out[len..n].fill(0);
        return;
    }

    let mask = if b_pack < Word::BITS {
        (1u64 << b_pack) - 1
    } else {
        u64::MAX
    };
    let word_bits = Word::BITS;
    let mut word_idx = 0usize;
    let mut bit_offset = 0u32;

    for coeff in out.iter_mut().take(n) {
        if word_idx >= words.len() {
            *coeff = 0;
            continue;
        }

        if bit_offset + b_pack <= word_bits {
            *coeff = ((words[word_idx] >> bit_offset) & mask) as Lane;
            bit_offset += b_pack;
            if bit_offset == word_bits {
                bit_offset = 0;
                word_idx += 1;
            }
        } else {
            let bits_first = word_bits - bit_offset;
            let bits_second = b_pack - bits_first;
            let mut val =
                (words[word_idx] >> bit_offset) & ((1u64 << bits_first) - 1);
            word_idx += 1;
            if word_idx < words.len() {
                val |= (words[word_idx] & ((1u64 << bits_second) - 1))
                    << bits_first;
            }
            *coeff = val as Lane;
            bit_offset = bits_second;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    /// Accumulate CRT-recovered convolution coefficients into the output limb
    /// array with carry propagation.
    ///
    /// Each coefficient `c_k` contributes `c_k << (k * b_pack)` bits to the
    /// output.  `output` must have capacity for `c.len()` coefficients plus any
    /// carry overflow.
    fn unpack_accumulate(output: &mut [Word], coeffs: &[Lane], b_pack: u32, output_len: usize) {
        let word_bits = Word::BITS;

        for (k, &coeff) in coeffs.iter().enumerate().take(output_len) {
            if coeff == 0 {
                continue;
            }
            let shift_bits = (k as u32).wrapping_mul(b_pack);
            let word_idx = (shift_bits / word_bits) as usize;
            let bit_shift = shift_bits % word_bits;

            let lo = coeff;
            let mut carry: Word;
            let mut idx = word_idx;

            if bit_shift == 0 {
                let (sum, c) = output
                    .get(idx)
                    .copied()
                    .unwrap_or(0)
                    .overflowing_add(lo);
                carry = Word::from(c);
                if idx < output.len() {
                    output[idx] = sum;
                }
                idx += 1;
            } else {
                let lo_part = lo << bit_shift;
                let hi_part = if bit_shift > 0 { lo >> (64 - bit_shift) } else { 0 };

                let (sum, c1) = output
                    .get(idx)
                    .copied()
                    .unwrap_or(0)
                    .overflowing_add(lo_part);
                carry = Word::from(c1);
                if idx < output.len() {
                    output[idx] = sum;
                }
                idx += 1;

                let (sum2, c2) = output
                    .get(idx)
                    .copied()
                    .unwrap_or(0)
                    .overflowing_add(hi_part + carry);
                carry = Word::from(c2);
                if idx < output.len() {
                    output[idx] = sum2;
                }
                idx += 1;
            }

            // Propagate remaining carry
            while carry != 0 && idx < output.len() {
                let (sum, c) = output[idx].overflowing_add(carry);
                output[idx] = sum;
                carry = Word::from(c);
                idx += 1;
            }
        }
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        let b_pack = 16u32;
        let test_words: Vec<Word> = vec![0xDEADBEEF_CAFEBABE, 0x12345678_9ABCDEF0];
        let coeffs_per_word = (Word::BITS / b_pack) as usize;
        let n = test_words.len() * coeffs_per_word;

        let mut packed = vec![0u64 as Lane; n];
        pack(&mut packed, &test_words, b_pack, n);

        let output_len = test_words.len() + 1;
        let mut output = vec![0u64 as Word; output_len];
        unpack_accumulate(&mut output, &packed, b_pack, n);
        assert_eq!(&output[..test_words.len()], &test_words[..]);
    }

    #[test]
    fn test_pack_zero_pads() {
        let words = vec![0xFFFFu64 as Word];
        let n = 32;
        let mut packed = vec![0u64 as Lane; n];
        pack(&mut packed, &words, 16, n);
        assert_eq!(packed[0], 0xFFFF);
        for &c in packed.iter().skip(1) {
            assert_eq!(c, 0);
        }
    }

    #[test]
    fn test_pack_empty_input() {
        let mut packed = vec![0u64 as Lane; 8];
        pack(&mut packed, &[], 16, 8);
        assert_eq!(packed, vec![0u64 as Lane; 8]);
    }

    #[test]
    fn test_unpack_single_coeff() {
        let mut output = vec![0u64 as Word; 2];
        unpack_accumulate(&mut output, &[0xABCD], 16, 1);
        assert_eq!(output[0], 0xABCD);
        assert_eq!(output[1], 0);
    }

    #[test]
    fn test_unpack_carry_propagation() {
        // Coefficient at k=4 (shift by 64 bits = 1 word) + carry
        let mut output = vec![0u64 as Word; 3];
        unpack_accumulate(&mut output, &[0, 0, 0, 0, 1], 16, 5);
        assert_eq!(output[0], 0);
        assert_eq!(output[1], 1);
        assert_eq!(output[2], 0);
    }
}
