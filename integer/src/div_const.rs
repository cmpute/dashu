//! Public interface for creating a constant divisor.

use core::{
    fmt::{Display, Formatter},
    mem,
    ops::{Div, DivAssign, Rem, RemAssign},
};
use dashu_base::{DivRem, DivRemAssign};
use num_modular::{PreMulInv2by1, PreMulInv3by2};

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    div,
    error::panic_divide_by_0,
    helper_macros::debug_assert_zero,
    math::{shl_dword, FastDivideNormalized2},
    memory::MemoryAllocation,
    primitive::{double_word, extend_word, shrink_dword},
    repr::Repr,
    repr::TypedRepr,
    shift,
    ubig::UBig,
    IBig,
};
use alloc::boxed::Box;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConstSingleDivisor(pub(crate) PreMulInv2by1<Word>);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConstDoubleDivisor(pub(crate) PreMulInv3by2<Word, DoubleWord>);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConstLargeDivisor {
    pub(crate) normalized_divisor: Box<[Word]>,
    pub(crate) shift: u32,
    pub(crate) fast_div_top: FastDivideNormalized2,
}

impl ConstSingleDivisor {
    /// Create a single word const divisor
    #[inline]
    pub const fn new(n: Word) -> Self {
        debug_assert!(n != 0);
        Self(PreMulInv2by1::<Word>::new(n))
    }

    /// Get the original (unnormalized) divisor
    #[inline]
    pub const fn divisor(&self) -> Word {
        self.0.divisor() >> self.0.shift()
    }

    #[inline]
    pub const fn normalized_divisor(&self) -> Word {
        self.0.divisor()
    }
    pub const fn shift(&self) -> u32 {
        self.0.shift()
    }

    /// Calculate (word << self.shift) % self
    #[inline]
    pub const fn rem_word(&self, word: Word) -> Word {
        if self.0.shift() == 0 {
            self.0.divider().div_rem_1by1(word).1
        } else {
            self.0
                .divider()
                .div_rem_2by1(extend_word(word) << self.0.shift())
                .1
        }
    }

    /// Calculate (dword << self.shift) % self
    #[inline]
    pub const fn rem_dword(&self, dword: DoubleWord) -> Word {
        if self.0.shift() == 0 {
            self.0.divider().div_rem_2by1(dword).1
        } else {
            let (n0, n1, n2) = shl_dword(dword, self.0.shift());
            let (_, r1) = self.0.divider().div_rem_2by1(double_word(n1, n2));
            self.0.divider().div_rem_2by1(double_word(n0, r1)).1
        }
    }

    /// Calculate (words << self.shift) % self
    pub fn rem_large(&self, words: &[Word]) -> Word {
        let mut rem = div::fast_rem_by_normalized_word(words, *self.0.divider());
        if self.0.shift() != 0 {
            rem = self
                .0
                .divider()
                .div_rem_2by1(extend_word(rem) << self.0.shift())
                .1
        }
        rem
    }
}

impl ConstDoubleDivisor {
    /// Create a double word const divisor
    #[inline]
    pub const fn new(n: DoubleWord) -> Self {
        debug_assert!(n > Word::MAX as DoubleWord);
        Self(PreMulInv3by2::<Word, DoubleWord>::new(n))
    }

    /// Get the original (unnormalized) divisor
    #[inline]
    pub const fn divisor(&self) -> DoubleWord {
        self.0.divisor() >> self.0.shift()
    }

    #[inline]
    pub const fn normalized_divisor(&self) -> DoubleWord {
        self.0.divisor()
    }
    pub const fn shift(&self) -> u32 {
        self.0.shift()
    }

    /// Calculate (dword << self.shift) % self
    #[inline]
    pub const fn rem_dword(&self, dword: DoubleWord) -> DoubleWord {
        if self.0.shift() == 0 {
            self.0.divider().div_rem_2by2(dword).1
        } else {
            let (n0, n1, n2) = shl_dword(dword, self.0.shift());
            self.0.divider().div_rem_3by2(n0, double_word(n1, n2)).1
        }
    }

    /// Calculate (words << self.shift) % self
    pub fn rem_large(&self, words: &[Word]) -> DoubleWord {
        let mut rem = div::fast_rem_by_normalized_dword(words, *self.0.divider());
        if self.0.shift() != 0 {
            let (r0, r1, r2) = shl_dword(rem, self.0.shift());
            rem = self.0.divider().div_rem_3by2(r0, double_word(r1, r2)).1
        }
        rem
    }
}

impl ConstLargeDivisor {
    /// Create a const divisor with multiple words
    pub fn new(mut n: Buffer) -> Self {
        let (shift, fast_div_top) = crate::div::normalize(&mut n);
        Self {
            normalized_divisor: n.into_boxed_slice(),
            shift,
            fast_div_top,
        }
    }

    /// Get the original (unnormalized) divisor
    pub fn divisor(&self) -> Buffer {
        let mut buffer = Buffer::from(self.normalized_divisor.as_ref());
        debug_assert_zero!(shift::shr_in_place(&mut buffer, self.shift));
        buffer
    }

    /// Calculate (words << self.shift) % self
    #[inline]
    pub fn rem_large(&self, mut words: Buffer) -> Buffer {
        // shift
        let carry = shift::shl_in_place(&mut words, self.shift);
        words.push_resizing(carry);

        // reduce
        let modulus = &self.normalized_divisor;
        if words.len() >= modulus.len() {
            let mut allocation =
                MemoryAllocation::new(div::memory_requirement_exact(words.len(), modulus.len()));
            let _overflow = div::div_rem_in_place(
                &mut words,
                modulus,
                self.fast_div_top,
                &mut allocation.memory(),
            );
            words.truncate(modulus.len());
        }
        words
    }

    /// Calculate (x << self.shift) % self
    #[inline]
    pub fn rem_repr(&self, x: TypedRepr) -> Buffer {
        match x {
            TypedRepr::Small(dword) => {
                let (lo, mid, hi) = shl_dword(dword, self.shift);
                let mut buffer = Buffer::allocate_exact(self.normalized_divisor.len());
                buffer.push(lo);
                buffer.push(mid);
                buffer.push(hi);

                // because ConstLargeDivisor is used only for integer with more than two words,
                // word << ring.shift() must be smaller than the normalized modulus
                buffer
            }
            TypedRepr::Large(words) => self.rem_large(words),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConstDivisorRepr {
    Single(ConstSingleDivisor),
    Double(ConstDoubleDivisor),
    Large(ConstLargeDivisor),
}

// TODO(v0.4): add docs for ConstDivisor (and mention it in the top level doc)
#[derive(Debug, PartialEq, Eq)]
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

    #[inline]
    pub fn value(&self) -> UBig {
        UBig(match &self.0 {
            ConstDivisorRepr::Single(d) => Repr::from_word(d.divisor()),
            ConstDivisorRepr::Double(d) => Repr::from_dword(d.divisor()),
            ConstDivisorRepr::Large(d) => Repr::from_buffer(d.divisor()),
        })
    }
}

impl Display for ConstDivisor {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.value(), f)
    }
}

impl<'r> Div<&'r ConstDivisor> for UBig {
    type Output = UBig;

    #[inline]
    fn div(self, rhs: &ConstDivisor) -> UBig {
        UBig(self.into_repr() / &rhs.0)
    }
}
impl<'l, 'r> Div<&'r ConstDivisor> for &'l UBig {
    type Output = UBig;

    #[inline]
    fn div(self, rhs: &ConstDivisor) -> UBig {
        UBig(self.clone().into_repr() / &rhs.0)
    }
}
impl<'r> DivAssign<&'r ConstDivisor> for UBig {
    #[inline]
    fn div_assign(&mut self, rhs: &'r ConstDivisor) {
        *self = mem::take(self) / rhs;
    }
}

impl<'r> Rem<&'r ConstDivisor> for UBig {
    type Output = UBig;

    #[inline]
    fn rem(self, rhs: &ConstDivisor) -> UBig {
        UBig(self.into_repr() % &rhs.0)
    }
}
impl<'l, 'r> Rem<&'r ConstDivisor> for &'l UBig {
    type Output = UBig;

    #[inline]
    fn rem(self, rhs: &ConstDivisor) -> UBig {
        UBig(self.repr() % &rhs.0)
    }
}
impl<'r> RemAssign<&'r ConstDivisor> for UBig {
    #[inline]
    fn rem_assign(&mut self, rhs: &'r ConstDivisor) {
        *self = mem::take(self) % rhs;
    }
}

impl<'r> DivRem<&'r ConstDivisor> for UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem(self, rhs: &ConstDivisor) -> (UBig, UBig) {
        let (q, r) = self.into_repr().div_rem(&rhs.0);
        (UBig(q), UBig(r))
    }
}
impl<'l, 'r> DivRem<&'r ConstDivisor> for &'l UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem(self, rhs: &ConstDivisor) -> (UBig, UBig) {
        let (q, r) = self.clone().into_repr().div_rem(&rhs.0);
        (UBig(q), UBig(r))
    }
}
impl<'r> DivRemAssign<&'r ConstDivisor> for UBig {
    type OutputRem = UBig;
    #[inline]
    fn div_rem_assign(&mut self, rhs: &ConstDivisor) -> UBig {
        let (q, r) = mem::take(self).div_rem(rhs);
        *self = q;
        r
    }
}

impl<'r> Div<&'r ConstDivisor> for IBig {
    type Output = IBig;

    #[inline]
    fn div(self, rhs: &ConstDivisor) -> IBig {
        let (sign, repr) = self.into_sign_repr();
        IBig((repr / &rhs.0).with_sign(sign))
    }
}
impl<'l, 'r> Div<&'r ConstDivisor> for &'l IBig {
    type Output = IBig;

    #[inline]
    fn div(self, rhs: &ConstDivisor) -> IBig {
        let (sign, repr) = self.clone().into_sign_repr();
        IBig((repr / &rhs.0).with_sign(sign))
    }
}
impl<'r> DivAssign<&'r ConstDivisor> for IBig {
    #[inline]
    fn div_assign(&mut self, rhs: &'r ConstDivisor) {
        *self = mem::take(self) / rhs;
    }
}

impl<'r> Rem<&'r ConstDivisor> for IBig {
    type Output = IBig;

    #[inline]
    fn rem(self, rhs: &ConstDivisor) -> IBig {
        let (sign, repr) = self.into_sign_repr();
        IBig((repr % &rhs.0).with_sign(sign))
    }
}
impl<'l, 'r> Rem<&'r ConstDivisor> for &'l IBig {
    type Output = IBig;

    #[inline]
    fn rem(self, rhs: &ConstDivisor) -> IBig {
        let (sign, repr) = self.as_sign_repr();
        IBig((repr % &rhs.0).with_sign(sign))
    }
}
impl<'r> RemAssign<&'r ConstDivisor> for IBig {
    #[inline]
    fn rem_assign(&mut self, rhs: &'r ConstDivisor) {
        *self = mem::take(self) % rhs;
    }
}

impl<'r> DivRem<&'r ConstDivisor> for IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem(self, rhs: &ConstDivisor) -> (IBig, IBig) {
        let (sign, repr) = self.into_sign_repr();
        let (q, r) = repr.div_rem(&rhs.0);
        (IBig(q.with_sign(sign)), IBig(r.with_sign(sign)))
    }
}
impl<'l, 'r> DivRem<&'r ConstDivisor> for &'l IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem(self, rhs: &ConstDivisor) -> (IBig, IBig) {
        let (sign, repr) = self.clone().into_sign_repr();
        let (q, r) = repr.div_rem(&rhs.0);
        (IBig(q.with_sign(sign)), IBig(r.with_sign(sign)))
    }
}
impl<'r> DivRemAssign<&'r ConstDivisor> for IBig {
    type OutputRem = IBig;
    #[inline]
    fn div_rem_assign(&mut self, rhs: &ConstDivisor) -> IBig {
        let (q, r) = mem::take(self).div_rem(rhs);
        *self = q;
        r
    }
}

mod repr {
    use super::*;
    use crate::repr::{
        Repr,
        TypedRepr::{self, *},
        TypedReprRef::{self, *},
    };

    impl<'r> Div<&'r ConstDivisorRepr> for TypedRepr {
        type Output = Repr;
        fn div(self, rhs: &ConstDivisorRepr) -> Repr {
            match (self, rhs) {
                (Small(dword), ConstDivisorRepr::Single(div)) => {
                    Repr::from_dword(div_rem_small_single(dword, div).0)
                }
                (Small(dword), ConstDivisorRepr::Double(div)) => {
                    Repr::from_word(div_rem_small_double(dword, div).0)
                }
                (Small(_), ConstDivisorRepr::Large(_)) => {
                    // lhs must be less than rhs
                    Repr::zero()
                }
                (Large(mut buffer), ConstDivisorRepr::Single(div)) => {
                    let _rem = div::fast_div_by_word_in_place(
                        &mut buffer,
                        div.0.shift(),
                        *div.0.divider(),
                    );
                    Repr::from_buffer(buffer)
                }
                (Large(mut buffer), ConstDivisorRepr::Double(div)) => {
                    let _rem = div::fast_div_by_dword_in_place(
                        &mut buffer,
                        div.0.shift(),
                        *div.0.divider(),
                    );
                    Repr::from_buffer(buffer)
                }
                (Large(mut buffer), ConstDivisorRepr::Large(div)) => {
                    let div_len = div.normalized_divisor.len();
                    if buffer.len() < div_len {
                        Repr::zero()
                    } else {
                        let mut allocation = MemoryAllocation::new(div::memory_requirement_exact(
                            buffer.len(),
                            div_len,
                        ));
                        let q_top = div::div_rem_unshifted_in_place(
                            &mut buffer,
                            &div.normalized_divisor,
                            div.shift,
                            div.fast_div_top,
                            &mut allocation.memory(),
                        );
                        buffer.erase_front(div_len);
                        buffer.push_resizing(q_top);
                        Repr::from_buffer(buffer)
                    }
                }
            }
        }
    }

    impl<'r> Rem<&'r ConstDivisorRepr> for TypedRepr {
        type Output = Repr;

        fn rem(self, rhs: &ConstDivisorRepr) -> Repr {
            match (self, rhs) {
                (Small(dword), ConstDivisorRepr::Single(div)) => {
                    Repr::from_word(div.rem_dword(dword) >> div.0.shift())
                }
                (Small(dword), ConstDivisorRepr::Double(div)) => {
                    Repr::from_dword(div.rem_dword(dword) >> div.0.shift())
                }
                (Small(dword), ConstDivisorRepr::Large(_)) => {
                    // lhs must be less than rhs
                    Repr::from_dword(dword)
                }
                (Large(buffer), ConstDivisorRepr::Single(div)) => {
                    Repr::from_word(div.rem_large(&buffer) >> div.0.shift())
                }
                (Large(buffer), ConstDivisorRepr::Double(div)) => {
                    Repr::from_dword(div.rem_large(&buffer) >> div.0.shift())
                }
                (Large(buffer), ConstDivisorRepr::Large(div)) => rem_large_large(buffer, div),
            }
        }
    }

    impl<'l, 'r> Rem<&'r ConstDivisorRepr> for TypedReprRef<'l> {
        type Output = Repr;

        fn rem(self, rhs: &ConstDivisorRepr) -> Repr {
            match (self, rhs) {
                (RefSmall(dword), ConstDivisorRepr::Single(div)) => {
                    Repr::from_word(div.rem_dword(dword) >> div.0.shift())
                }
                (RefSmall(dword), ConstDivisorRepr::Double(div)) => {
                    Repr::from_dword(div.rem_dword(dword) >> div.0.shift())
                }
                (RefSmall(dword), ConstDivisorRepr::Large(_)) => {
                    // lhs must be less than rhs
                    Repr::from_dword(dword)
                }
                (RefLarge(words), ConstDivisorRepr::Single(div)) => {
                    Repr::from_word(div.rem_large(words) >> div.0.shift())
                }
                (RefLarge(words), ConstDivisorRepr::Double(div)) => {
                    Repr::from_dword(div.rem_large(words) >> div.0.shift())
                }
                (RefLarge(words), ConstDivisorRepr::Large(div)) => {
                    rem_large_large(words.into(), div)
                }
            }
        }
    }

    impl<'r> DivRem<&'r ConstDivisorRepr> for TypedRepr {
        type OutputDiv = Repr;
        type OutputRem = Repr;

        fn div_rem(self, rhs: &ConstDivisorRepr) -> (Repr, Repr) {
            match (self, rhs) {
                (Small(dword), ConstDivisorRepr::Single(div)) => {
                    let (q, r) = div_rem_small_single(dword, div);
                    (Repr::from_dword(q), Repr::from_word(r))
                }
                (Small(dword), ConstDivisorRepr::Double(div)) => {
                    let (q, r) = div_rem_small_double(dword, div);
                    (Repr::from_word(q), Repr::from_dword(r))
                }
                (Small(dword), ConstDivisorRepr::Large(_)) => {
                    // lhs must be less than rhs
                    (Repr::zero(), Repr::from_dword(dword))
                }
                (Large(mut buffer), ConstDivisorRepr::Single(div)) => {
                    let r = div::fast_div_by_word_in_place(
                        &mut buffer,
                        div.0.shift(),
                        *div.0.divider(),
                    );
                    (Repr::from_buffer(buffer), Repr::from_word(r))
                }
                (Large(mut buffer), ConstDivisorRepr::Double(div)) => {
                    let r = div::fast_div_by_dword_in_place(
                        &mut buffer,
                        div.0.shift(),
                        *div.0.divider(),
                    );
                    (Repr::from_buffer(buffer), Repr::from_dword(r))
                }
                (Large(mut buffer), ConstDivisorRepr::Large(div)) => {
                    let div_len = div.normalized_divisor.len();
                    if buffer.len() < div_len {
                        (Repr::zero(), Repr::from_buffer(buffer))
                    } else {
                        let mut allocation = MemoryAllocation::new(div::memory_requirement_exact(
                            buffer.len(),
                            div_len,
                        ));
                        let q_top = div::div_rem_unshifted_in_place(
                            &mut buffer,
                            &div.normalized_divisor,
                            div.shift,
                            div.fast_div_top,
                            &mut allocation.memory(),
                        );

                        let mut q = Buffer::from(&buffer[div_len..]);
                        q.push_resizing(q_top);
                        buffer.truncate(div_len);
                        debug_assert_zero!(shift::shr_in_place(&mut buffer, div.shift));
                        (Repr::from_buffer(q), Repr::from_buffer(buffer))
                    }
                }
            }
        }
    }

    fn div_rem_small_single(lhs: DoubleWord, rhs: &ConstSingleDivisor) -> (DoubleWord, Word) {
        let (lo, mid, hi) = shl_dword(lhs, rhs.0.shift());
        let (q1, r1) = rhs.0.divider().div_rem_2by1(double_word(mid, hi));
        let (q0, r0) = rhs.0.divider().div_rem_2by1(double_word(lo, r1));
        (double_word(q0, q1), r0 >> rhs.0.shift())
    }

    fn div_rem_small_double(lhs: DoubleWord, rhs: &ConstDoubleDivisor) -> (Word, DoubleWord) {
        let (lo, mid, hi) = shl_dword(lhs, rhs.0.shift());
        let (q, r) = rhs.0.divider().div_rem_3by2(lo, double_word(mid, hi));
        (q, r >> rhs.0.shift())
    }

    fn rem_large_large(mut lhs: Buffer, rhs: &ConstLargeDivisor) -> Repr {
        let modulus = &rhs.normalized_divisor;

        // only reduce if lhs can be larger than rhs
        if lhs.len() >= modulus.len() {
            let mut allocation =
                MemoryAllocation::new(div::memory_requirement_exact(lhs.len(), modulus.len()));
            let _qtop = div::div_rem_unshifted_in_place(
                &mut lhs,
                modulus,
                rhs.shift,
                rhs.fast_div_top,
                &mut allocation.memory(),
            );

            lhs.truncate(modulus.len());
            debug_assert_zero!(shift::shr_in_place(&mut lhs, rhs.shift));
        }
        Repr::from_buffer(lhs)
    }
}
