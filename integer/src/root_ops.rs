use crate::ubig::UBig;

impl UBig {
    #[inline]
    pub fn sqrt(&self) -> UBig {
        UBig(self.repr().sqrt_rem().0)
    }

    #[inline]
    pub fn sqrt_rem(&self) -> (UBig, UBig) {
        let (s, r) = self.repr().sqrt_rem();
        (UBig(s), UBig(r))
    }
}

mod repr {
    use dashu_base::RootRem;
    use crate::{repr::{TypedReprRef::{self, *}, Repr}, primitive::{shrink_dword, WORD_BITS, WORD_BITS_USIZE, extend_word}, arch::word::Word, buffer::Buffer, shift_ops, root, memory::MemoryAllocation, mul, add, shift};

    impl<'a> TypedReprRef<'a> {
        #[inline]
        pub fn sqrt_rem(self) -> (Repr, Repr) {
            match self {
                RefSmall(dw) => {
                    if let Some(w) = shrink_dword(dw) {
                        let (s, r) = w.sqrt_rem();
                        (Repr::from_word(s), Repr::from_word(r))
                    } else {
                        let (s, r) = dw.sqrt_rem();
                        (Repr::from_dword(s), Repr::from_dword(r))
                    }
                },
                RefLarge(words) => sqrt_rem_large(words)
            }
        }
    }

    fn sqrt_rem_large(words: &[Word]) -> (Repr, Repr) {
        // first shift the words so that there are even words and
        // the top word is normalized. Note: shift <= 2 * WORD_BITS - 2
        let shift = WORD_BITS_USIZE * (words.len() & 1)
            + (words.last().unwrap().leading_zeros() & !1) as usize;
        let n = (words.len() + 1) / 2;
        let mut buffer = shift_ops::repr::shl_large_ref(words, shift).into_buffer();
        let mut out = Buffer::allocate(n);
        out.push_zeros(n);

        let mut allocation =
            MemoryAllocation::new(root::memory_requirement_sqrt_rem(n));
        let r_top = root::sqrt_rem(&mut out, &mut buffer, &mut allocation.memory());

        // afterwards, s = out[..], r = buffer[..n] + r_top << n*WORD_BITS
        // then recover the result if shift != 0
        if shift != 0 {
            // to get the final result, let s0 = s mod 2^(shift/2), then
            // 2^shift*n = (s-s0)^2 + 2s*s0 - s0^2 + r, so final r = (r + 2s*s0 - s0^2) / 2^shift
            let s0 = out[0] & ((1 << (shift / 2)) - 1);
            let c1 = mul::add_mul_word_in_place(&mut buffer[..n], 2 * s0, &out);
            let c2 = add::sub_dword_in_place(&mut buffer[..n], extend_word(s0) * extend_word(s0));
            buffer[n] = r_top as Word + c1 - c2 as Word;
    
            // s >>= shift/2, r >>= shift
            let _ = shift::shr_in_place(&mut out, shift as u32 / 2);
            if shift > WORD_BITS_USIZE {
                shift::shr_in_place_one_word(&mut buffer);
                buffer.truncate(n);
            } else {
                buffer.truncate(n + 1);
            }
            let _ = shift::shr_in_place(&mut buffer, shift as u32 % WORD_BITS);
        } else {
            buffer[n] = r_top as Word;
            buffer.truncate(n + 1);
        }

        (Repr::from_buffer(out), Repr::from_buffer(buffer))
    }
}
