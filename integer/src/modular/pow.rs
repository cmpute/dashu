use crate::{
    arch::word::Word,
    error::panic_allocate_too_much,
    math,
    memory::{self, MemoryAllocation},
    primitive::{double_word, split_dword, PrimitiveUnsigned, WORD_BITS, WORD_BITS_USIZE},
    repr::TypedReprRef::*,
    ubig::UBig,
};

use super::{
    modulo::{Modulo, ModuloDoubleRaw, ModuloLargeRaw, ModuloRepr, ModuloSingleRaw},
    modulo_ring::{ModuloRingDouble, ModuloRingLarge, ModuloRingSingle},
};

impl<'a> Modulo<'a> {
    /// Exponentiation.
    ///
    /// If you want use negative exponent, you can first use [inv()][Self::inv] to
    /// convert the base to its inverse, and then call this method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, UBig};
    /// // A Mersenne prime.
    /// let p = UBig::from(2u8).pow(607) - UBig::ONE;
    /// let ring = ModuloRing::new(p.clone());
    /// // Fermat's little theorem: a^(p-1) = 1 (mod p)
    /// let a = ring.convert(123);
    /// assert_eq!(a.pow(&(p - UBig::ONE)), ring.convert(1));
    /// ```
    #[inline]
    pub fn pow(&self, exp: &UBig) -> Modulo<'a> {
        match self.repr() {
            ModuloRepr::Single(raw, ring) => Modulo::from_single(ring.pow(*raw, exp), ring),
            ModuloRepr::Double(raw, ring) => Modulo::from_double(ring.pow(*raw, exp), ring),
            ModuloRepr::Large(raw, ring) => Modulo::from_large(ring.pow(raw, exp), ring),
        }
    }
}

macro_rules! impl_mod_pow_for_primitive {
    ($ring:ty, $raw:ty) => {
        impl $ring {
            #[inline]
            pub const fn pow_word(&self, raw: $raw, exp: Word) -> $raw {
                match exp {
                    0 => <$raw>::one(self),
                    1 => raw, // no-op
                    2 => self.sqr(raw),
                    _ => {
                        let bits = WORD_BITS - 1 - exp.leading_zeros();
                        self.pow_helper(raw, raw, exp, bits)
                    }
                }
            }

            /// lhs^2^bits * rhs^exp[..bits] (in the modulo ring)
            #[inline]
            const fn pow_helper(&self, lhs: $raw, rhs: $raw, exp: Word, mut bits: u32) -> $raw {
                let mut res = lhs;
                while bits > 0 {
                    res = self.sqr(res);
                    bits -= 1;
                    if exp & (1 << bits) != 0 {
                        res = self.mul(res, rhs);
                    }
                }
                res
            }

            /// Exponentiation.
            #[inline]
            pub fn pow(&self, raw: $raw, exp: &UBig) -> $raw {
                match exp.repr() {
                    RefSmall(dword) => {
                        let (lo, hi) = split_dword(dword);
                        if hi == 0 {
                            self.pow_word(raw, lo)
                        } else {
                            let res = self.pow_word(raw, hi);
                            self.pow_helper(res, raw, lo, WORD_BITS)
                        }
                    }
                    RefLarge(words) => self.pow_nontrivial(raw, words),
                }
            }

            fn pow_nontrivial(&self, raw: $raw, exp_words: &[Word]) -> $raw {
                let mut n = exp_words.len() - 1;
                let mut res = self.pow_word(raw, exp_words[n]); // apply the top word
                while n != 0 {
                    n -= 1;
                    res = self.pow_helper(res, raw, exp_words[n], WORD_BITS);
                }
                res
            }
        }
    };
}
impl_mod_pow_for_primitive!(ModuloRingSingle, ModuloSingleRaw);
impl_mod_pow_for_primitive!(ModuloRingDouble, ModuloDoubleRaw);

impl ModuloRingLarge {
    pub fn pow(&self, raw: &ModuloLargeRaw, exp: &UBig) -> ModuloLargeRaw {
        if exp.is_zero() {
            ModuloLargeRaw::one(self)
        } else if exp.is_one() {
            raw.clone()
        } else {
            self.pow_nontrivial(raw, exp)
        }
    }

    fn pow_nontrivial(&self, raw: &ModuloLargeRaw, exp: &UBig) -> ModuloLargeRaw {
        let n = self.normalized_modulus().len();
        let window_len = Self::choose_pow_window_len(exp.bit_len());

        // Precomputed table of small odd powers up to 2^window_len, starting from raw^3.
        #[allow(clippy::redundant_closure)]
        let table_words = ((1usize << (window_len - 1)) - 1)
            .checked_mul(n)
            .unwrap_or_else(|| panic_allocate_too_much());

        let memory_requirement = memory::add_layout(
            memory::array_layout::<Word>(table_words),
            self.mul_memory_requirement(),
        );
        let mut allocation = MemoryAllocation::new(memory_requirement);
        let mut memory = allocation.memory();
        let (table, mut memory) = memory.allocate_slice_fill::<Word>(table_words, 0);

        // val = raw^2
        let mut val = raw.clone();
        self.sqr_in_place(&mut val, &mut memory);

        // raw^(2*i+1) = raw^(2*i-1) * val
        for i in 1..(1 << (window_len - 1)) {
            let (prev, cur) = if i == 1 {
                (raw.0.as_ref(), &mut table[0..n])
            } else {
                let (prev, cur) = (&mut table[(i - 2) * n..i * n]).split_at_mut(n);
                (&*prev, cur)
            };
            cur.copy_from_slice(self.mul_normalized(prev, &val.0, &mut memory));
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
                    self.sqr_in_place(&mut val, &mut memory);
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
                let prod = self.mul_normalized(&val.0, entry, &mut memory);
                val.0.copy_from_slice(prod);
            }
            // val = raw ^ exp[bit..]
            if bit == 0 {
                break;
            }
            bit -= 1;
            self.sqr_in_place(&mut val, &mut memory);
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
        let ring = ModuloRingSingle::new(100);
        let modulo = ModuloSingleRaw::from_word(17, &ring);
        assert_eq!(ring.pow_word(modulo, 0).residue(&ring), 1);
        assert_eq!(ring.pow_word(modulo, 15).residue(&ring), 93);
    }
}
