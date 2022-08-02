use super::{FastDivideNormalized, FastDivideNormalized2};
use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    div,
    error::panic_divide_by_0,
    helper_macros::debug_assert_zero,
    math::shl_dword,
    memory::MemoryAllocation,
    primitive::{double_word, extend_word, shrink_dword},
    repr::TypedRepr,
    shift,
    ubig::UBig,
};
use alloc::boxed::Box;

pub(crate) struct ConstSingleDivisor {
    pub(crate) shift: u32,
    pub(crate) fast_div: FastDivideNormalized,
}

pub(crate) struct ConstDoubleDivisor {
    pub(crate) shift: u32,
    pub(crate) fast_div: FastDivideNormalized2,
}
pub(crate) struct ConstLargeDivisor {
    pub(crate) normalized_modulus: Box<[Word]>,
    pub(crate) shift: u32,
    pub(crate) fast_div_top: FastDivideNormalized2,
}

impl ConstSingleDivisor {
    #[inline]
    pub const fn new(n: Word) -> Self {
        debug_assert!(n != 0);
        let shift = n.leading_zeros();
        let fast_div = FastDivideNormalized::new(n << shift);
        Self { shift, fast_div }
    }

    #[inline]
    pub const fn divisor(&self) -> Word {
        self.fast_div.divisor >> self.shift
    }

    #[inline]
    pub const fn rem_word(&self, word: Word) -> Word {
        if self.shift == 0 {
            self.fast_div.div_rem_word(word).1
        } else {
            self.fast_div.div_rem(extend_word(word) << self.shift).1
        }
    }

    #[inline]
    pub const fn rem_dword(&self, dword: DoubleWord) -> Word {
        if self.shift == 0 {
            self.fast_div.div_rem(dword).1
        } else {
            let (n0, n1, n2) = shl_dword(dword, self.shift);
            let (_, r1) = self.fast_div.div_rem(double_word(n1, n2));
            self.fast_div.div_rem(double_word(n0, r1)).1
        }
    }

    pub fn rem_large(&self, words: &[Word]) -> Word {
        let mut rem = div::fast_rem_by_normalized_word(words, self.fast_div);
        if self.shift != 0 {
            rem = self.fast_div.div_rem(extend_word(rem) << self.shift).1
        }
        rem
    }
}

impl ConstDoubleDivisor {
    #[inline]
    pub const fn new(n: DoubleWord) -> Self {
        debug_assert!(n > Word::MAX as DoubleWord);
        let shift = n.leading_zeros();
        let fast_div = FastDivideNormalized2::new(n << shift);
        Self { shift, fast_div }
    }

    #[inline]
    pub const fn divisor(&self) -> DoubleWord {
        self.fast_div.divisor >> self.shift
    }

    #[inline]
    pub const fn rem_dword(&self, dword: DoubleWord) -> DoubleWord {
        if self.shift == 0 {
            self.fast_div.div_rem_dword(dword).1
        } else {
            let (n0, n1, n2) = shl_dword(dword, self.shift);
            self.fast_div.div_rem(n0, double_word(n1, n2)).1
        }
    }

    pub fn rem_large(&self, words: &[Word]) -> DoubleWord {
        let mut rem = div::fast_rem_by_normalized_dword(words, self.fast_div);
        if self.shift != 0 {
            let (r0, r1, r2) = shl_dword(rem, self.shift);
            rem = self.fast_div.div_rem(r0, double_word(r1, r2)).1
        }
        rem
    }
}

impl ConstLargeDivisor {
    pub fn new(mut n: Buffer) -> Self {
        let (shift, fast_div_top) = crate::div::normalize(&mut n);
        Self {
            normalized_modulus: n.into_boxed_slice(),
            shift,
            fast_div_top,
        }
    }

    pub fn divisor(&self) -> Buffer {
        let mut buffer = Buffer::from(self.normalized_modulus.as_ref());
        debug_assert_zero!(shift::shr_in_place(&mut buffer, self.shift));
        buffer
    }

    pub fn rem_large(&self, x: TypedRepr) -> Buffer {
        let modulus = &self.normalized_modulus;
        match x {
            TypedRepr::Small(dword) => {
                let (lo, mid, hi) = shl_dword(dword, self.shift);
                let mut buffer = Buffer::allocate_exact(modulus.len());
                buffer.push(lo);
                buffer.push(mid);
                buffer.push(hi);

                // because ModuloLarge is used only for integer with more than two words,
                // word << ring.shift() must be smaller than the normalized modulus
                buffer
            }
            TypedRepr::Large(mut words) => {
                // normalize
                let carry = shift::shl_in_place(&mut words, self.shift);
                if carry != 0 {
                    words.push_resizing(carry);
                }

                // reduce
                if words.len() >= modulus.len() {
                    let mut allocation = MemoryAllocation::new(div::memory_requirement_exact(
                        words.len(),
                        modulus.len(),
                    ));
                    let _overflow = div::div_rem_in_place(
                        &mut words,
                        modulus,
                        self.fast_div_top,
                        &mut allocation.memory(),
                    );
                    words.truncate(modulus.len());
                }
                words.ensure_capacity_exact(modulus.len());
                words
            }
        }
    }
}

pub(crate) enum ConstDivisorRepr {
    Single(ConstSingleDivisor),
    Double(ConstDoubleDivisor),
    Large(ConstLargeDivisor),
}

pub struct ConstDivisor(pub(crate) ConstDivisorRepr);

impl ConstDivisor {
    pub fn new(n: UBig) -> ConstDivisor {
        Self(match n.into_repr() {
            TypedRepr::Small(0) => panic_divide_by_0(),
            TypedRepr::Small(dword) => {
                if let Some(word) = shrink_dword(dword) {
                    ConstDivisorRepr::Single(ConstSingleDivisor::new(word))
                } else {
                    ConstDivisorRepr::Double(ConstDoubleDivisor::new(dword))
                }
            }
            TypedRepr::Large(words) => ConstDivisorRepr::Large(ConstLargeDivisor::new(words)),
        })
    }

    #[inline]
    pub const fn from_word(word: Word) -> Self {
        if word == 0 {
            panic_divide_by_0()
        }
        Self(ConstDivisorRepr::Single(ConstSingleDivisor::new(word)))
    }

    #[inline]
    pub const fn from_dword(dword: DoubleWord) -> Self {
        if dword == 0 {
            panic_divide_by_0()
        }

        Self(if let Some(word) = shrink_dword(dword) {
            ConstDivisorRepr::Single(ConstSingleDivisor::new(word))
        } else {
            ConstDivisorRepr::Double(ConstDoubleDivisor::new(dword))
        })
    }
}
