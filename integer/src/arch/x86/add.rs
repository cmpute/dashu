use crate::arch::word::Word;

/// Add a + b + carry.
///
/// Returns (result, overflow).
//
// `_addcarry_u32` was originally declared `unsafe fn`; Rust 1.81 made it
// safe. The `unsafe` block is retained so the MSRV (1.73) build still
// compiles; `unused_unsafe` is allowed so post-1.81 builds don't warn.
#[inline]
#[allow(unused_unsafe)]
pub fn add_with_carry(a: Word, b: Word, carry: bool) -> (Word, bool) {
    let mut sum = 0;
    // SAFETY: this intrinsic is actually safe; the `unsafe` block is
    // retained for MSRV 1.73 where the intrinsic was still `unsafe fn`.
    let carry = unsafe { core::arch::x86::_addcarry_u32(carry.into(), a, b, &mut sum) };
    (sum, carry != 0)
}

/// Subtract a - b - borrow.
///
/// Returns (result, overflow).
#[inline]
#[allow(unused_unsafe)]
pub fn sub_with_borrow(a: Word, b: Word, borrow: bool) -> (Word, bool) {
    let mut diff = 0;
    // SAFETY: see `add_with_carry`.
    let borrow = unsafe { core::arch::x86::_subborrow_u32(borrow.into(), a, b, &mut diff) };
    (diff, borrow != 0)
}
