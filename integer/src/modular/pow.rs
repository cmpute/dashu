use crate::{
    arch::word::Word,
    math,
    memory::{self, MemoryAllocation},
    modular::{
        modulo::{Modulo, ModuloLarge, ModuloRepr, ModuloSingle, ModuloSingleRaw},
        modulo_ring::ModuloRingSingle,
    },
    primitive::{double_word, split_dword, PrimitiveUnsigned, WORD_BITS, WORD_BITS_USIZE, shrink_dword},
    repr::TypedReprRef::*,
    ubig::UBig,
};

impl<'a> Modulo<'a> {
    /// Exponentiation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{modular::ModuloRing, ubig};
    /// // A Mersenne prime.
    /// let p = ubig!(2).pow(607) - ubig!(1);
    /// let ring = ModuloRing::new(&p);
    /// // Fermat's little theorem: a^(p-1) = 1 (mod p)
    /// let a = ring.from(123);
    /// assert_eq!(a.pow(&(p - ubig!(1))), ring.from(1));
    /// ```
    #[inline]
    pub fn pow(&self, exp: &UBig) -> Modulo<'a> {
        match self.repr() {
            ModuloRepr::Small(self_small) => ModuloSingle::new(
                self_small.ring().pow(self_small.raw(), exp),
                self_small.ring()
            ).into(),
            ModuloRepr::Large(self_large) => self_large.pow(exp).into(),
        }
    }
}

impl ModuloRingSingle {
    #[inline]
    pub const fn pow_word(&self, raw: ModuloSingleRaw, exp: Word) -> ModuloSingleRaw {
        match exp {
            0 => ModuloSingleRaw::from_word(1, &self),
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
    const fn pow_helper(
        &self,
        lhs: ModuloSingleRaw,
        rhs: ModuloSingleRaw,
        exp: Word,
        mut bits: u32,
    ) -> ModuloSingleRaw {
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
    pub fn pow(&self, raw: ModuloSingleRaw, exp: &UBig) -> ModuloSingleRaw{
        match exp.repr() {
            RefSmall(dword) => {
                let (lo, hi) = split_dword(dword);
                if hi == 0 {
                    self.pow_word(raw, lo)
                } else {
                    let res = self.pow_word(raw, hi);
                    self.pow_helper(res, res, lo, WORD_BITS)
                }
            },
            RefLarge(buffer) => self.pow_nontrivial(raw, buffer),
        }
    }

    fn pow_nontrivial(&self, raw: ModuloSingleRaw, exp_words: &[Word]) -> ModuloSingleRaw{
        let mut n = exp_words.len() - 1;
        let mut res = self.pow_word(raw, exp_words[n]); // apply the top word
        while n != 0 {
            n -= 1;
            res = self.pow_helper(res, raw, exp_words[n], WORD_BITS);
        }
        res
    }
}

impl<'a> ModuloLarge<'a> {
    fn pow(&self, exp: &UBig) -> ModuloLarge<'a> {
        match exp.repr() {
            // self^0 == 1
            RefSmall(0) => ModuloLarge::from_ubig(UBig::one(), self.ring()),
            // self^1 == self
            RefSmall(1) => self.clone(),
            _ => self.pow_nontrivial(exp),
        }
    }

    fn pow_nontrivial(&self, exp: &UBig) -> ModuloLarge<'a> {
        debug_assert!(*exp >= UBig::from(2u8));

        let n = self.ring().normalized_modulus().len();
        let window_len = ModuloLarge::choose_pow_window_len(exp.bit_len());

        // Precomputed table of small odd powers up to 2^window_len, starting from self^3.
        #[allow(clippy::redundant_closure)]
        let table_words = ((1usize << (window_len - 1)) - 1)
            .checked_mul(n)
            .unwrap_or_else(|| memory::panic_out_of_memory());

        let memory_requirement = memory::add_layout(
            memory::array_layout::<Word>(table_words),
            self.ring().mul_memory_requirement(),
        );
        let mut allocation = MemoryAllocation::new(memory_requirement);
        let mut memory = allocation.memory();
        let (table, mut memory) = memory.allocate_slice_fill::<Word>(table_words, 0);

        // val = self^2
        let mut val = self.clone();
        val.mul_in_place(self, &mut memory);

        // self^(2*i+1) = self^(2*i-1) * val
        for i in 1..(1 << (window_len - 1)) {
            let (prev, cur) = if i == 1 {
                (self.normalized_value(), &mut table[0..n])
            } else {
                let (prev, cur) = (&mut table[(i - 2) * n..i * n]).split_at_mut(n);
                (&*prev, cur)
            };
            cur.copy_from_slice(self.ring().mul_normalized(
                prev,
                val.normalized_value(),
                &mut memory,
            ));
        }

        let exp_words = exp.as_words();
        // We already have self^2 in val.
        // exp.bit_len() >= 2 because exp >= 2.
        let mut bit = exp.bit_len() - 2;

        loop {
            // val = self ^ exp[bit..] ignoring the lowest bit
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
                    val.square_in_place(&mut memory);
                }
                bit -= (num_bits as usize) - 1;
                // Now val = self ^ exp[bit..] ignoring the num_bits lowest bits.
                // val = val * self^window from precomputed table.
                debug_assert!(window & 1 == 1);
                let entry_idx = (window >> 1) as usize;
                let entry = if entry_idx == 0 {
                    self.normalized_value()
                } else {
                    &table[(entry_idx - 1) * n..entry_idx * n]
                };
                val.mul_normalized_in_place(entry, &mut memory);
            }
            // val = self ^ exp[bit..]
            if bit == 0 {
                break;
            }
            bit -= 1;
            val.square_in_place(&mut memory);
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
        let modulo = ModuloSingleRaw(17);
        assert_eq!(ring.pow_word(modulo, 0).residue(&ring), 1);
        assert_eq!(ring.pow_word(modulo, 15).residue(&ring), 93);
    }
}
