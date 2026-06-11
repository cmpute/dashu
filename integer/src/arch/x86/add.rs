use crate::arch::word::Word;

/// Add a + b + carry.
///
/// Returns (result, overflow).
//
// `_addcarry_u32` was originally declared `unsafe fn`; Rust 1.81 made it
// safe. The `unsafe` block is retained so the MSRV (1.68) build still
// compiles; `unused_unsafe` is allowed so post-1.81 builds don't warn.
#[inline]
#[allow(unused_unsafe)]
pub fn add_with_carry(a: Word, b: Word, carry: Word) -> (Word, Word) {
    let mut sum = 0;
    // SAFETY: this intrinsic is actually safe; the `unsafe` block is
    // retained for MSRV 1.68 where the intrinsic was still `unsafe fn`.
    let carry = unsafe { core::arch::x86::_addcarry_u32((carry & 1) as u8, a, b, &mut sum) };
    (sum, Word::from(carry))
}

/// Subtract a - b - borrow.
///
/// Returns (result, overflow).
#[inline]
#[allow(unused_unsafe)]
pub fn sub_with_borrow(a: Word, b: Word, borrow: Word) -> (Word, Word) {
    let mut diff = 0;
    // SAFETY: see `add_with_carry`.
    let borrow = unsafe { core::arch::x86::_subborrow_u32((borrow & 1) as u8, a, b, &mut diff) };
    (diff, Word::from(borrow))
}
