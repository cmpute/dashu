use crate::{ibig::IBig, ubig::UBig};

impl UBig {
    #[inline]
    pub fn square(&self) -> UBig {
        UBig(self.repr().square())
    }
}

impl IBig {
    #[inline]
    pub fn square(&self) -> IBig {
        IBig(self.as_sign_repr().1.square())
    }
}

mod repr {
    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        math,
        memory::MemoryAllocation,
        primitive::{extend_word, shrink_dword, split_dword},
        repr::{Repr, TypedReprRef},
        sqr,
    };

    impl TypedReprRef<'_> {
        pub fn square(&self) -> Repr {
            match self {
                TypedReprRef::RefSmall(dword) => {
                    if let Some(word) = shrink_dword(*dword) {
                        Repr::from_dword(extend_word(word) * extend_word(word))
                    } else {
                        unimplemented!()
                    }
                }
                TypedReprRef::RefLarge(words) => square_large(words),
            }
        }
    }

    fn square_dword_spilled(dw: DoubleWord) -> Repr {
        let (lo, hi) = math::mul_add_carry_dword(dw, dw, 0);
        let mut buffer = Buffer::allocate(4);
        let (n0, n1) = split_dword(lo);
        buffer.push(n0);
        buffer.push(n1);
        let (n2, n3) = split_dword(hi);
        buffer.push(n2);
        buffer.push(n3);
        Repr::from_buffer(buffer)
    }

    fn square_large(words: &[Word]) -> Repr {
        debug_assert!(words.len() >= 2);

        let mut buffer = Buffer::allocate(words.len() * 2);
        buffer.push_zeros(words.len());

        let mut allocation = MemoryAllocation::new(sqr::memory_requirement_exact(words.len()));
        let mut memory = allocation.memory();
        sqr::square(&mut buffer, words, &mut memory);
        Repr::from_buffer(buffer)
    }
}
