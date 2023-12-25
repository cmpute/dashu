use crate::{
    arch::word::Word,
    div_const::{ConstDoubleDivisor, ConstSingleDivisor},
    primitive::{split_dword, WORD_BITS},
    repr::TypedReprRef::*,
    ubig::UBig,
};

use super::repr::{Reduced, ReducedDword, ReducedRepr, ReducedWord};
use num_modular::Reducer;

impl<'a> Reduced<'a> {
    /// Exponentiation.
    ///
    /// If you want use a negative exponent, you can first use [inv()][Self::inv] to
    /// convert the base to its inverse, and then call this method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{fast_div::ConstDivisor, UBig};
    /// // A Mersenne prime.
    /// let p = UBig::from(2u8).pow(607) - UBig::ONE;
    /// let ring = ConstDivisor::new(p.clone());
    /// // Fermat's little theorem: a^(p-1) = 1 (mod p)
    /// let a = ring.reduce(123);
    /// assert_eq!(a.pow(&(p - UBig::ONE)), ring.reduce(1));
    /// ```
    #[inline]
    pub fn pow(&self, exp: &UBig) -> Reduced<'a> {
        match self.repr() {
            ReducedRepr::Single(raw, ring) => {
                Reduced::from_single(single::pow(ring, *raw, exp), ring)
            }
            ReducedRepr::Double(raw, ring) => {
                Reduced::from_double(double::pow(ring, *raw, exp), ring)
            }
            ReducedRepr::Large(raw, ring) => Reduced::from_large(large::pow(ring, raw, exp), ring),
        }
    }
}

macro_rules! impl_mod_pow_for_primitive {
    ($ns:ident, $ring:ty, $raw:ident) => {
        mod $ns {
            use super::*;

            #[inline]
            pub(super) fn pow_word(ring: &$ring, raw: $raw, exp: Word) -> $raw {
                match exp {
                    0 => <$raw>::one(ring),
                    1 => raw, // no-op
                    2 => $raw(ring.0.sqr(raw.0)),
                    _ => {
                        let bits = WORD_BITS - 1 - exp.leading_zeros();
                        pow_helper(ring, raw, raw, exp, bits)
                    }
                }
            }

            /// lhs^2^bits * rhs^exp[..bits] (in the modulo ring)
            #[inline]
            fn pow_helper(ring: &$ring, lhs: $raw, rhs: $raw, exp: Word, mut bits: u32) -> $raw {
                let mut res = lhs;
                while bits > 0 {
                    res.0 = ring.0.sqr(res.0);
                    bits -= 1;
                    if exp & (1 << bits) != 0 {
                        res.0 = ring.0.mul(&res.0, &rhs.0);
                    }
                }
                res
            }

            /// Exponentiation.
            #[inline]
            pub(super) fn pow(ring: &$ring, raw: $raw, exp: &UBig) -> $raw {
                match exp.repr() {
                    RefSmall(dword) => {
                        let (lo, hi) = split_dword(dword);
                        if hi == 0 {
                            pow_word(ring, raw, lo)
                        } else {
                            let res = pow_word(ring, raw, hi);
                            pow_helper(ring, res, raw, lo, WORD_BITS)
                        }
                    }
                    RefLarge(words) => pow_nontrivial(ring, raw, words),
                }
            }

            fn pow_nontrivial(ring: &$ring, raw: $raw, exp_words: &[Word]) -> $raw {
                let mut n = exp_words.len() - 1;
                let mut res = pow_word(ring, raw, exp_words[n]); // apply the top word
                while n != 0 {
                    n -= 1;
                    res = pow_helper(ring, res, raw, exp_words[n], WORD_BITS);
                }
                res
            }
        }
    };
}
impl_mod_pow_for_primitive!(single, ConstSingleDivisor, ReducedWord);
impl_mod_pow_for_primitive!(double, ConstDoubleDivisor, ReducedDword);

mod large {
    use dashu_base::BitTest;

    use super::{
        super::mul::{mul_memory_requirement, mul_normalized, sqr_in_place},
        *,
    };
    use crate::{
        div_const::ConstLargeDivisor,
        error::panic_allocate_too_much,
        math,
        memory::{self, MemoryAllocation},
        modular::repr::ReducedLarge,
        primitive::{double_word, split_dword, PrimitiveUnsigned, WORD_BITS_USIZE},
    };

    pub(super) fn pow(ring: &ConstLargeDivisor, raw: &ReducedLarge, exp: &UBig) -> ReducedLarge {
        if exp.is_zero() {
            ReducedLarge::one(ring)
        } else if exp.is_one() {
            raw.clone()
        } else {
            pow_nontrivial(ring, raw, exp)
        }
    }

    fn pow_nontrivial(ring: &ConstLargeDivisor, raw: &ReducedLarge, exp: &UBig) -> ReducedLarge {
        let n = ring.normalized_divisor.len();
        let window_len = choose_pow_window_len(exp.bit_len());

        // Precomputed table of small odd powers up to 2^window_len, starting from raw^3.
        #[allow(clippy::redundant_closure)]
        let table_words = ((1usize << (window_len - 1)) - 1)
            .checked_mul(n)
            .unwrap_or_else(|| panic_allocate_too_much());

        let memory_requirement = memory::add_capacity(table_words, mul_memory_requirement(ring));
        let mut allocation = MemoryAllocation::new(memory_requirement);
        let mut memory = allocation.memory();
        let (table, mut memory) = memory.allocate_slice_fill(table_words, 0);

        // val = raw^2
        let mut val = raw.clone();
        sqr_in_place(ring, &mut val, &mut memory);

        // raw^(2*i+1) = raw^(2*i-1) * val
        for i in 1..(1 << (window_len - 1)) {
            let (prev, cur) = if i == 1 {
                (raw.0.as_ref(), &mut table[0..n])
            } else {
                let (prev, cur) = table[(i - 2) * n..i * n].split_at_mut(n);
                (&*prev, cur)
            };
            cur.copy_from_slice(mul_normalized(ring, prev, &val.0, &mut memory));
        }

        let exp_words = exp.as_words();
        // We already have raw^2 in val.
        // exp.bit_len() >= 2 because exp >= 2.
        let mut bit = exp.bit_len() - 2;

        loop {
            // val = raw ^ exp[bit..] ignoring the lowest bit
            let word_idx = bit / WORD_BITS_USIZE;
            let bit_idx = (bit % WORD_BITS_USIZE) as u32;
            let cur_word = exp_words[word_idx];
            if cur_word & (1 << bit_idx) != 0 {
                let next_word = if word_idx == 0 {
                    0
                } else {
                    exp_words[word_idx - 1]
                };
                // Get a window of window_len bits, with top bit of 1.
                let (mut window, _) = split_dword(
                    double_word(next_word, cur_word) >> (bit_idx + 1 + WORD_BITS - window_len),
                );
                window &= math::ones_word(window_len);
                // Shift right to make the window odd.
                let num_bits = window_len - window.trailing_zeros();
                window >>= window_len - num_bits;
                // val := val^2^(num_bits-1)
                for _ in 0..num_bits - 1 {
                    sqr_in_place(ring, &mut val, &mut memory);
                }
                bit -= (num_bits as usize) - 1;
                // Now val = raw ^ exp[bit..] ignoring the num_bits lowest bits.
                // val = val * raw^window from precomputed table.
                debug_assert!(window & 1 == 1);
                let entry_idx = (window >> 1) as usize;
                let entry = if entry_idx == 0 {
                    &raw.0
                } else {
                    &table[(entry_idx - 1) * n..entry_idx * n]
                };
                let prod = mul_normalized(ring, &val.0, entry, &mut memory);
                val.0.copy_from_slice(prod);
            }
            // val = raw ^ exp[bit..]
            if bit == 0 {
                break;
            }
            bit -= 1;
            sqr_in_place(ring, &mut val, &mut memory);
        }
        val
    }

    /// Choose the optimal window size for n-bit exponents.
    /// 1 <= window_size < min(WORD_BITS, usize::BIT_SIZE) inclusive.
    fn choose_pow_window_len(n: usize) -> u32 {
        // This won't overflow because cost(3) is already approximately usize::MAX / 4
        // and it can only grow by a factor of 2.
        let cost = |window_size| (1usize << (window_size - 1)) - 1 + n / (window_size as usize + 1);
        let mut window_size = 1;
        let mut c = cost(window_size);
        while window_size + 1 < WORD_BITS.min(usize::BIT_SIZE) {
            let c2 = cost(window_size + 1);
            if c <= c2 {
                break;
            }
            window_size += 1;
            c = c2;
        }
        window_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_word() {
        let ring = ConstSingleDivisor::new(100);
        let modulo = ReducedWord(ring.0.transform(17));
        assert_eq!(single::pow_word(&ring, modulo, 0).residue(&ring), 1);
        assert_eq!(single::pow_word(&ring, modulo, 15).residue(&ring), 93);
    }
}
